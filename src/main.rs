mod rpc;
mod server;
mod client;

use clap::{Parser, Subcommand};

#[derive(Parser, Debug)]
#[command(name = "MiniRPC")]
#[command(about = "Small JSON-RPC 2.0 over TCP demo", long_about = None)]
struct Opts {
    #[command(subcommand)]
    cmd: Mode,
}

#[derive(Subcommand, Debug)]
enum Mode {
    /// Run the RPC server
    Server {
        #[arg(short, long, default_value = "127.0.0.1:4000")]
        addr: String,
    },

    /// Run a one-shot client request
    Client {
        #[command(flatten)]
        client: crate::client::ClientArgs,
    },
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let opts = Opts::parse();
    match opts.cmd {
        Mode::Server { addr } => {
            server::run(&addr).await?;
        }
        Mode::Client { client } => {
            client::run_client(client).await?;
        }
    }
    Ok(())
}
