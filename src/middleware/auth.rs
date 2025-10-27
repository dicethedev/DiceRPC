use crate::rpc::{RpcErrorObj, RpcRequest, RpcResponse};
use serde_json::Value;
use std::collections::HashSet;
use std::sync::Arc;
use tokio::sync::RwLock;

/// Authentication error codes
pub const AUTH_ERROR: i64 = -32001;
pub const AUTH_REQUIRED: i64 = -32002;

 #[allow(dead_code)]
/// Authentication strategy
#[derive(Clone)]
pub enum AuthStrategy {
    /// No authentication required
    None,
    /// API key in params: { "api_key": "..." }
    ApiKeyInParams,
    /// API key in custom header (for HTTP transport)
    ApiKeyInHeader,
}

/// Authentication middleware for RPC requests
pub struct AuthMiddleware {
    strategy: AuthStrategy,
    valid_keys: Arc<RwLock<HashSet<String>>>,
}

impl AuthMiddleware {
     #[allow(dead_code)]
    /// Create a new authentication middleware
    pub fn new(strategy: AuthStrategy) -> Self {
        Self {
            strategy,
            valid_keys: Arc::new(RwLock::new(HashSet::new())),
        }
    }

     #[allow(dead_code)]
    /// Add a valid API key
    pub async fn add_key(&self, key: impl Into<String>) {
        self.valid_keys.write().await.insert(key.into());
    }
     #[allow(dead_code)]
    /// Remove an API key
    pub async fn remove_key(&self, key: &str) {
        self.valid_keys.write().await.remove(key);
    }

    /// Check if a key is valid
    pub async fn is_valid_key(&self, key: &str) -> bool {
        self.valid_keys.read().await.contains(key)
    }

    /// Validate a request based on the authentication strategy
    pub async fn validate_request(&self, req: &RpcRequest) -> Result<(), RpcErrorObj> {
        match &self.strategy {
            AuthStrategy::None => Ok(()),
            AuthStrategy::ApiKeyInParams => self.validate_params_key(req).await,
            AuthStrategy::ApiKeyInHeader => {
                // For header-based auth, this would be checked at transport layer
                Ok(())
            }
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

        if self.is_valid_key(api_key).await {
            Ok(())
        } else {
            Err(RpcErrorObj {
                code: AUTH_ERROR,
                message: "Invalid API key".to_string(),
                data: None,
            })
        }
    }
 

     #[allow(dead_code)]
    /// Create an authentication error response
    pub fn auth_error_response(id: Value, message: impl Into<String>) -> RpcResponse {
        RpcResponse::with_error(id, AUTH_ERROR, message)
    }
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
            return RpcResponse::with_error(req.id.clone(), err.code, err.message);
        }

        // Process request if authenticated
        self.handle_request(req).await
    }
}
