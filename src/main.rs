use anyhow::{Context, bail};
use clap::{Parser, Subcommand};
use dice_rpc::{RpcServer, client, middleware, server, state, transport};
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
    use middleware::AuthStrategy;
    use state::StateStore;
    use transport::tcp::TcpServerConfig;

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
        let auth = load_auth_from_env(AuthStrategy::ApiKeyInParams).await?;
        println!("Authentication enabled with keys loaded from API_KEYS");
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
    use middleware::AuthStrategy;
    use state::StateStore;
    use transport::HttpTransport;

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
        let auth = load_auth_from_env(AuthStrategy::ApiKeyInHeader).await?;
        println!("Authentication enabled with x-api-key and keys loaded from API_KEYS");
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
        println!(r#"  -H "x-api-key: $DICERPC_API_KEY" \"#);
        println!(r#"  -d '{{"jsonrpc":"2.0","method":"ping","params":{{}},"id":1}}'"#);
    } else {
        println!(r#"  -d '{{"jsonrpc":"2.0","method":"ping","params":{{}},"id":1}}'"#);
    }
    println!();

    // Run server
    http.serve(addr).await?;

    server::metrics::log_shutdown();
    Ok(())
}

async fn load_auth_from_env(
    strategy: middleware::AuthStrategy,
) -> anyhow::Result<Arc<middleware::AuthMiddleware>> {
    let raw_keys = std::env::var("API_KEYS")
        .context("authentication requires API_KEYS as a comma-separated list")?;
    let auth = Arc::new(middleware::AuthMiddleware::new(strategy));
    let mut added = 0usize;
    for key in raw_keys
        .split(',')
        .map(str::trim)
        .filter(|key| !key.is_empty())
    {
        added += usize::from(auth.add_key(key).await);
    }
    if added == 0 {
        bail!("API_KEYS must contain at least one non-empty key");
    }
    Ok(auth)
}
