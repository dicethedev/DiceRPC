use std::time::Duration;
use tokio::time::sleep;
use dice_rpc::client::ClientArgs; 
use tokio::task;
use tokio::io::AsyncWriteExt;
use tokio::io::AsyncBufReadExt;

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn test_ping() {
    // spawn server in background
    let addr = "127.0.0.1:5001";
    task::spawn(async move {
        dice_rpc::server::run(addr).await.unwrap();
    });

    // small sleep to let server bind
    sleep(Duration::from_millis(200)).await;

    let client = ClientArgs {
        addr: addr.to_string(),
        method: "ping".to_string(),
        params: "{}".to_string(),
    };

    // run client - it should print response; we'll just try to connect and read result
    let mut stream = tokio::net::TcpStream::connect(&client.addr).await.unwrap();
    let req = serde_json::json!({
        "jsonrpc": "2.0",
        "method": client.method,
        "params": {},
        "id": 1
    });
    let req_text = serde_json::to_string(&req).unwrap() + "\n";
    stream.write_all(req_text.as_bytes()).await.unwrap();

    let mut reader = tokio::io::BufReader::new(stream);
    let mut line = String::new();
    reader.read_line(&mut line).await.unwrap();
    let resp: serde_json::Value = serde_json::from_str(&line).unwrap();
    assert_eq!(resp["result"], "pong");
}