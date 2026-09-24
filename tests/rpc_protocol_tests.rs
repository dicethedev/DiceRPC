use dice_rpc::rpc::{INVALID_REQUEST, METHOD_NOT_FOUND};
use dice_rpc::{RpcErrorObj, RpcRequest, RpcResponse, RpcServer};
use serde_json::json;

#[tokio::test]
async fn method_not_found_uses_standard_code() {
    let response = RpcServer::new()
        .handle_request(RpcRequest {
            jsonrpc: "2.0".into(),
            method: "missing".into(),
            params: json!({}),
            id: json!(1),
        })
        .await;
    assert_eq!(response.error.unwrap().code, METHOD_NOT_FOUND);
    assert_eq!(METHOD_NOT_FOUND, -32601);
}

#[tokio::test]
async fn invalid_protocol_version_is_rejected() {
    let response = RpcServer::new()
        .handle_request(RpcRequest {
            jsonrpc: "1.0".into(),
            method: "ping".into(),
            params: json!({}),
            id: json!(1),
        })
        .await;
    assert_eq!(response.error.unwrap().code, INVALID_REQUEST);
}

#[tokio::test]
async fn invalid_id_type_is_rejected_with_null_id() {
    let response = RpcServer::new()
        .handle_request(RpcRequest {
            jsonrpc: "2.0".into(),
            method: "ping".into(),
            params: json!({}),
            id: json!({"invalid": true}),
        })
        .await;
    assert_eq!(response.id, serde_json::Value::Null);
    assert_eq!(response.error.unwrap().code, INVALID_REQUEST);
}

#[test]
fn responses_serialize_exactly_one_result_or_error_member() {
    let success = serde_json::to_value(RpcResponse::with_result(json!(1), json!("ok"))).unwrap();
    assert!(success.get("result").is_some());
    assert!(success.get("error").is_none());

    let failure =
        serde_json::to_value(RpcResponse::with_error(json!(1), -32000, "failed")).unwrap();
    assert!(failure.get("result").is_none());
    assert!(failure.get("error").is_some());
}

#[tokio::test]
async fn handler_error_data_is_preserved() {
    let server = RpcServer::new();
    server
        .register("fail", |_| async {
            Err(RpcErrorObj {
                code: -32010,
                message: "failure".into(),
                data: Some(json!({"reason": "test"})),
            })
        })
        .await;
    let response = server
        .handle_request(RpcRequest {
            jsonrpc: "2.0".into(),
            method: "fail".into(),
            params: json!({}),
            id: json!(1),
        })
        .await;
    assert_eq!(
        response.error.unwrap().data,
        Some(json!({"reason": "test"}))
    );
}
