use anyhow::Result;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use uuid::Uuid;

#[derive(Debug, Serialize, Deserialize)]
pub struct RpcRequest {
    pub jsonrpc: String,
    pub method: String,
    #[serde(default)]
    pub params: Value,
    pub id: Value, // id can be string or number or null
}

#[derive(Debug, Serialize, Deserialize)]
pub struct RpcErrorObj {
    pub code: i64,
    pub message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub data: Option<Value>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct RpcResponse {
    pub jsonrpc: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub result: Option<Value>,
    pub error: Option<RpcErrorObj>,
    pub id: Value,
}

pub const METHOD_NOT_FOUND: i64 = -32602;
pub const INVALID_PARAMS: i64 = -32602;

/// Helper methods for constructing JSON-RPC 2.0 responses.
///
/// `RpcResponse` represents a standard JSON-RPC response object,
/// including the `jsonrpc` version, `result`, `error`, and `id`.
/// These helpers make it easy to create success or error responses
/// in a consistent way.
impl RpcResponse {

    /// Constructs a successful JSON-RPC response with the given `id` and `result`.
    ///
    /// # Arguments
    /// * `id` - The request ID to correlate the response with.
    /// * `res` - The result value returned from the RPC call.
    ///
    /// # Example
    /// ```rust
    /// let res = RpcResponse::with_result(request_id, serde_json::json!({"balance": 100}));
    /// ```
    pub fn with_result(id: Value, res: Value) -> Self {
        RpcResponse {
            jsonrpc: "2.0".to_string(),
            result: Some(res),
            error: None,
            id,
        }
    }

    /// Constructs an error JSON-RPC response with the given `id`, `code`, and `message`.
    ///
    /// # Arguments
    /// * `id` - The request ID to correlate the response with.
    /// * `code` - The numeric error code (e.g., `-32601` for method not found).
    /// * `message` - A human-readable error message describing the problem.
    ///
    /// # Example
    /// ```rust
    /// let err = RpcResponse::with_error(request_id, -32601, "Method not found");
    /// ```

    pub fn with_error(id: Value, code: i64, message: impl Into<String>) -> Self {
        RpcResponse {
            jsonrpc: "2.0".to_string(),
            result: None,
            error: Some(RpcErrorObj {
                code,
                message: message.into(),
                data: None,
            }),
            id,
        }
    }
}

// A Handler is an async function that takes params and returns Result<Value, RpcErrorObj>.
pub type Handler = dyn Fn(Value) -> HandlerFuture + Send + Sync + 'static;
pub type HandlerFuture =
    std::pin::Pin<Box<dyn std::future::Future<Output = Result<Value, RpcErrorObj>> + Send>>;


/// Represents a lightweight asynchronous JSON-RPC server.
///
/// The `RpcServer` stores a set of method handlers that can be
/// registered dynamically and called concurrently. Each handler is
/// stored as an `Arc<Handler>` inside an `RwLock<HashMap>` to allow
/// safe shared access across asynchronous tasks.
///
/// # Fields
/// - `handlers`: A thread-safe map from method names (`String`) to
///   their corresponding RPC handlers (`Arc<Handler>`).
/// ```
pub struct RpcServer {
    handlers: RwLock<HashMap<String, Arc<Handler>>>,
}

/// Implementation of the core functionality for the `RpcServer`.
///
/// This `RpcServer` provides a lightweight, asynchronous JSON-RPC–style
/// handler registry and request processor. It allows dynamic registration of
/// RPC methods before the server starts, and safe concurrent access at runtime.
///
/// ### Key Methods
///
/// **`new()`**
/// - Creates a new `RpcServer` instance with an empty handler map.
/// - The handlers are stored in a `RwLock<HashMap<String, Arc<Handler>>>`,
///   allowing thread-safe reads and writes.
///
/// **`register()`**
/// - Registers a new RPC method handler.
/// - Takes a method name (`&str`) and an async function that accepts a `serde_json::Value`
///   as input parameters and returns a `Result<Value, RpcErrorObj>`.
/// - The handler is boxed and wrapped in an `Arc` for shared ownership and inserted into the internal map.
/// - Example:
///   ```rust
///   server.register("ping", |_params| async move {
///       Ok(Value::String("pong".into()))
///   }).await;
///   ```
///
/// **`handle_request()`**
/// - Processes an incoming `RpcRequest` by matching its `method` against the registered handlers.
/// - If found, it awaits the handler’s async execution and wraps the output into a `RpcResponse`.
/// - On success, returns a response with the handler’s result.
/// - On failure
impl RpcServer {
    pub fn new() -> Self {
        Self {
            handlers: RwLock::new(HashMap::new()),
        }
    }

