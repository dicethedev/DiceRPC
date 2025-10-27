/// TCP server with authentication
/// 
/// Run with:
/// cargo run --example tcp_with_auth --features tcp

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
        middleware::AuthStrategy::ApiKeyInParams
    ));
    
    // Add API keys
    auth.add_key("dev-key-123").await;
    auth.add_key("prod-key-456").await;
    auth.add_key("test-key-789").await;

    println!("Authentication enabled");
    println!("Valid API keys:");
    println!("  - dev-key-123");
    println!("  - prod-key-456");
    println!("  - test-key-789");
    println!();

    // Configure server
    let addr = "127.0.0.1:4000";
    let config = transport::tcp::TcpServerConfig::new(addr, server)
        .with_auth(auth);

    println!("Server listening on {} (with authentication)", addr);
    println!();
    println!("Example request with API key:");
    println!(r#"  {{"jsonrpc":"2.0","method":"ping","params":{{"api_key":"dev-key-123"}},"id":1}}"#);
    println!();
    println!("Without valid API key, requests will be rejected.");
    println!();

    // Run server
    transport::tcp::run_with_framing(config).await?;

    Ok(())
}