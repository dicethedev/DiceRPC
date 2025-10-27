use dice_rpc::middleware::auth::*;
use dice_rpc::rpc::RpcRequest;
use serde_json::json;

#[tokio::test]
async fn test_no_auth() {
    let auth = AuthMiddleware::new(AuthStrategy::None);
    let req = RpcRequest {
        jsonrpc: "2.0".to_string(),
        method: "ping".to_string(),
        params: json!({}),
        id: json!(1),
    };

    assert!(auth.validate_request(&req).await.is_ok());
}

#[tokio::test]
async fn test_api_key_auth_success() {
    let auth = AuthMiddleware::new(AuthStrategy::ApiKeyInParams);
    auth.add_key("test-key-123").await;

    let req = RpcRequest {
        jsonrpc: "2.0".to_string(),
        method: "ping".to_string(),
        params: json!({
            "api_key": "test-key-123"
        }),
        id: json!(1),
    };

    assert!(auth.validate_request(&req).await.is_ok());
}

#[tokio::test]
async fn test_api_key_auth_failure() {
    let auth = AuthMiddleware::new(AuthStrategy::ApiKeyInParams);
    auth.add_key("valid-key").await;

    let req = RpcRequest {
        jsonrpc: "2.0".to_string(),
        method: "ping".to_string(),
        params: json!({
            "api_key": "invalid-key"
        }),
        id: json!(1),
    };

    let result = auth.validate_request(&req).await;
    assert!(result.is_err());
    assert_eq!(result.unwrap_err().code, AUTH_ERROR);
}

#[tokio::test]
async fn test_missing_api_key() {
    let auth = AuthMiddleware::new(AuthStrategy::ApiKeyInParams);
    auth.add_key("test-key").await;

    let req = RpcRequest {
        jsonrpc: "2.0".to_string(),
        method: "ping".to_string(),
        params: json!({}),
        id: json!(1),
    };

    let result = auth.validate_request(&req).await;
    assert!(result.is_err());
    assert_eq!(result.unwrap_err().code, AUTH_REQUIRED);
}
