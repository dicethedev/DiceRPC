/// HTTP server with persistent state
/// 
/// Run with:
/// cargo run --example http_with_state --features http

use dice_rpc::*;
use std::sync::Arc;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    server::metrics::init_logging();

    println!("╔══════════════════════════════════════╗");
    println!("║  DiceRPC HTTP Server with State      ║");
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
    println!("0xAlice: 10000");
    println!("0xBob: 5000");
    println!("0xCharlie: 7500");
    println!();

    // Register stateful handlers
    server::handlers::register_stateful_handlers(&server, state).await;

    let addr = "127.0.0.1:3000";
    println!("Server listening on http://{}", addr);
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
    println!();
    println!("Get balance:");
    println!(r#"curl -X POST http://127.0.0.1:3000/rpc \"#);
    println!(r#"  -H "Content-Type: application/json" \"#);
    println!(r#"  -d '{{"jsonrpc":"2.0","method":"get_balance","params":{{"address":"0xAlice"}},"id":1}}'"#);
    println!();
    println!("Transfer:");
    println!(r#"curl -X POST http://127.0.0.1:3000/rpc \"#);
    println!(r#"  -H "Content-Type: application/json" \"#);
    println!(r#"  -d '{{"jsonrpc":"2.0","method":"transfer","params":{{"from":"0xAlice","to":"0xBob","amount":1000}},"id":2}}'"#);
    println!();
    println!("List all accounts:");
    println!(r#"curl -X POST http://127.0.0.1:3000/rpc \"#);
    println!(r#"  -H "Content-Type: application/json" \"#);
    println!(r#"  -d '{{"jsonrpc":"2.0","method":"list_accounts","params":{{}},"id":3}}'"#);
    println!();

    // Run server
    transport::HttpTransport::new(server)
        .serve(addr)
        .await?;

    Ok(())
}