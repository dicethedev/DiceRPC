/// HTTP server demonstrating batch request support
/// 
/// Run with:
/// cargo run --example http_batch_requests --features http

use dice_rpc::*;
use std::sync::Arc;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    server::metrics::init_logging();

    println!("╔══════════════════════════════════════╗");
    println!("║  DiceRPC HTTP Batch Requests         ║");
    println!("╚══════════════════════════════════════╝");
    println!();

    // Create server and state
    let server = Arc::new(RpcServer::new());
    let state = Arc::new(state::StateStore::new());
    
    // Setup demo data
    state.set_balance("0xAlice", 10000).await;
    state.set_balance("0xBob", 5000).await;
    state.set_balance("0xCharlie", 7500).await;

    server::handlers::register_stateful_handlers(&server, state).await;

    let addr = "127.0.0.1:3000";
    println!("Server listening on http://{}", addr);
    println!();
    println!("This server supports JSON-RPC 2.0 batch requests!");
    println!();
    println!("Example batch request:");
    println!(r#"curl -X POST http://127.0.0.1:3000/rpc \"#);
    println!(r#"  -H "Content-Type: application/json" \"#);
    println!(r#"  -d '["#);
    println!(r#"    {{"jsonrpc":"2.0","method":"ping","params":{{}},"id":1}},"#);
    println!(r#"    {{"jsonrpc":"2.0","method":"get_balance","params":{{"address":"0xAlice"}},"id":2}},"#);
    println!(r#"    {{"jsonrpc":"2.0","method":"get_balance","params":{{"address":"0xBob"}},"id":3}},"#);
    println!(r#"    {{"jsonrpc":"2.0","method":"list_accounts","params":{{}},"id":4}}"#);
    println!(r#"  ]'"#);
    println!();
    println!("All requests in the batch are processed concurrently!");
    println!();

    // Run server
    transport::HttpTransport::new(server)
        .serve(addr)
        .await?;

    Ok(())
}