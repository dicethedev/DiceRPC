#[cfg(feature = "http")]
mod http_tests {
    use axum::{
        Router,
        body::{Body, to_bytes},
        http::Request,
    };
    use dice_rpc::{
        RpcServer,
        middleware::{AUTH_ERROR, AUTH_REQUIRED, AuthMiddleware, AuthStrategy},
        rpc,
        transport::HttpTransport,
    };
    use serde_json::{Value, json};
    use std::sync::Arc;
    use tower::ServiceExt;

    async fn call(router: Router, body: Value, api_key: Option<&str>) -> Value {
        let mut request = Request::builder()
            .method("POST")
            .uri("/rpc")
            .header("content-type", "application/json");
        if let Some(api_key) = api_key {
            request = request.header("x-api-key", api_key);
        }
        let response = router
            .oneshot(request.body(Body::from(body.to_string())).unwrap())
            .await
            .unwrap();
        let bytes = to_bytes(response.into_body(), 1024 * 1024).await.unwrap();
        serde_json::from_slice(&bytes).unwrap()
    }

    #[tokio::test]
    async fn basic_http_request_returns_result() {
        let server = Arc::new(RpcServer::new());
        rpc::register_default_handlers(&server).await;
        let response = call(
            HttpTransport::new(server).router(),
            json!({"jsonrpc":"2.0","method":"ping","params":{},"id":1}),
            None,
        )
        .await;
        assert_eq!(response["result"], "pong");
        assert!(response.get("error").is_none());
    }

    #[tokio::test]
    async fn parameter_authentication_is_enforced() {
        let server = Arc::new(RpcServer::new());
        rpc::register_default_handlers(&server).await;
        let auth = Arc::new(AuthMiddleware::new(AuthStrategy::ApiKeyInParams));
        auth.add_key("test-key").await;
        let router = HttpTransport::new(server).with_auth(auth).router();

        let missing = call(
            router.clone(),
            json!({"jsonrpc":"2.0","method":"ping","params":{},"id":1}),
            None,
        )
        .await;
        assert_eq!(missing["error"]["code"], AUTH_REQUIRED);

        let valid = call(
            router,
            json!({"jsonrpc":"2.0","method":"ping","params":{"api_key":"test-key"},"id":2}),
            None,
        )
        .await;
        assert_eq!(valid["result"], "pong");
    }

    #[tokio::test]
    async fn header_authentication_is_enforced() {
        let server = Arc::new(RpcServer::new());
        rpc::register_default_handlers(&server).await;
        let auth = Arc::new(AuthMiddleware::new(AuthStrategy::ApiKeyInHeader));
        auth.add_key("header-key").await;
        let router = HttpTransport::new(server).with_auth(auth).router();
        let request = json!({"jsonrpc":"2.0","method":"ping","params":{},"id":1});

        let invalid = call(router.clone(), request.clone(), Some("wrong-key")).await;
        assert_eq!(invalid["error"]["code"], AUTH_ERROR);

        let valid = call(router, request, Some("header-key")).await;
        assert_eq!(valid["result"], "pong");
    }

    #[tokio::test]
    async fn batch_limit_is_enforced() {
        let server = Arc::new(RpcServer::new());
        rpc::register_default_handlers(&server).await;
        let router = HttpTransport::new(server).with_max_batch_size(1).router();
        let response = call(
            router,
            json!([
                {"jsonrpc":"2.0","method":"ping","params":{},"id":1},
                {"jsonrpc":"2.0","method":"ping","params":{},"id":2}
            ]),
            None,
        )
        .await;
        assert_eq!(response["error"]["code"], -32600);
    }
}
