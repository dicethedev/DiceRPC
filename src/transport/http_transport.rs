use crate::middleware::auth::{AuthMiddleware, AuthenticatedServer};
use crate::rpc::{RpcResponse, RpcServer};
use crate::server::metrics::{Metrics, RequestTracer};
use crate::util::batch::{BatchRequest, BatchResponse};
use axum::{
    Json, Router,
    extract::State,
    http::StatusCode,
    response::{IntoResponse, Response},
    routing::post,
};
use serde_json::Value;
use std::sync::Arc;

/// Example usage:
/// ```rust
/// use dice_rpc::*;
///
/// #[tokio::main]
/// async fn main() -> anyhow::Result<()> {
///     let server = Arc::new(RpcServer::new());
///     
///     // Register handlers
///     register_default_handlers(&server).await;
///     
///     // Optional: Add authentication
///     let auth = Arc::new(AuthMiddleware::new(AuthStrategy::ApiKeyInParams));
///     auth.add_key("my-secret-key").await;
///     
///     // Start HTTP server
///     HttpTransport::new(server)
///         .with_auth(auth)
///         .serve("127.0.0.1:3000")
///         .await?;
///     
///     Ok(())
/// }
/// ```

/// HTTP transport layer for RPC server

#[allow(dead_code)]
pub struct HttpTransport {
    server: Arc<RpcServer>,
    auth: Option<Arc<AuthMiddleware>>,
    metrics: Option<Arc<Metrics>>,
}

#[allow(dead_code)]
impl HttpTransport {
    pub fn new(server: Arc<RpcServer>) -> Self {
        Self {
            server,
            auth: None,
            metrics: None,
        }
    }

    pub fn with_auth(mut self, auth: Arc<AuthMiddleware>) -> Self {
        self.auth = Some(auth);
        self
    }

    pub fn with_metrics(mut self, metrics: Arc<Metrics>) -> Self {
        self.metrics = Some(metrics);
        self
    }

    /// Create the axum router
    pub fn router(self) -> Router {
        let state = Arc::new(self);

        let mut router = Router::new()
            .route("/", post(rpc_handler))
            .route("/rpc", post(rpc_handler))
            .with_state(state.clone());

        // Add metrics endpoints if metrics are enabled
        if let Some(ref metrics) = state.metrics {
            router = router.merge(crate::transport::metrics_endpoint::metrics_router(
                metrics.clone(),
            ));
        }

        router
    }

    /// Start the HTTP server
    pub async fn serve(self, addr: &str) -> anyhow::Result<()> {
        let listener = tokio::net::TcpListener::bind(addr).await?;
        println!("HTTP RPC server listening on {}", addr);

        axum::serve(listener, self.router()).await?;
        Ok(())
    }
}

/// Main RPC handler for HTTP requests
async fn rpc_handler(
    State(transport): State<Arc<HttpTransport>>,
    Json(payload): Json<Value>,
) -> Response {
    // Parse as batch request (handles both single and batch)
    let batch_req = match serde_json::from_value::<BatchRequest>(payload) {
        Ok(req) => req,
        Err(e) => {
            let error_response =
                RpcResponse::with_error(Value::Null, -32700, format!("Parse error: {}", e));
            return (StatusCode::OK, Json(error_response)).into_response();
        }
    };

    //METRICS TRACKING
    let method = match &batch_req {
        BatchRequest::Single(req) => req.method.clone(),
        BatchRequest::Batch(reqs) => format!("batch({})", reqs.len()),
    };

    let tracer = if let Some(metrics) = &transport.metrics {
        Some(RequestTracer::new(&method, metrics.clone()))
    } else {
        None
    };

    // Handle with or without authentication
    let batch_resp = if let Some(auth) = &transport.auth {
        handle_authenticated_batch(&transport.server, batch_req, auth).await
    } else {
        transport.server.handle_batch(batch_req).await
    };

    // ← CHECK FOR ERRORS AND RECORD
    if let Some(tracer) = tracer {
        let has_error = match &batch_resp {
            BatchResponse::Single(resp) => resp.error.is_some(),
            BatchResponse::Batch(resps) => resps.iter().any(|r| r.error.is_some()),
        };

        if has_error {
            tracer.error("Request returned error").await;
        } else {
            tracer.success().await;
        }
    }

    (StatusCode::OK, Json(batch_resp)).into_response()
}

/// Handle batch request with authentication
async fn handle_authenticated_batch(
    server: &RpcServer,
    batch: BatchRequest,
    auth: &AuthMiddleware,
) -> BatchResponse {
    match batch {
        BatchRequest::Single(req) => {
            BatchResponse::Single(server.handle_authenticated_request(req, auth).await)
        }
        BatchRequest::Batch(requests) => {
            let futures: Vec<_> = requests
                .into_iter()
                .map(|req| server.handle_authenticated_request(req, auth))
                .collect();

            let responses = futures::future::join_all(futures).await;
            BatchResponse::Batch(responses)
        }
    }
}