    pub async fn register<F, Fut>(&self, method: &str, f: F)
    where
        F: Fn(Value) -> Fut + Send + Sync + 'static,
        Fut: std::future::Future<Output = Result<Value, RpcErrorObj>> + Send + 'static,
    {
        //wrap into Arc<Handlers>
        let method_name = method.to_string();
        let handler_arc: Arc<Handler> = Arc::new(move |params: Value| {
            let fut = f(params);
            Box::pin(fut)
        });

        self.handlers.write().await.insert(method_name, handler_arc);
    }

    pub async fn handle_request(&self, req: RpcRequest) -> RpcResponse {
        let id = req.id.clone();
        let handlers = self.handlers.read().await;
        if let Some(h) = handlers.get(&req.method) {
            // call handler
            match (h)(req.params).await {
                Ok(res) => RpcResponse::with_result(id, res),
                Err(err) => RpcResponse::with_error(id, err.code, err.message),
            }
        } else {
            RpcResponse::with_error(
                id,
                METHOD_NOT_FOUND,
                format!("Method not found: {}", req.method),
            )
        }
    }
}


// Helper function to parse raw JSON string into RpcRequest
pub fn parse_rpc_request(raw: &str) -> Result<RpcRequest, serde_json::Error> {
    serde_json::from_str::<RpcRequest>(raw)
}

/// Registers a set of default RPC handlers for the given `RpcServer`.
///
/// This function sets up three basic endpoints commonly used for testing or demo purposes:
///
/// 1. **`ping`** → Always responds with `"pong"`.
///    - Usage: `{ "method": "ping" }`
///    - Response: `"pong"`
///
/// 2. **`get_balance`** → Returns a fake balance string based on the length of the provided address.
///    - Usage: `{ "method": "get_balance", "params": { "address": "0x123..." } }`
///    - Response: `"123450"` (a deterministic number derived from address length)
///    - If `address` is missing, returns an RPC error with code `INVALID_PARAMS`.
///
/// 3. **`send_tx`** → Simulates sending a transaction by returning a randomly generated UUID as a fake transaction ID.
///    - Usage: `{ "method": "send_tx", "params": { "raw_tx": "0xabc..." } }`
///    - Response: `"b6e1a47b-9cf1-42f1-b087-30d44c48e4f3"`
///    - If `raw_tx` is missing, returns an RPC error with code `INVALID_PARAMS`.
///
/// ## Implementation Notes
/// - Each handler is registered asynchronously before the server starts.
/// - The `RpcServer` maintains its handlers in a `RwLock<HashMap>` allowing concurrent reads
///   and safe registration of handlers prior to running the server.
/// - These handlers serve as mock implementations useful for testing RPC integration
///   or demonstrating how to define async RPC endpoints.

pub async fn register_default_handlers(server: &RpcServer) {
    // ping -> "pong"
    server.register("ping", |_params| async move {
        Ok(Value::String("pong".into()))
    }).await;

    // get_balance -> params { address: "0x..." } -> returns string of fake balance
    server.register("get_balance", |params| async move {
        // accept either object or array. We'll expect object with "address"
        let address = if params.is_object() {
            params.get("address").and_then(|v| v.as_str()).unwrap_or("")
        } else {
            ""
        };
        if address.is_empty() {
            return Err(RpcErrorObj { code: INVALID_PARAMS, message: "Missing 'address' param".into(), data: None });
        }
        // fake balance: length-based deterministic value for demo
        let bal = (address.len() * 12345) as u64;
        Ok(Value::String(format!("{}", bal)))
    }).await;

    // send_tx -> params { raw_tx: "0x..." } -> returns txid
    server.register("send_tx", |params| async move {
        let raw = if params.is_object() {
            params.get("raw_tx").and_then(|v| v.as_str()).unwrap_or("")
        } else {
            ""
        };
        if raw.is_empty() {
            return Err(RpcErrorObj { code: INVALID_PARAMS, message: "Missing 'raw_tx' param".into(), data: None });
        }
        // "send" generates a uuid txid
        let txid = Uuid::new_v4().to_string();
        Ok(Value::String(txid))
    }).await;
}
