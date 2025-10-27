/// TCP server with persistent state
/// 
/// Run with:
/// cargo run --example tcp_with_state --features tcp

use dice_rpc::*;
use std::sync::Arc;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // Initialize logging
    server::metrics::init_logging();

    println!("╔══════════════════════════════════════╗");
    println!("║  DiceRPC TCP Server with State       ║");
    println!("╚══════════════════════════════════════╝");
    println!();

    // Create server and state
    let server = Arc::new(RpcServer::new());
    let state = Arc::new(state::StateStore::new());

    // Initialize with demo accounts
    state.set_balance("0xAlice", 10000).await;
    state.set_balance("0xBob", 5000).await;
    state.set_balance("0xCharlie", 7500).await;

    println!("Initialized demo accounts:");
    println!("  0xAlice: 10000");
    println!("  0xBob: 5000");
    println!("  0xCharlie: 7500");
    println!();

    // Register stateful handlers
    server::handlers::register_stateful_handlers(&server, state).await;

    let addr = "127.0.0.1:4000";
    println!("Server listening on {}", addr);
    println!();
    println!("Available methods:");
    println!("  - ping");
    println!("  - get_balance");
    println!("  - set_balance");
    println!("  - transfer");
    println!("  - get_transaction");
    println!("  - confirm_transaction");
    println!("  - get_transactions");
    println!("  - list_accounts");
    println!();
    println!("Example commands:");
    println!(r#"  cargo run -- client --method get_balance --params '{{"address":"0xAlice"}}'"#);
    println!(r#"  cargo run -- client --method transfer --params '{{"from":"0xAlice","to":"0xBob","amount":1000}}'"#);
    println!();

    // Run server
    server::server::run(addr).await?;

    Ok(())
}