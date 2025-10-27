/// TCP server with length-prefixed framing
/// 
/// Run with:
/// cargo run --example tcp_framed --features tcp

use dice_rpc::*;
use std::sync::Arc;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // Initialize logging
    server::metrics::init_logging();

    println!("╔══════════════════════════════════════╗");
    println!("║  DiceRPC TCP Server (Framed)         ║");
    println!("╚══════════════════════════════════════╝");
    println!();

    // Create components
    let server = Arc::new(RpcServer::new());
    let state = Arc::new(state::StateStore::new());
    let metrics = Arc::new(dice_rpc::Metrics::new());

    // Setup demo data
    state.set_balance("0xAlice", 10000).await;
    state.set_balance("0xBob", 5000).await;

    // Register handlers
    server::handlers::register_stateful_handlers(&server, state).await;

    // Configure TCP server
    let addr = "127.0.0.1:4000";
    let config = transport::tcp::TcpServerConfig::new(addr, server)
        .with_metrics(metrics.clone());

    println!("Server listening on {} (framed protocol)", addr);
    println!();
    println!("This server uses length-prefixed framing instead of newlines.");
    println!("It's more robust for binary data and high-performance scenarios.");
    println!();
    println!("Press Ctrl+C to shutdown gracefully");
    println!();

    // Spawn metrics reporter
    tokio::spawn(async move {
        loop {
            tokio::time::sleep(tokio::time::Duration::from_secs(30)).await;
            let snapshot = metrics.snapshot().await;
            tracing::info!("Metrics: {:?}", snapshot);
        }
    });

    // Run server
    transport::tcp::run_with_framing(config).await?;

    Ok(())
}