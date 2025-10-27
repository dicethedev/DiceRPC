/// HTTP client example
/// 
/// Run with:
/// cargo run --example http_client --features http

use serde_json::json;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let client = reqwest::Client::new();
    let url = "http://127.0.0.1:3000/rpc";

    println!("╔══════════════════════════════════════╗");
    println!("║     DiceRPC HTTP Client              ║");
    println!("╚══════════════════════════════════════╝");
    println!();
    println!("Connecting to {}...", url);
    println!();

    // Example 1: Ping
    println!("Testing ping...");
    let ping_req = json!({
        "jsonrpc": "2.0",
        "method": "ping",
        "params": {"api_key": "dev-secret-key"},
        "id": 1
    });

    let response = client
        .post(url)
        .json(&ping_req)
        .send()
        .await?;

    let result: serde_json::Value = response.json().await?;
    println!("Response: {}", serde_json::to_string_pretty(&result)?);
    println!();

    // Example 2: Get balance
    println!("Getting balance for 0xAlice...");
    let balance_req = json!({
        "jsonrpc": "2.0",
        "method": "get_balance",
        "params": {
            "address": "0xAlice",
            "api_key": "dev-secret-key"
        },
        "id": 2
    });

    let response = client
        .post(url)
        .json(&balance_req)
        .send()
        .await?;

    let result: serde_json::Value = response.json().await?;
    println!("Response: {}", serde_json::to_string_pretty(&result)?);
    println!();

    // Example 3: Transfer
    println!("Transferring 1000 from Alice to Bob...");
    let transfer_req = json!({
        "jsonrpc": "2.0",
        "method": "transfer",
        "params": {
            "from": "0xAlice",
            "to": "0xBob",
            "amount": 1000,
            "api_key": "dev-secret-key"
        },
        "id": 3
    });

    let response = client
        .post(url)
        .json(&transfer_req)
        .send()
        .await?;

    let result: serde_json::Value = response.json().await?;
    println!("Response: {}", serde_json::to_string_pretty(&result)?);
    println!();

    // Example 4: Batch request
    println!("Sending batch request...");
    let batch_req = json!([
        {
            "jsonrpc": "2.0",
            "method": "get_balance",
            "params": {"address": "0xAlice", "api_key": "dev-secret-key"},
            "id": 4
        },
        {
            "jsonrpc": "2.0",
            "method": "get_balance",
            "params": {"address": "0xBob", "api_key": "dev-secret-key"},
            "id": 5
        },
        {
            "jsonrpc": "2.0",
            "method": "list_accounts",
            "params": {"api_key": "dev-secret-key"},
            "id": 6
        }
    ]);

    let response = client
        .post(url)
        .json(&batch_req)
        .send()
        .await?;

    let result: serde_json::Value = response.json().await?;
    println!("Response: {}", serde_json::to_string_pretty(&result)?);
    println!();

    println!("All tests completed!");

    Ok(())
}