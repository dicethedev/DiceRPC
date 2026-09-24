use crate::middleware::auth::{AuthMiddleware, AuthStrategy, AuthenticatedServer};
use crate::rpc::{
    INTERNAL_ERROR, INVALID_REQUEST, PARSE_ERROR, RpcErrorObj, RpcResponse, RpcServer,
};
use crate::server::metrics::{Metrics, RequestTracer};
use crate::util::batch::{BatchRequest, BatchResponse};
use axum::{
    Json, Router,
    extract::{DefaultBodyLimit, State, rejection::JsonRejection},
    http::{HeaderMap, StatusCode},
    response::{IntoResponse, Response},
    routing::post,
};
use serde_json::Value;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::Semaphore;

pub const API_KEY_HEADER: &str = "x-api-key";
const DEFAULT_MAX_BODY_SIZE: usize = 1024 * 1024;
const DEFAULT_MAX_BATCH_SIZE: usize = 100;
const DEFAULT_MAX_CONCURRENCY: usize = 256;
const DEFAULT_REQUEST_TIMEOUT: Duration = Duration::from_secs(30);

/// HTTP transport for JSON-RPC requests.
pub struct HttpTransport {
    server: Arc<RpcServer>,
    auth: Option<Arc<AuthMiddleware>>,
    metrics: Option<Arc<Metrics>>,
    max_body_size: usize,
    max_batch_size: usize,
    request_timeout: Duration,
    concurrency_limit: Arc<Semaphore>,
}

