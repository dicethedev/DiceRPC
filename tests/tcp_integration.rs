//! Integration tests for TCP transport
//! Run with: cargo test --features tcp

#[cfg(feature = "tcp")]
mod tcp_tests {
    use dice_rpc::*;
    use serde_json::json;
    use std::sync::Arc;
    use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
    use tokio::net::TcpStream;

    /// Helper to connect and send a request
    async fn send_request(
        addr: &str,
        method: &str,
        params: serde_json::Value,
    ) -> anyhow::Result<RpcResponse> {
        let stream = TcpStream::connect(addr).await?;
        let (read_half, mut write_half) = stream.into_split();
        let mut reader = BufReader::new(read_half);

        let req = json!({
            "jsonrpc": "2.0",
            "method": method,
            "params": params,
            "id": 1
        });

        let req_text = serde_json::to_string(&req)? + "\n";
        write_half.write_all(req_text.as_bytes()).await?;

        let mut line = String::new();
        reader.read_line(&mut line).await?;

        let response: RpcResponse = serde_json::from_str(&line)?;
        Ok(response)
    }

    /// Helper to send framed request
    async fn send_framed_request(
        addr: &str,
        method: &str,
        params: serde_json::Value,
    ) -> anyhow::Result<RpcResponse> {
        use dice_rpc::transport::FrameCodec;

        let mut stream = TcpStream::connect(addr).await?;

        let req = json!({
            "jsonrpc": "2.0",
            "method": method,
            "params": params,
            "id": 1
        });

        let req_bytes = serde_json::to_vec(&req)?;
        FrameCodec::write_frame(&mut stream, &req_bytes).await?;

        let resp_bytes = FrameCodec::read_frame(&mut stream).await?;
        let response: RpcResponse = serde_json::from_slice(&resp_bytes)?;

        Ok(response)
    }

    #[tokio::test]
    async fn test_basic_tcp_server() {
        // Start server in background
        let addr = "127.0.0.1:14001";
        tokio::spawn(async move {
            let server = Arc::new(RpcServer::new());
            rpc::register_default_handlers(&server).await;
            let _ = server::server::run(addr).await;
        });

        // Give server time to start
        tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;

        // Test ping
        let response = send_request(addr, "ping", json!({})).await.unwrap();
        assert!(response.error.is_none());
        assert_eq!(response.result, Some(json!("pong")));
    }

    #[tokio::test]
    async fn test_tcp_with_state() {
        let addr = "127.0.0.1:14002";

        // Setup server
        tokio::spawn(async move {
            let server = Arc::new(RpcServer::new());
            let state = Arc::new(state::StateStore::new());
            
            // Initialize with test data
            state.set_balance("0xAlice", 1000).await;
            state.set_balance("0xBob", 500).await;
            
            server::handlers::register_stateful_handlers(&server, state).await;
            let _ = server::server::run(addr).await;
        });

        tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;

        // Test get_balance
        let response = send_request(
            addr,
            "get_balance",
            json!({"address": "0xAlice"}),
        )
        .await
        .unwrap();

        assert!(response.error.is_none());
        let result = response.result.unwrap();
        assert_eq!(result["balance"], "1000");
    }

    #[tokio::test]
    async fn test_tcp_transfer() {
        let addr = "127.0.0.1:14003";

        // Setup server
        let state = Arc::new(state::StateStore::new());
        state.set_balance("0xAlice", 1000).await;
        state.set_balance("0xBob", 500).await;

        let state_clone = state.clone();
        tokio::spawn(async move {
            let server = Arc::new(RpcServer::new());
            server::handlers::register_stateful_handlers(&server, state_clone).await;
            let _ = server::server::run(addr).await;
        });

        tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;

        // Test transfer
        let response = send_request(
            addr,
            "transfer",
            json!({
                "from": "0xAlice",
                "to": "0xBob",
                "amount": 300
            }),
        )
        .await
        .unwrap();

        assert!(response.error.is_none());
        let result = response.result.unwrap();
        assert_eq!(result["from"], "0xAlice");
        assert_eq!(result["to"], "0xBob");
        assert_eq!(result["amount"], 300);

        // Verify balances changed
        assert_eq!(state.get_balance("0xAlice").await, Some(700));
        assert_eq!(state.get_balance("0xBob").await, Some(800));
    }

