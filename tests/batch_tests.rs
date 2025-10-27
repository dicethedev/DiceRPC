use dice_rpc::{BatchRequest, BatchResponse};
use dice_rpc::RpcRequest;
use dice_rpc::rpc;
use serde_json::json;

#[test]
fn test_parse_single_request() {
    let raw = r#"{"jsonrpc":"2.0","method":"ping","params":{},"id":1}"#;
    let batch = BatchRequest::parse(raw).unwrap();
    assert!(!batch.is_batch());
    assert_eq!(batch.len(), 1);
}

#[test]
fn test_parse_batch_request() {
    let raw = r#"[
            {"jsonrpc":"2.0","method":"ping","params":{},"id":1},
            {"jsonrpc":"2.0","method":"get_balance","params":{"address":"0x123"},"id":2}
        ]"#;
    let batch = BatchRequest::parse(raw).unwrap();
    assert!(batch.is_batch());
    assert_eq!(batch.len(), 2);
}

#[tokio::test]
async fn test_batch_processing() {
    use crate::rpc::RpcServer;

    let server = RpcServer::new();

    // Register a simple handler
    server
        .register("ping", |_| async move { Ok(json!("pong")) })
        .await;

    // Create batch request
    let requests = vec![
        RpcRequest {
            jsonrpc: "2.0".to_string(),
            method: "ping".to_string(),
            params: json!({}),
            id: json!(1),
        },
        RpcRequest {
            jsonrpc: "2.0".to_string(),
            method: "ping".to_string(),
            params: json!({}),
            id: json!(2),
        },
    ];

    let batch = BatchRequest::Batch(requests);
    let response = server.handle_batch(batch).await;

    match response {
        BatchResponse::Batch(responses) => {
            assert_eq!(responses.len(), 2);
            assert_eq!(responses[0].result, Some(json!("pong")));
            assert_eq!(responses[1].result, Some(json!("pong")));
        }
        _ => panic!("Expected batch response"),
    }
}
