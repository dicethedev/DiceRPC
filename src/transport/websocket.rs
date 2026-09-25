use crate::middleware::auth::{AUTH_REQUIRED, AuthMiddleware, AuthStrategy, AuthenticatedServer};
use crate::rpc::{INTERNAL_ERROR, INVALID_REQUEST, PARSE_ERROR, RpcResponse, RpcServer};
use crate::server::metrics::{Metrics, RequestTracer};
use crate::transport::http_transport::API_KEY_HEADER;
use crate::util::batch::{BatchRequest, BatchResponse};
use axum::{
    Router,
    extract::{
        State,
        ws::{Message, WebSocket, WebSocketUpgrade},
    },
    http::{HeaderMap, StatusCode},
    response::{IntoResponse, Response},
    routing::get,
};
use futures::StreamExt;
use serde_json::Value;
use std::{sync::Arc, time::Duration};
use tokio::sync::{OwnedSemaphorePermit, Semaphore};

const DEFAULT_MAX_MESSAGE_SIZE: usize = 1024 * 1024;
const DEFAULT_MAX_BATCH_SIZE: usize = 100;
const DEFAULT_MAX_CONNECTIONS: usize = 1024;
const DEFAULT_REQUEST_TIMEOUT: Duration = Duration::from_secs(30);

/// WebSocket transport for persistent JSON-RPC request/response sessions.
///
/// Each text or UTF-8 binary WebSocket message must contain one JSON-RPC
/// request or a JSON-RPC batch. Responses are sent on the same connection in
/// request order. Requests from different connections are processed
/// concurrently.
pub struct WebSocketTransport {
    server: Arc<RpcServer>,
    auth: Option<Arc<AuthMiddleware>>,
    metrics: Option<Arc<Metrics>>,
    max_message_size: usize,
    max_batch_size: usize,
    request_timeout: Duration,
    connection_limit: Arc<Semaphore>,
}

