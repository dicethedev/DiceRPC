use tokio::net::{TcpListener, TcpStream};
use crate::rpc::{RpcServer, parse_rpc_request};
use crate::transport::framing::FrameCodec;
use crate::util::batch::{BatchRequest, BatchResponse};
use crate::middleware::auth::AuthMiddleware;
use crate::server::metrics::{Metrics, RequestTracer};
use crate::transport::shutdown::ShutdownCoordinator;use anyhow::Result;
use std::sync::Arc;
use tracing::{info, error};
use tokio::io::AsyncWriteExt;
use tokio::io::AsyncBufReadExt;
use tokio::io::BufReader;

pub struct TcpServerConfig {
    pub addr: String,
    pub server: Arc<RpcServer>,
    pub auth: Option<Arc<AuthMiddleware>>,
    pub metrics: Arc<Metrics>,
}

impl TcpServerConfig {
    pub fn new(addr: impl Into<String>, server: Arc<RpcServer>) -> Self {
        Self {
            addr: addr.into(),
            server,
            auth: None,
            metrics: Arc::new(Metrics::new()),
        }
    }

    pub fn with_auth(mut self, auth: Arc<AuthMiddleware>) -> Self {
        self.auth = Some(auth);
        self
    }

    pub fn with_metrics(mut self, metrics: Arc<Metrics>) -> Self {
        self.metrics = metrics;
        self
    }
}

/// Run TCP server with length-prefixed framing
pub async fn run_with_framing(config: TcpServerConfig) -> Result<()> {
    let listener = TcpListener::bind(&config.addr).await?;
    info!("DiceRPC TCP server (framed) listening on {}", config.addr);

    let shutdown = Arc::new(ShutdownCoordinator::new());
    let shutdown_clone = shutdown.clone();
    
    // Spawn signal handler
    tokio::spawn(async move {
        shutdown_clone.wait_for_signal().await;
    });

    let server = config.server;
    let auth = config.auth;
    let metrics = config.metrics;
    let mut shutdown_rx = shutdown.subscribe();

    loop {
        tokio::select! {
            accept_result = listener.accept() => {
                match accept_result {
                    Ok((socket, _)) => {
                        let server = server.clone();
                        let auth = auth.clone();
                        let metrics = metrics.clone();
                        
                        tokio::spawn(async move {
                            if let Err(e) = handle_framed_connection(server, socket, auth, metrics).await {
                                error!("Connection error: {:?}", e);
                            }
                        });
                    }
                    Err(e) => {
                        error!("Failed to accept connection: {:?}", e);
                    }
                }
            }
            _ = shutdown_rx.recv() => {
                info!("Shutting down TCP server");
                break;
            }
        }
    }

    Ok(())
}

async fn handle_framed_connection(
    server: Arc<RpcServer>,
    mut stream: TcpStream,
    auth: Option<Arc<AuthMiddleware>>,
    metrics: Arc<Metrics>,
) -> Result<()> {
    loop {
        // Read framed message
        let frame = match FrameCodec::read_frame(&mut stream).await {
            Ok(f) => f,
            Err(e) => {
                if e.to_string().contains("unexpected end of file") {
                    // Client disconnected
                    break;
                }
                return Err(e);
            }
        };

        // Parse as JSON string
        let raw = String::from_utf8(frame)?;
        
        // Parse as batch request
        let batch_req = match BatchRequest::parse(&raw) {
            Ok(req) => req,
            Err(e) => {
                let error_resp = crate::rpc::RpcResponse::with_error(
                    serde_json::Value::Null,
                    -32700,
                    format!("Parse error: {}", e),
                );
                let resp_bytes = serde_json::to_vec(&error_resp)?;
                FrameCodec::write_frame(&mut stream, &resp_bytes).await?;
                continue;
            }
        };

        // Track request
        let method = match &batch_req {
            BatchRequest::Single(req) => req.method.clone(),
            BatchRequest::Batch(reqs) => format!("batch({})", reqs.len()),
        };
        
        let tracer = RequestTracer::new(&method, metrics.clone());

        // Handle request
        let batch_resp = if let Some(ref auth_arc) = auth {
            // pass an Arc<RpcServer> and a reference to the middleware implementation
           handle_authenticated_batch(server.clone(), batch_req, &auth_arc).await
        } else {
            server_handle_batch(server.clone(), batch_req).await
        };

        // Check if response contains errors
        let has_error = match &batch_resp {
            BatchResponse::Single(resp) => resp.error.is_some(),
            BatchResponse::Batch(resps) => resps.iter().any(|r| r.error.is_some()),
        };

        if has_error {
            tracer.error("Request returned error").await;
        } else {
            tracer.success().await;
        }

        // Send response
        let resp_bytes = serde_json::to_vec(&batch_resp)?;
        FrameCodec::write_frame(&mut stream, &resp_bytes).await?;
    }

    Ok(())
}