    #[tokio::test]
    async fn test_tcp_framed_server() {
        let addr = "127.0.0.1:14004";

        // Start framed server
        tokio::spawn(async move {
            let server = Arc::new(RpcServer::new());
            rpc::register_default_handlers(&server).await;
            
            let config = transport::tcp::TcpServerConfig::new(addr, server);
            let _ = transport::tcp::run_with_framing(config).await;
        });

        tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;

        // Test with framed protocol
        let response = send_framed_request(addr, "ping", json!({}))
            .await
            .unwrap();

        assert!(response.error.is_none());
        assert_eq!(response.result, Some(json!("pong")));
    }

    #[tokio::test]
    async fn test_tcp_batch_requests() {
        let addr = "127.0.0.1:14005";

        tokio::spawn(async move {
            let server = Arc::new(RpcServer::new());
            rpc::register_default_handlers(&server).await;
            let config = transport::tcp::TcpServerConfig::new(addr, server);
            let _ = transport::tcp::run_with_framing(config).await;
        });

        tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;

        // Send batch request
        use dice_rpc::transport::FrameCodec;
        let mut stream = TcpStream::connect(addr).await.unwrap();

        let batch = json!([
            {"jsonrpc": "2.0", "method": "ping", "params": {}, "id": 1},
            {"jsonrpc": "2.0", "method": "ping", "params": {}, "id": 2},
            {"jsonrpc": "2.0", "method": "ping", "params": {}, "id": 3},
        ]);

        let req_bytes = serde_json::to_vec(&batch).unwrap();
        FrameCodec::write_frame(&mut stream, &req_bytes)
            .await
            .unwrap();

        let resp_bytes = FrameCodec::read_frame(&mut stream).await.unwrap();
        let responses: Vec<RpcResponse> = serde_json::from_slice(&resp_bytes).unwrap();

        assert_eq!(responses.len(), 3);
        for resp in responses {
            assert!(resp.error.is_none());
            assert_eq!(resp.result, Some(json!("pong")));
        }
    }

    #[tokio::test]
    async fn test_tcp_with_auth() {
        let addr = "127.0.0.1:14006";

        // Setup server with auth
        tokio::spawn(async move {
            let server = Arc::new(RpcServer::new());
            rpc::register_default_handlers(&server).await;

            let auth = Arc::new(middleware::AuthMiddleware::new(
                middleware::AuthStrategy::ApiKeyInParams,
            ));
            auth.add_key("test-key-123").await;

            let config = transport::tcp::TcpServerConfig::new(addr, server)
                .with_auth(auth);

            let _ = transport::tcp::run_with_framing(config).await;
        });

        tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;

        // Test with valid key
        let response = send_framed_request(
            addr,
            "ping",
            json!({"api_key": "test-key-123"}),
        )
        .await
        .unwrap();
        assert!(response.error.is_none());

        // Test with invalid key
        let response = send_framed_request(
            addr,
            "ping",
            json!({"api_key": "wrong-key"}),
        )
        .await
        .unwrap();
        assert!(response.error.is_some());
    }

    #[tokio::test]
    async fn test_tcp_error_handling() {
        let addr = "127.0.0.1:14007";

        tokio::spawn(async move {
            let server = Arc::new(RpcServer::new());
            rpc::register_default_handlers(&server).await;
            let _ = server::server::run(addr).await;
        });

        tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;

        // Test method not found
        let response = send_request(addr, "nonexistent_method", json!({}))
            .await
            .unwrap();

        assert!(response.error.is_some());
        let error = response.error.unwrap();
        assert!(error.message.contains("Method not found"));

        // Test invalid params
        let response = send_request(
            addr,
            "get_balance",
            json!({}), // Missing required address param
        )
        .await
        .unwrap();

        assert!(response.error.is_some());
    }
}