impl WebSocketTransport {
    pub fn new(server: Arc<RpcServer>) -> Self {
        Self {
            server,
            auth: None,
            metrics: None,
            max_message_size: DEFAULT_MAX_MESSAGE_SIZE,
            max_batch_size: DEFAULT_MAX_BATCH_SIZE,
            request_timeout: DEFAULT_REQUEST_TIMEOUT,
            connection_limit: Arc::new(Semaphore::new(DEFAULT_MAX_CONNECTIONS)),
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

    pub fn with_max_message_size(mut self, bytes: usize) -> Self {
        self.max_message_size = bytes.max(1);
        self
    }

    pub fn with_max_batch_size(mut self, requests: usize) -> Self {
        self.max_batch_size = requests.max(1);
        self
    }

    pub fn with_max_connections(mut self, connections: usize) -> Self {
        self.connection_limit = Arc::new(Semaphore::new(connections.max(1)));
        self
    }

    pub fn with_request_timeout(mut self, timeout: Duration) -> Self {
        self.request_timeout = timeout;
        self
    }

    /// Build an Axum router with a WebSocket endpoint at `/ws`.
    pub fn router(self) -> Router {
        Router::new()
            .route("/ws", get(websocket_handler))
            .with_state(Arc::new(self))
    }

    pub async fn serve(self, addr: &str) -> anyhow::Result<()> {
        let listener = tokio::net::TcpListener::bind(addr).await?;
        println!("WebSocket RPC server listening on ws://{addr}/ws");
        axum::serve(listener, self.router()).await?;
        Ok(())
    }
}

async fn websocket_handler(
    State(transport): State<Arc<WebSocketTransport>>,
    headers: HeaderMap,
    ws: WebSocketUpgrade,
) -> Response {
    if let Err(response) = authenticate_upgrade(&transport, &headers).await {
        return response;
    }

    let permit = match transport.connection_limit.clone().try_acquire_owned() {
        Ok(permit) => permit,
        Err(_) => {
            return (
                StatusCode::SERVICE_UNAVAILABLE,
                "WebSocket connection limit reached",
            )
                .into_response();
        }
    };

    let max_message_size = transport.max_message_size;
    ws.max_message_size(max_message_size)
        .max_frame_size(max_message_size)
        .on_upgrade(move |socket| handle_socket(socket, transport, permit))
}

async fn authenticate_upgrade(
    transport: &WebSocketTransport,
    headers: &HeaderMap,
) -> Result<(), Response> {
    let Some(auth) = transport.auth.as_deref() else {
        return Ok(());
    };
    if auth.strategy() != AuthStrategy::ApiKeyInHeader {
        return Ok(());
    }

    let key = headers
        .get(API_KEY_HEADER)
        .and_then(|value| value.to_str().ok());
    auth.validate_key(key).await.map_err(|error| {
        let status = if error.code == AUTH_REQUIRED {
            StatusCode::UNAUTHORIZED
        } else {
            StatusCode::FORBIDDEN
        };
        (status, error.message).into_response()
    })
}

async fn handle_socket(
    mut socket: WebSocket,
    transport: Arc<WebSocketTransport>,
    _permit: OwnedSemaphorePermit,
) {
    while let Some(message) = socket.next().await {
        let response = match message {
            Ok(Message::Text(text)) => process_payload(&transport, text.as_bytes()).await,
            Ok(Message::Binary(bytes)) => process_payload(&transport, &bytes).await,
            Ok(Message::Ping(payload)) => {
                if socket.send(Message::Pong(payload)).await.is_err() {
                    break;
                }
                continue;
            }
            Ok(Message::Pong(_)) => continue,
            Ok(Message::Close(_)) | Err(_) => break,
        };

        let response = match serde_json::to_string(&response) {
            Ok(response) => response,
            Err(_) => serde_json::to_string(&RpcResponse::with_error(
                Value::Null,
                INTERNAL_ERROR,
                "Failed to serialize response",
            ))
            .expect("static JSON-RPC error response must serialize"),
        };

        if socket.send(Message::Text(response.into())).await.is_err() {
            break;
        }
    }
}

async fn process_payload(transport: &WebSocketTransport, payload: &[u8]) -> BatchResponse {
    if payload.len() > transport.max_message_size {
        return error_response(
            INVALID_REQUEST,
            format!(
                "Message exceeds maximum of {} bytes",
                transport.max_message_size
            ),
        );
    }

    let value = match serde_json::from_slice::<Value>(payload) {
        Ok(value) => value,
        Err(error) => return error_response(PARSE_ERROR, format!("Parse error: {error}")),
    };
    let batch = match serde_json::from_value::<BatchRequest>(value) {
        Ok(batch) => batch,
        Err(error) => {
            return error_response(INVALID_REQUEST, format!("Invalid Request: {error}"));
        }
    };

    if batch.len() > transport.max_batch_size {
        return error_response(
            INVALID_REQUEST,
            format!(
                "Batch exceeds maximum of {} requests",
                transport.max_batch_size
            ),
        );
    }

    let method = match &batch {
        BatchRequest::Single(request) => request.method.clone(),
        BatchRequest::Batch(requests) => format!("batch({})", requests.len()),
    };
    let tracer = transport
        .metrics
        .as_ref()
        .map(|metrics| RequestTracer::new(method, metrics.clone()));

    let response = match tokio::time::timeout(
        transport.request_timeout,
        handle_websocket_batch(transport, batch),
    )
    .await
    {
        Ok(response) => response,
        Err(_) => error_response(INTERNAL_ERROR, "Request timed out"),
    };

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

    response
}

async fn handle_websocket_batch(
    transport: &WebSocketTransport,
    batch: BatchRequest,
) -> BatchResponse {
    let Some(auth) = transport.auth.as_deref() else {
        return transport.server.handle_batch(batch).await;
    };

    match auth.strategy() {
        AuthStrategy::None | AuthStrategy::ApiKeyInHeader => {
            transport.server.handle_batch(batch).await
        }
        AuthStrategy::ApiKeyInParams => match batch {
            BatchRequest::Single(request) => BatchResponse::Single(
                transport
                    .server
                    .handle_authenticated_request(request, auth)
                    .await,
            ),
            BatchRequest::Batch(requests) => {
                if requests.is_empty() {
                    return error_response(INVALID_REQUEST, "Invalid Request: empty batch");
                }
                let futures = requests
                    .into_iter()
                    .map(|request| transport.server.handle_authenticated_request(request, auth));
                BatchResponse::Batch(futures::future::join_all(futures).await)
            }
        },
    }
}

fn error_response(code: i64, message: impl Into<String>) -> BatchResponse {
    BatchResponse::Single(RpcResponse::with_error(Value::Null, code, message))
}
