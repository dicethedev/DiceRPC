use anyhow::Context;
use dice_rpc::{AuthMiddleware, AuthStrategy, RpcServer, rpc, transport::HttpTransport};
use std::sync::Arc;

/// HTTP API-key authentication using the `x-api-key` header.
///
/// Run:
/// API_KEY=replace-me cargo run --example http_with_header_auth
///
/// Call:
/// curl http://127.0.0.1:3000/rpc \
///   -H 'content-type: application/json' \
///   -H "x-api-key: $API_KEY" \
///   -d '{"jsonrpc":"2.0","method":"ping","params":{},"id":1}'
#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let api_key = std::env::var("API_KEY").context("set API_KEY before starting the example")?;
    let server = Arc::new(RpcServer::new());
    rpc::register_default_handlers(&server).await;

    let auth = Arc::new(AuthMiddleware::new(AuthStrategy::ApiKeyInHeader));
    auth.add_key(api_key).await;

    HttpTransport::new(server)
        .with_auth(auth)
        .serve("127.0.0.1:3000")
        .await
}
