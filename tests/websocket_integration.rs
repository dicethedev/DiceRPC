#![cfg(feature = "websocket")]

use dice_rpc::{AuthMiddleware, AuthStrategy, RpcServer, transport::WebSocketTransport};
use futures::{SinkExt, StreamExt};
use serde_json::{Value, json};
use std::{net::SocketAddr, sync::Arc};
use tokio::task::JoinHandle;
use tokio_tungstenite::{
    connect_async,
    tungstenite::{Message, client::IntoClientRequest, http::HeaderValue},
};

struct TestServer {
    addr: SocketAddr,
    task: JoinHandle<()>,
}

impl Drop for TestServer {
    fn drop(&mut self) {
        self.task.abort();
    }
}

async fn start_server(transport: WebSocketTransport) -> TestServer {
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();
    let task = tokio::spawn(async move {
        axum::serve(listener, transport.router()).await.unwrap();
    });
    TestServer { addr, task }
}

async fn ping_server() -> Arc<RpcServer> {
    let server = Arc::new(RpcServer::new());
    server
        .register("ping", |_| async move { Ok(json!("pong")) })
        .await;
    server
}

#[tokio::test]
async fn handles_single_and_batch_requests_on_one_connection() {
    let test_server = start_server(WebSocketTransport::new(ping_server().await)).await;
    let (mut socket, _) = connect_async(format!("ws://{}/ws", test_server.addr))
        .await
        .unwrap();

    socket
        .send(Message::Text(
            json!({"jsonrpc":"2.0","method":"ping","params":{},"id":1})
                .to_string()
                .into(),
        ))
        .await
        .unwrap();
    let response: Value =
        serde_json::from_str(socket.next().await.unwrap().unwrap().to_text().unwrap()).unwrap();
    assert_eq!(response["result"], "pong");
    assert_eq!(response["id"], 1);

    socket
        .send(Message::Text(
            json!([
                {"jsonrpc":"2.0","method":"ping","params":{},"id":2},
                {"jsonrpc":"2.0","method":"ping","params":{},"id":3}
            ])
            .to_string()
            .into(),
        ))
        .await
        .unwrap();
    let response: Value =
        serde_json::from_str(socket.next().await.unwrap().unwrap().to_text().unwrap()).unwrap();
    assert_eq!(response.as_array().unwrap().len(), 2);
    assert_eq!(response[0]["result"], "pong");
    assert_eq!(response[1]["id"], 3);
}

#[tokio::test]
async fn rejects_unauthorized_handshakes_and_accepts_valid_keys() {
    let auth = Arc::new(AuthMiddleware::new(AuthStrategy::ApiKeyInHeader));
    assert!(auth.add_key("correct-secret").await);
    let test_server =
        start_server(WebSocketTransport::new(ping_server().await).with_auth(auth)).await;
    let url = format!("ws://{}/ws", test_server.addr);

    let error = connect_async(&url).await.unwrap_err();
    match error {
        tokio_tungstenite::tungstenite::Error::Http(response) => {
            assert_eq!(response.status(), 401);
        }
        other => panic!("expected an HTTP handshake rejection, got {other}"),
    }

    let mut request = url.into_client_request().unwrap();
    request
        .headers_mut()
        .insert("x-api-key", HeaderValue::from_static("correct-secret"));
    let (mut socket, _) = connect_async(request).await.unwrap();
    socket
        .send(Message::Text(
            json!({"jsonrpc":"2.0","method":"ping","params":{},"id":1})
                .to_string()
                .into(),
        ))
        .await
        .unwrap();
    let response: Value =
        serde_json::from_str(socket.next().await.unwrap().unwrap().to_text().unwrap()).unwrap();
    assert_eq!(response["result"], "pong");
}

#[tokio::test]
async fn reports_parse_errors_and_enforces_batch_limits() {
    let test_server =
        start_server(WebSocketTransport::new(ping_server().await).with_max_batch_size(1)).await;
    let (mut socket, _) = connect_async(format!("ws://{}/ws", test_server.addr))
        .await
        .unwrap();

    socket.send(Message::Text("not-json".into())).await.unwrap();
    let response: Value =
        serde_json::from_str(socket.next().await.unwrap().unwrap().to_text().unwrap()).unwrap();
    assert_eq!(response["error"]["code"], -32700);

    socket
        .send(Message::Text(
            json!([
                {"jsonrpc":"2.0","method":"ping","params":{},"id":1},
                {"jsonrpc":"2.0","method":"ping","params":{},"id":2}
            ])
            .to_string()
            .into(),
        ))
        .await
        .unwrap();
    let response: Value =
        serde_json::from_str(socket.next().await.unwrap().unwrap().to_text().unwrap()).unwrap();
    assert_eq!(response["error"]["code"], -32600);
}

#[tokio::test]
async fn supports_parameter_authentication_per_message() {
    let auth = Arc::new(AuthMiddleware::new(AuthStrategy::ApiKeyInParams));
    assert!(auth.add_key("correct-secret").await);
    let test_server =
        start_server(WebSocketTransport::new(ping_server().await).with_auth(auth)).await;
    let (mut socket, _) = connect_async(format!("ws://{}/ws", test_server.addr))
        .await
        .unwrap();

    socket
        .send(Message::Text(
            json!({"jsonrpc":"2.0","method":"ping","params":{},"id":1})
                .to_string()
                .into(),
        ))
        .await
        .unwrap();
    let response: Value =
        serde_json::from_str(socket.next().await.unwrap().unwrap().to_text().unwrap()).unwrap();
    assert_eq!(response["error"]["code"], -32002);

    socket
        .send(Message::Text(
            json!({
                "jsonrpc":"2.0",
                "method":"ping",
                "params":{"api_key":"correct-secret"},
                "id":2
            })
            .to_string()
            .into(),
        ))
        .await
        .unwrap();
    let response: Value =
        serde_json::from_str(socket.next().await.unwrap().unwrap().to_text().unwrap()).unwrap();
    assert_eq!(response["result"], "pong");
}
