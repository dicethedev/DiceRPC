use crate::rpc::{RpcErrorObj, RpcRequest, RpcResponse};
use serde_json::Value;
use std::sync::Arc;
use subtle::ConstantTimeEq;
use tokio::sync::RwLock;

/// Authentication error codes
pub const AUTH_ERROR: i64 = -32001;
pub const AUTH_REQUIRED: i64 = -32002;

/// Authentication strategy
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AuthStrategy {
    /// No authentication required
    None,
    /// API key in params: { "api_key": "..." }
    ApiKeyInParams,
    /// API key in the `x-api-key` header (for HTTP and WebSocket transports)
    ApiKeyInHeader,
}

/// Authentication middleware for RPC requests
pub struct AuthMiddleware {
    strategy: AuthStrategy,
    valid_keys: Arc<RwLock<Vec<String>>>,
}

impl AuthMiddleware {
    /// Create a new authentication middleware
    pub fn new(strategy: AuthStrategy) -> Self {
        Self {
            strategy,
            valid_keys: Arc::new(RwLock::new(Vec::new())),
        }
    }

    /// Add a non-empty API key. Returns `true` when the key was added.
    pub async fn add_key(&self, key: impl Into<String>) -> bool {
        let key = key.into();
        if key.trim().is_empty() {
            return false;
        }

        let mut keys = self.valid_keys.write().await;
        if keys.iter().any(|existing| constant_time_eq(existing, &key)) {
            return false;
        }
        keys.push(key);
        true
    }

    /// Remove an API key
    pub async fn remove_key(&self, key: &str) -> bool {
        let mut keys = self.valid_keys.write().await;
        let original_len = keys.len();
        keys.retain(|existing| !constant_time_eq(existing, key));
        keys.len() != original_len
    }

    /// Check if a key is valid using constant-time byte comparison.
    pub async fn is_valid_key(&self, key: &str) -> bool {
        self.valid_keys
            .read()
            .await
            .iter()
            .any(|candidate| constant_time_eq(candidate, key))
    }

    pub fn strategy(&self) -> AuthStrategy {
        self.strategy
    }

    pub async fn validate_key(&self, key: Option<&str>) -> Result<(), RpcErrorObj> {
        let key = key
            .filter(|value| !value.is_empty())
            .ok_or_else(|| RpcErrorObj {
                code: AUTH_REQUIRED,
                message: "API key required".to_string(),
                data: None,
            })?;

        if self.is_valid_key(key).await {
            Ok(())
        } else {
            Err(RpcErrorObj {
                code: AUTH_ERROR,
                message: "Invalid API key".to_string(),
                data: None,
            })
        }
    }

    /// Validate a request based on the authentication strategy
    pub async fn validate_request(&self, req: &RpcRequest) -> Result<(), RpcErrorObj> {
        match &self.strategy {
            AuthStrategy::None => Ok(()),
            AuthStrategy::ApiKeyInParams => self.validate_params_key(req).await,
            AuthStrategy::ApiKeyInHeader => Err(RpcErrorObj {
                code: AUTH_REQUIRED,
                message: "Header authentication requires an HTTP-based transport".to_string(),
                data: None,
            }),
        }
    }

    /// Validate API key from request params
    async fn validate_params_key(&self, req: &RpcRequest) -> Result<(), RpcErrorObj> {
        let api_key = match &req.params {
            Value::Object(map) => {
                map.get("api_key")
                    .and_then(|v| v.as_str())
                    .ok_or_else(|| RpcErrorObj {
                        code: AUTH_REQUIRED,
                        message: "API key required in params".to_string(),
                        data: None,
                    })?
            }
            _ => {
                return Err(RpcErrorObj {
                    code: AUTH_REQUIRED,
                    message: "API key required in params".to_string(),
                    data: None,
                });
            }
        };

        self.validate_key(Some(api_key)).await
    }

    #[allow(dead_code)]
    /// Create an authentication error response
    pub fn auth_error_response(id: Value, message: impl Into<String>) -> RpcResponse {
        RpcResponse::with_error(id, AUTH_ERROR, message)
    }
}

fn constant_time_eq(left: &str, right: &str) -> bool {
    left.as_bytes().ct_eq(right.as_bytes()).into()
}

/// Extension trait for RpcServer to add authentication
#[allow(async_fn_in_trait)]
pub trait AuthenticatedServer {
    #[allow(dead_code)]
    async fn handle_authenticated_request(
        &self,
        req: RpcRequest,
        auth: &AuthMiddleware,
    ) -> RpcResponse;
}

impl AuthenticatedServer for crate::rpc::RpcServer {
    async fn handle_authenticated_request(
        &self,
        req: RpcRequest,
        auth: &AuthMiddleware,
    ) -> RpcResponse {
        // Validate authentication first
        if let Err(err) = auth.validate_request(&req).await {
            return RpcResponse::with_error_obj(req.response_id(), err);
        }

        // Process request if authenticated
        self.handle_request(req).await
    }
}
