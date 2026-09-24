#![allow(clippy::module_inception)]

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

pub use middleware::{AuthMiddleware, AuthStrategy, AuthenticatedServer};
pub use rpc::{RpcErrorObj, RpcRequest, RpcResponse, RpcServer};
pub use server::metrics::Metrics;
pub use state::{Account, StateStore, Transaction, TransactionStatus};
pub use util::{BatchRequest, BatchResponse};
