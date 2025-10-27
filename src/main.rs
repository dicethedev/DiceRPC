mod client;
mod macros;
mod middleware;
mod rpc;
mod server;
mod state;
mod transport;
mod util;

use clap::{Parser, Subcommand};
use std::sync::Arc;
use std::time::Duration;

#[derive(Parser, Debug)]
#[command(name = "DiceRPC")]
#[command(about = "JSON-RPC 2.0 server with TCP and HTTP support")]
struct Opts {
    #[command(subcommand)]
    cmd: Mode,
}

/// CLI modes
#[derive(Subcommand, Debug)]
enum Mode {
    /// Run the TCP RPC server (basic line-delimited)
    Server {
        #[arg(short, long, default_value = "127.0.0.1:4000")]
        addr: String,
    },

    /// Run the TCP RPC server with framing and metrics
    #[cfg(feature = "tcp")]
    TcpServer {
        #[arg(short, long, default_value = "127.0.0.1:4000")]
        addr: String,

        /// Enable authentication
        #[arg(long)]
        auth: bool,
    },

    /// Run the HTTP RPC server
    #[cfg(feature = "http")]
    HttpServer {
        #[arg(short, long, default_value = "127.0.0.1:3000")]
        addr: String,

        /// Enable authentication
        #[arg(long)]
        auth: bool,
    },

    /// Run a one-shot client request
    Client {
        #[command(flatten)]
        client: client::ClientArgs,
    },
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // Initialize logging
    server::metrics::init_logging();

    let opts = Opts::parse();
    match opts.cmd {
        Mode::Server { addr } => {
            // Basic TCP server (no metrics, no auth)
            println!("Starting basic TCP server on {}...", addr);
            server::server::run(&addr).await?;
        }

        #[cfg(feature = "tcp")]
        Mode::TcpServer { addr, auth } => {
            run_tcp_server(&addr, auth).await?;
        }

        #[cfg(feature = "http")]
        Mode::HttpServer { addr, auth } => {
            run_http_server(&addr, auth).await?;
        }

        Mode::Client { client } => {
            client::run_client(client).await?;
        }
    }
    Ok(())
}

#[cfg(feature = "tcp")]
async fn run_tcp_server(addr: &str, enable_auth: bool) -> anyhow::Result<()> {
    use crate::middleware::{AuthMiddleware, AuthStrategy};
    use crate::rpc::RpcServer;
    use crate::state::StateStore;
    use crate::transport::tcp::TcpServerConfig;

    // Create components
    let server = Arc::new(RpcServer::new());
    let state = Arc::new(StateStore::new());
    let metrics = Arc::new(server::metrics::Metrics::new());

    // Initialize demo data
    state.set_balance("0xAlice", 100000).await;
    state.set_balance("0xBob", 50000).await;
    state.set_balance("0xCharlie", 75000).await;

    // Register stateful handlers
    server::handlers::register_stateful_handlers(&server, state.clone()).await;

    // Spawn metrics reporter
    let metrics_clone = metrics.clone();
    tokio::spawn(async move {
        loop {
            tokio::time::sleep(Duration::from_secs(30)).await;
            let snapshot = metrics_clone.snapshot().await;
            tracing::info!("Metrics Report");
            tracing::info!("Total Requests: {}", snapshot.total_requests);
            tracing::info!("Successful: {}", snapshot.total_success);
            tracing::info!("Errors: {}", snapshot.total_errors);
            tracing::info!("Avg Duration: {}μs", snapshot.avg_duration_us);
            tracing::info!("Method Counts: {:?}", snapshot.method_counts);
        }
    });

    // Configure TCP server
    let mut config = TcpServerConfig::new(addr, server).with_metrics(metrics);

    // Optionally enable authentication
    if enable_auth {
        let auth = Arc::new(AuthMiddleware::new(AuthStrategy::ApiKeyInParams));
        auth.add_key("dev-key-123").await;
        auth.add_key("prod-key-456").await;
        println!("Authentication enabled. Valid keys: dev-key-123, prod-key-456");
        config = config.with_auth(auth);
    }

    server::metrics::log_startup(addr, "TCP (Framed)");
    println!();
    println!("Features enabled:");
    println!("Length-prefixed framing");
    println!("Metrics collection");
    println!("Persistent state");
    if enable_auth {
        println!("Authentication");
    }
    println!();

    // Run server
    transport::tcp::run_with_framing(config).await?;

    server::metrics::log_shutdown();
    Ok(())
}

#[cfg(feature = "http")]
async fn run_http_server(addr: &str, enable_auth: bool) -> anyhow::Result<()> {
    use crate::middleware::{AuthMiddleware, AuthStrategy};
    use crate::rpc::RpcServer;
    use crate::state::StateStore;
    use crate::transport::HttpTransport;

    // Create components
    let server = Arc::new(RpcServer::new());
    let state = Arc::new(StateStore::new());
    let metrics = Arc::new(server::metrics::Metrics::new());

    // Initialize demo data
    state.set_balance("0xAlice", 100000).await;
    state.set_balance("0xBob", 50000).await;
    state.set_balance("0xCharlie", 75000).await;

    // Register stateful handlers
    server::handlers::register_stateful_handlers(&server, state.clone()).await;

    // Spawn metrics reporter
    let metrics_clone = metrics.clone();
    tokio::spawn(async move {
        loop {
            tokio::time::sleep(Duration::from_secs(30)).await;
            let snapshot = metrics_clone.snapshot().await;
            tracing::info!("Metrics Report");
            tracing::info!("Total Requests: {}", snapshot.total_requests);
            tracing::info!("Successful: {}", snapshot.total_success);
            tracing::info!("Errors: {}", snapshot.total_errors);
            tracing::info!("Avg Duration: {}μs", snapshot.avg_duration_us);
            tracing::info!("Method Counts: {:?}", snapshot.method_counts);
        }
    });

    // Create HTTP transport with metrics
    let mut http = HttpTransport::new(server).with_metrics(metrics);

    // Optionally enable authentication
    if enable_auth {
        let auth = Arc::new(AuthMiddleware::new(AuthStrategy::ApiKeyInParams));
        auth.add_key("dev-key-123").await;
        auth.add_key("prod-key-456").await;
        println!("Authentication enabled. Valid keys: dev-key-123, prod-key-456");
        http = http.with_auth(auth);
    }

    server::metrics::log_startup(addr, "HTTP");
    println!();
    println!("Features enabled:");
    println!("HTTP/REST transport");
    println!("Metrics collection");
    println!("Persistent state");
    println!("Batch request support");
    if enable_auth {
        println!("Authentication");
    }
    println!();
    println!("Endpoints:");
    println!("POST http://{}/", addr);
    println!("POST http://{}/rpc", addr);
    println!("GET  http://{}/metrics", addr);
    println!("GET  http://{}/health", addr);
    println!();
    println!("Example request:");
    println!(r#"curl -X POST http://{}/rpc \"#, addr);
    println!(r#"  -H "Content-Type: application/json" \"#);
    if enable_auth {
        println!(r#"  -d '{{"jsonrpc":"2.0","method":"ping","params":{{"api_key":"dev-key-123"}},"id":1}}'"#);
    } else {
        println!(r#"  -d '{{"jsonrpc":"2.0","method":"ping","params":{{}},"id":1}}'"#);
    }
    println!();

    // Run server
    http.serve(addr).await?;

    server::metrics::log_shutdown();
    Ok(())
}