impl HttpTransport {
    pub fn new(server: Arc<RpcServer>) -> Self {
        Self {
            server,
            auth: None,
            metrics: None,
            max_body_size: DEFAULT_MAX_BODY_SIZE,
            max_batch_size: DEFAULT_MAX_BATCH_SIZE,
            request_timeout: DEFAULT_REQUEST_TIMEOUT,
            concurrency_limit: Arc::new(Semaphore::new(DEFAULT_MAX_CONCURRENCY)),
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

    pub fn with_max_body_size(mut self, bytes: usize) -> Self {
        self.max_body_size = bytes.max(1);
        self
    }

    pub fn with_max_batch_size(mut self, requests: usize) -> Self {
        self.max_batch_size = requests.max(1);
        self
    }

    pub fn with_max_concurrency(mut self, requests: usize) -> Self {
        self.concurrency_limit = Arc::new(Semaphore::new(requests.max(1)));
        self
    }

    pub fn with_request_timeout(mut self, timeout: Duration) -> Self {
        self.request_timeout = timeout;
        self
    }

    pub fn router(self) -> Router {
        let max_body_size = self.max_body_size;
        let state = Arc::new(self);
        let mut router = Router::new()
            .route("/", post(rpc_handler))
            .route("/rpc", post(rpc_handler))
            .with_state(state.clone())
            .layer(DefaultBodyLimit::max(max_body_size));

        if let Some(metrics) = &state.metrics {
            router = router.merge(crate::transport::metrics_endpoint::metrics_router(
                metrics.clone(),
            ));
        }

        router
    }

    pub async fn serve(self, addr: &str) -> anyhow::Result<()> {
        let listener = tokio::net::TcpListener::bind(addr).await?;
        println!("HTTP RPC server listening on {addr}");
        axum::serve(listener, self.router()).await?;
        Ok(())
    }
}

async fn rpc_handler(
    State(transport): State<Arc<HttpTransport>>,
    headers: HeaderMap,
    payload: Result<Json<Value>, JsonRejection>,
) -> Response {
    let Json(payload) = match payload {
        Ok(payload) => payload,
        Err(error) => {
            return json_rpc_response(RpcResponse::with_error(
                Value::Null,
                PARSE_ERROR,
                format!("Parse error: {error}"),
            ));
        }
    };

    let batch = match serde_json::from_value::<BatchRequest>(payload) {
        Ok(batch) => batch,
        Err(error) => {
            return json_rpc_response(RpcResponse::with_error(
                Value::Null,
                INVALID_REQUEST,
                format!("Invalid Request: {error}"),
            ));
        }
    };

    if batch.len() > transport.max_batch_size {
        return json_rpc_response(RpcResponse::with_error(
            Value::Null,
            INVALID_REQUEST,
            format!(
                "Batch exceeds maximum of {} requests",
                transport.max_batch_size
            ),
        ));
    }

    let method = match &batch {
        BatchRequest::Single(request) => request.method.clone(),
        BatchRequest::Batch(requests) => format!("batch({})", requests.len()),
    };
    let tracer = transport
        .metrics
        .as_ref()
        .map(|metrics| RequestTracer::new(method, metrics.clone()));

    let permit = match tokio::time::timeout(
        transport.request_timeout,
        transport.concurrency_limit.clone().acquire_owned(),
    )
    .await
    {
        Ok(Ok(permit)) => permit,
        _ => {
            return json_rpc_response(RpcResponse::with_error(
                Value::Null,
                INTERNAL_ERROR,
                "Server is busy",
            ));
        }
    };

    let response_future = handle_http_batch(&transport, batch, &headers);
    let response = match tokio::time::timeout(transport.request_timeout, response_future).await {
        Ok(response) => response,
        Err(_) => BatchResponse::Single(RpcResponse::with_error(
            Value::Null,
            INTERNAL_ERROR,
            "Request timed out",
        )),
    };
    drop(permit);

    let has_error = match &response {
        BatchResponse::Single(response) => response.error.is_some(),
        BatchResponse::Batch(responses) => {
            responses.iter().any(|response| response.error.is_some())
        }
    };
    if let Some(tracer) = tracer {
        if has_error {
            tracer.error("Request returned error").await;
        } else {
            tracer.success().await;
        }
    }

    json_rpc_response(response)
}

async fn handle_http_batch(
    transport: &HttpTransport,
    batch: BatchRequest,
    headers: &HeaderMap,
) -> BatchResponse {
    let Some(auth) = transport.auth.as_deref() else {
        return transport.server.handle_batch(batch).await;
    };

    match auth.strategy() {
        AuthStrategy::None => transport.server.handle_batch(batch).await,
        AuthStrategy::ApiKeyInParams => {
            handle_parameter_authenticated_batch(&transport.server, batch, auth).await
        }
        AuthStrategy::ApiKeyInHeader => {
            let key = headers
                .get(API_KEY_HEADER)
                .and_then(|value| value.to_str().ok());
            match auth.validate_key(key).await {
                Ok(()) => transport.server.handle_batch(batch).await,
                Err(error) => authentication_error_batch(batch, error),
            }
        }
    }
}

async fn handle_parameter_authenticated_batch(
    server: &RpcServer,
    batch: BatchRequest,
    auth: &AuthMiddleware,
) -> BatchResponse {
    match batch {
        BatchRequest::Single(request) => {
            BatchResponse::Single(server.handle_authenticated_request(request, auth).await)
        }
        BatchRequest::Batch(requests) => {
            if requests.is_empty() {
                return invalid_empty_batch();
            }
            let futures = requests
                .into_iter()
                .map(|request| server.handle_authenticated_request(request, auth));
            BatchResponse::Batch(futures::future::join_all(futures).await)
        }
    }
}

fn authentication_error_batch(batch: BatchRequest, error: RpcErrorObj) -> BatchResponse {
    match batch {
        BatchRequest::Single(request) => {
            BatchResponse::Single(RpcResponse::with_error_obj(request.response_id(), error))
        }
        BatchRequest::Batch(requests) => {
            if requests.is_empty() {
                return invalid_empty_batch();
            }
            BatchResponse::Batch(
                requests
                    .into_iter()
                    .map(|request| {
                        RpcResponse::with_error_obj(request.response_id(), error.clone())
                    })
                    .collect(),
            )
        }
    }
}

fn invalid_empty_batch() -> BatchResponse {
    BatchResponse::Single(RpcResponse::with_error(
        Value::Null,
        INVALID_REQUEST,
        "Invalid Request: empty batch",
    ))
}

fn json_rpc_response<T: serde::Serialize>(value: T) -> Response {
    (StatusCode::OK, Json(value)).into_response()
}
