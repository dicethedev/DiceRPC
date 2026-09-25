//! Minimal JSON-RPC server over WebSocket.
//!
//! Run with:
//! `cargo run --example websocket_server --features websocket`

#[cfg(feature = "websocket")]
use dice_rpc::{RpcServer, transport::WebSocketTransport};
#[cfg(feature = "websocket")]
use serde_json::json;
#[cfg(feature = "websocket")]
use std::sync::Arc;

#[cfg(feature = "websocket")]
#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let server = Arc::new(RpcServer::new());
    server
        .register("ping", |_| async move { Ok(json!("pong")) })
        .await;
    server
        .register("greet", |params| async move {
            let name = params
                .get("name")
                .and_then(|value| value.as_str())
                .unwrap_or("friend");
            Ok(json!({ "message": format!("Hello, {name}!") }))
        })
        .await;

    WebSocketTransport::new(server)
        .serve("127.0.0.1:3001")
        .await
}

#[cfg(not(feature = "websocket"))]
fn main() {
    eprintln!("enable the `websocket` feature to run this example");
}
