use crate::middleware::auth::{AuthMiddleware, AuthenticatedServer};
use crate::rpc::{INTERNAL_ERROR, INVALID_REQUEST, PARSE_ERROR, RpcResponse, RpcServer};
use crate::server::metrics::{Metrics, RequestTracer};
use crate::transport::framing::{DEFAULT_MAX_FRAME_SIZE, FrameCodec};
use crate::transport::shutdown::ShutdownCoordinator;
use crate::util::batch::{BatchRequest, BatchResponse};
use anyhow::{Result, anyhow};
use std::sync::Arc;
use std::time::Duration;
use tokio::net::{TcpListener, TcpStream};
use tokio::sync::Semaphore;
use tracing::{error, info, warn};

const DEFAULT_MAX_BATCH_SIZE: usize = 100;
const DEFAULT_MAX_CONNECTIONS: usize = 1024;
const DEFAULT_REQUEST_TIMEOUT: Duration = Duration::from_secs(30);

pub struct TcpServerConfig {
    pub addr: String,
    pub server: Arc<RpcServer>,
    pub auth: Option<Arc<AuthMiddleware>>,
    pub metrics: Arc<Metrics>,
    pub max_frame_size: usize,
    pub max_batch_size: usize,
    pub max_connections: usize,
    pub request_timeout: Duration,
}

