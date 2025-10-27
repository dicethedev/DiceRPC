#[cfg(feature = "http")] // means - Only compile the following code if the Cargo feature named http is enabled.
mod http_tests {
    use dice_rpc::*;
    use std::sync::Arc;
    use serde_json::json;

    #[tokio::test]
    async fn test_http_transport_basic() {
        let server = Arc::new(RpcServer::new());
        rpc::register_default_handlers(&server).await;

        // Create HTTP transport
        let http = transport::HttpTransport::new(server);
        let router = http.router();

        // Test with axum test helpers
        // (You'd need axum-test crate for this)
    }

    #[tokio::test]
    async fn test_http_with_auth() {
        let server = Arc::new(RpcServer::new());
        rpc::register_default_handlers(&server).await;

        let auth = Arc::new(middleware::AuthMiddleware::new(
            middleware::AuthStrategy::ApiKeyInParams
        ));
        auth.add_key("test-key").await;

        let http = transport::HttpTransport::new(server)
            .with_auth(auth);

        let _router = http.router();
        // Test authenticated requests
    }
}