async fn handle_authenticated_batch(
    server: Arc<RpcServer>,
    batch: BatchRequest,
    _auth: &AuthMiddleware,
) -> BatchResponse {
    match batch {
        BatchRequest::Single(req) => {
            // Use the existing handle_request method on RpcServer
            BatchResponse::Single(server.handle_request(req).await)
        }
        BatchRequest::Batch(requests) => {
            // Spawn futures that call handle_request on clones of the Arc<RpcServer>
            let futures: Vec<_> = requests
                .into_iter()
                .map(|req| {
                    let srv = server.clone();
                    async move { srv.handle_request(req).await }
                })
                .collect();

            let responses = futures::future::join_all(futures).await;
            BatchResponse::Batch(responses)
        }
    }
}

async fn server_handle_batch(server: Arc<RpcServer>, batch: BatchRequest) -> BatchResponse {
    match batch {
        BatchRequest::Single(req) => {
            // Delegate single request to RpcServer::handle_request
            BatchResponse::Single(server.handle_request(req).await)
        }
        BatchRequest::Batch(requests) => {
            // Spawn futures that call handle_request on clones of the Arc<RpcServer>
            let futures: Vec<_> = requests
                .into_iter()
                .map(|req| {
                    let srv = server.clone();
                    async move { srv.handle_request(req).await }
                })
                .collect();

            let responses = futures::future::join_all(futures).await;
            BatchResponse::Batch(responses)
        }
    }
}

/// Legacy newline-delimited server (for backwards compatibility)
pub async fn run(addr: &str) -> Result<()> {    
    let listener = TcpListener::bind(addr).await?;
    info!("DiceRPC TCP server (line-delimited) listening on {}", addr);

    let server = Arc::new(RpcServer::new());
    crate::rpc::register_default_handlers(&server).await;

    loop {
        let (socket, _) = listener.accept().await?;
        let server = server.clone();
        tokio::spawn(async move {
            if let Err(e) = handle_connection_legacy(server, socket).await {
                error!("Connection error: {:?}", e);
            }
        });
    }
}

async fn handle_connection_legacy(server: Arc<RpcServer>, stream: TcpStream) -> Result<()> {
    let (reader, mut writer) = stream.into_split();
    let mut br = BufReader::new(reader);
    let mut line = String::new();

    loop {
        line.clear();
        let n = br.read_line(&mut line).await?;
        if n == 0 {
            break;
        }

        let raw = line.trim_end();
        if raw.is_empty() {
            continue;
        }

        match parse_rpc_request(raw) {
            Ok(req) => {
                let resp = server.handle_request(req).await;
                let resp_text = serde_json::to_string(&resp)?;
                writer.write_all(resp_text.as_bytes()).await?;
                writer.write_all(b"\n").await?;
            }
            Err(e) => {
                let err_resp = crate::rpc::RpcResponse::with_error(
                    serde_json::Value::Null,
                    -32700,
                    format!("Parse error: {}", e),
                );
                let resp_text = serde_json::to_string(&err_resp)?;
                writer.write_all(resp_text.as_bytes()).await?;
                writer.write_all(b"\n").await?;
            }
        }
    }

    Ok(())
}