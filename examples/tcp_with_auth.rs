/// TCP server with authentication
///
/// Run with:
/// cargo run --example tcp_with_auth --features tcp
use anyhow::Context;
use dice_rpc::*;
use std::sync::Arc;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    server::metrics::init_logging();

    println!("╔══════════════════════════════════════╗");
    println!("║  DiceRPC TCP Server with Auth        ║");
    println!("╚══════════════════════════════════════╝");
    println!();

    // Create server and state
    let server = Arc::new(RpcServer::new());
    let state = Arc::new(state::StateStore::new());

    state.set_balance("0xAlice", 10000).await;
    state.set_balance("0xBob", 5000).await;

    server::handlers::register_stateful_handlers(&server, state).await;

    // Setup authentication
    let auth = Arc::new(middleware::AuthMiddleware::new(
        middleware::AuthStrategy::ApiKeyInParams,
    ));

    let api_key = std::env::var("API_KEY").context("set API_KEY before starting the example")?;
    auth.add_key(api_key).await;

    println!("Authentication enabled");
    println!("API key loaded from the API_KEY environment variable");
    println!();

    // Configure server
    let addr = "127.0.0.1:4000";
    let config = transport::tcp::TcpServerConfig::new(addr, server).with_auth(auth);

    println!("Server listening on {} (with authentication)", addr);
    println!();
    println!("Example request with API key:");
    println!(r#"  {{"jsonrpc":"2.0","method":"ping","params":{{"api_key":"<API_KEY>"}},"id":1}}"#);
    println!();
    println!("Without valid API key, requests will be rejected.");
    println!();

    // Run server
    transport::tcp::run_with_framing(config).await?;

    Ok(())
}
