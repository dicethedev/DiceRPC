/// Example: HTTP RPC Server with Authentication
///
/// Run with:
/// cargo run --example http_server --features http
use anyhow::Context;
use dice_rpc::*;
use std::sync::Arc;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // Initialize logging
    server::metrics::init_logging();

    // Create RPC server
    let server = Arc::new(RpcServer::new());

    // Register default handlers
    rpc::register_default_handlers(&server).await;

    // Create state store and register stateful handlers
    let state = Arc::new(state::StateStore::new());
    dice_rpc::server::handlers::register_stateful_handlers(&server, state.clone()).await;

    // Setup some initial test data
    state.set_balance("0xAlice", 10000).await;
    state.set_balance("0xBob", 5000).await;

    // Setup authentication
    let auth = Arc::new(middleware::AuthMiddleware::new(
        middleware::AuthStrategy::ApiKeyInParams,
    ));
    let api_key = std::env::var("API_KEY").context("set API_KEY before starting the example")?;
    auth.add_key(api_key).await;

    println!("╔══════════════════════════════════════════════╗");
    println!("║     DiceRPC HTTP Server with Auth           ║");
    println!("╚══════════════════════════════════════════════╝");
    println!();
    println!("Server running at: http://127.0.0.1:3000");
    println!("API key loaded from the API_KEY environment variable");
    println!();
    println!("Test with curl:");
    println!(r#"curl -X POST http://127.0.0.1:3000 \"#);
    println!(r#"  -H "Content-Type: application/json" \"#);
    println!(
        r#"  -d '{{"jsonrpc":"2.0","method":"ping","params":{{"api_key":"<API_KEY>"}},"id":1}}'"#
    );
    println!();

    // Create and start HTTP transport
    transport::HttpTransport::new(server)
        .with_auth(auth)
        .serve("127.0.0.1:3000")
        .await?;

    Ok(())
}
