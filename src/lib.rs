// Core RPC functionality
pub mod rpc;

// State management
pub mod state;

// Transport layer
pub mod transport;

// Utilities
pub mod util;

// Middleware
pub mod middleware;

// Server implementations
#[cfg(feature = "http")]
pub mod server;

// Client
pub mod client;

// Macros
pub mod macros;
pub use macros::*;

#[cfg(not(feature = "http"))]
pub fn no_http_warning() {
    println!("HTTP support disabled");
}

pub use rpc::{RpcServer, RpcRequest, RpcResponse, RpcErrorObj};
pub use state::{StateStore, Transaction, TransactionStatus, Account};
pub use util::{BatchRequest, BatchResponse};
pub use middleware::{AuthMiddleware, AuthStrategy, AuthenticatedServer};
pub use server::metrics::Metrics;