impl TcpServerConfig {
    pub fn new(addr: impl Into<String>, server: Arc<RpcServer>) -> Self {
        Self {
            addr: addr.into(),
            server,
            auth: None,
            metrics: Arc::new(Metrics::new()),
            max_frame_size: DEFAULT_MAX_FRAME_SIZE,
            max_batch_size: DEFAULT_MAX_BATCH_SIZE,
            max_connections: DEFAULT_MAX_CONNECTIONS,
            request_timeout: DEFAULT_REQUEST_TIMEOUT,
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

    pub fn with_max_frame_size(mut self, bytes: usize) -> Self {
        self.max_frame_size = bytes.max(1);
        self
    }

    pub fn with_max_batch_size(mut self, requests: usize) -> Self {
        self.max_batch_size = requests.max(1);
        self
    }

    pub fn with_max_connections(mut self, connections: usize) -> Self {
        self.max_connections = connections.max(1);
        self
    }

    pub fn with_request_timeout(mut self, timeout: Duration) -> Self {
        self.request_timeout = timeout;
        self
    }
}

/// Run a length-prefixed TCP server with bounded frames, batches, connections,
/// and request execution time.
pub async fn run_with_framing(config: TcpServerConfig) -> Result<()> {
    let listener = TcpListener::bind(&config.addr).await?;
    info!("DiceRPC TCP server (framed) listening on {}", config.addr);

    let shutdown = Arc::new(ShutdownCoordinator::new());
    let shutdown_clone = shutdown.clone();
    tokio::spawn(async move {
        shutdown_clone.wait_for_signal().await;
    });

    let server = config.server;
    let auth = config.auth;
    let metrics = config.metrics;
    let max_frame_size = config.max_frame_size;
    let max_batch_size = config.max_batch_size;
    let request_timeout = config.request_timeout;
    let connection_limit = Arc::new(Semaphore::new(config.max_connections));
    let mut shutdown_rx = shutdown.subscribe();

    loop {
        tokio::select! {
            accept_result = listener.accept() => {
                match accept_result {
                    Ok((socket, _)) => {
                        let permit = match connection_limit.clone().try_acquire_owned() {
                            Ok(permit) => permit,
                            Err(_) => {
                                warn!("Connection limit reached; rejecting TCP client");
                                continue;
                            }
                        };
                        let server = server.clone();
                        let auth = auth.clone();
                        let metrics = metrics.clone();

                        tokio::spawn(async move {
                            let _permit = permit;
                            if let Err(error) = handle_framed_connection(
                                server,
                                socket,
                                auth,
                                metrics,
                                max_frame_size,
                                max_batch_size,
                                request_timeout,
                            )
                            .await
                            {
                                error!("Connection error: {error:?}");
                            }
                        });
                    }
                    Err(error) => error!("Failed to accept connection: {error:?}"),
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
    max_frame_size: usize,
    max_batch_size: usize,
    request_timeout: Duration,
) -> Result<()> {
    loop {
        let frame = match tokio::time::timeout(
            request_timeout,
            FrameCodec::read_frame_with_limit(&mut stream, max_frame_size),
        )
        .await
        {
            Ok(Ok(frame)) => frame,
            Err(_) => return Err(anyhow!("Timed out while reading TCP frame")),
            Ok(Err(error)) if error.to_string().contains("unexpected end of file") => break,
            Ok(Err(error)) => return Err(error),
        };

        let raw = std::str::from_utf8(&frame)?;
        let batch = match BatchRequest::parse(raw) {
            Ok(batch) => batch,
            Err(error) => {
                let response = RpcResponse::with_error(
                    serde_json::Value::Null,
                    PARSE_ERROR,
                    format!("Parse error: {error}"),
                );
                write_response(&mut stream, &response, max_frame_size).await?;
                continue;
            }
        };

        if batch.len() > max_batch_size {
            let response = RpcResponse::with_error(
                serde_json::Value::Null,
                INVALID_REQUEST,
                format!("Batch exceeds maximum of {max_batch_size} requests"),
            );
            write_response(&mut stream, &response, max_frame_size).await?;
            continue;
        }

        let method = match &batch {
            BatchRequest::Single(request) => request.method.clone(),
            BatchRequest::Batch(requests) => format!("batch({})", requests.len()),
        };
        let tracer = RequestTracer::new(&method, metrics.clone());

        let response_future = async {
            if let Some(auth) = auth.as_deref() {
                handle_authenticated_batch(server.clone(), batch, auth).await
            } else {
                server.handle_batch(batch).await
            }
        };

        let response = match tokio::time::timeout(request_timeout, response_future).await {
            Ok(response) => response,
            Err(_) => BatchResponse::Single(RpcResponse::with_error(
                serde_json::Value::Null,
                INTERNAL_ERROR,
                "Request timed out",
            )),
        };

        let has_error = match &response {
            BatchResponse::Single(response) => response.error.is_some(),
            BatchResponse::Batch(responses) => {
                responses.iter().any(|response| response.error.is_some())
            }
        };

        if has_error {
            tracer.error("Request returned error").await;
        } else {
            tracer.success().await;
        }

        write_response(&mut stream, &response, max_frame_size).await?;
    }

    Ok(())
}

async fn handle_authenticated_batch(
    server: Arc<RpcServer>,
    batch: BatchRequest,
    auth: &AuthMiddleware,
) -> BatchResponse {
    match batch {
        BatchRequest::Single(request) => {
            BatchResponse::Single(server.handle_authenticated_request(request, auth).await)
        }
        BatchRequest::Batch(requests) => {
            if requests.is_empty() {
                return BatchResponse::Single(RpcResponse::with_error(
                    serde_json::Value::Null,
                    INVALID_REQUEST,
                    "Invalid Request: empty batch",
                ));
            }

            let futures = requests.into_iter().map(|request| {
                let server = server.clone();
                async move { server.handle_authenticated_request(request, auth).await }
            });
            BatchResponse::Batch(futures::future::join_all(futures).await)
        }
    }
}

async fn write_response<T: serde::Serialize>(
    stream: &mut TcpStream,
    response: &T,
    max_frame_size: usize,
) -> Result<()> {
    let bytes = serde_json::to_vec(response)?;
    FrameCodec::write_frame_with_limit(stream, &bytes, max_frame_size).await
}

/// Run the legacy newline-delimited TCP server.
pub async fn run(addr: &str) -> Result<()> {
    crate::server::server::run(addr).await
}
