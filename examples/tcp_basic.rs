/// Basic TCP server example
/// 
/// Run with:
/// cargo run --example tcp_basic

use dice_rpc::*;
use std::sync::Arc;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    println!("╔══════════════════════════════════════╗");
    println!("║     DiceRPC Basic TCP Server         ║");
    println!("╚══════════════════════════════════════╝");
    println!();

    // Create server
    let server = Arc::new(RpcServer::new());
    
    // Register default handlers
    rpc::register_default_handlers(&server).await;

    let addr = "127.0.0.1:4000";
    println!("Server listening on {}", addr);
    println!();
    println!("Test with client:");
    println!("  cargo run -- client --method ping");
    println!("  cargo run -- client --method get_balance --params '{{\"address\":\"0x123\"}}'");
    println!();

    // Run server
    server::server::run(addr).await?;

    Ok(())
}