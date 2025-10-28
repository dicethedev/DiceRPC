# DiceRPC — A Lightweight JSON-RPC 2.0 Framework in Rust

[![Rust](https://img.shields.io/badge/rust-1.70%2B-orange.svg)](https://www.rust-lang.org/)
[![License](https://img.shields.io/badge/license-MIT-blue.svg)](LICENSE)
[![Build Status](https://img.shields.io/badge/build-passing-brightgreen.svg)]()

DiceRPC is a **minimal yet powerful** JSON-RPC 2.0 framework built in Rust. It allows clients and servers to communicate over HTTP or TCP using a simple request-response model, similar to how Ethereum's `eth_call`, `eth_sendRawTransaction`, and other RPC methods work.

> 📖 **[Read the Complete Guide](https://hackmd.io/AJz1P0gISx6W0TEewLRJ3w?view)** — Detailed architecture, implementation walkthrough, and design decisions explained.

---

## Table of Contents

- [Features](#-features)
- [Tech Stack](#-tech-stack)
- [Getting Started](#-getting-started)
- [Usage Examples](#-usage-examples)
- [Available Handlers](#-available-handlers)
- [Architecture](#-architecture)
- [Feature Flags](#-feature-flags)
- [Testing](#-testing)
- [Production Features](#-production-features)
- [Roadmap](#-roadmap)
- [Contributing](#-contributing)
- [Resources](#-resources)
- [License](#-license)

---

## Features

- **JSON-RPC 2.0 compliant** — Full specification with `id`, `method`, `params`, and `error` handling
- **Multi-transport support** — HTTP (Axum) and TCP with length-prefixed framing
- **Built-in authentication** — API key validation with pluggable strategies
- **Batch request processing** — Handle multiple requests concurrently
- **Persistent state** — In-memory store for accounts and transactions
- **Metrics & observability** — Request tracking, tracing, and performance monitoring
- **Graceful shutdown** — Signal handling (SIGTERM/SIGINT) with proper cleanup
- **Custom handlers** — Easy registration of your own RPC methods
- **CLI client included** — Test your server directly from the terminal
- **Extensible architecture** — Modular design for easy customization

---

## Tech Stack

- **Rust 1.70+**
- **Tokio** — Async runtime for concurrent request handling
- **Serde & serde_json** — Fast and safe JSON serialization
- **Axum** — Modern HTTP framework (optional with `http` feature)
- **Futures** — Concurrent batch request processing
- **Tracing** — Structured logging and diagnostics
- **Anyhow** — Flexible error handling

---

## Getting Started

### Prerequisites

- Rust 1.70 or higher ([Install from rustup.rs](https://rustup.rs))
- Git

### Installation

```bash
# Clone the repository
git clone https://github.com/dicethedev/DiceRPC.git
cd DiceRPC

# Build with TCP support (default)
cargo build --release

# Or build with all features (TCP + HTTP)
cargo build --release --features full
```

### Quick Start - TCP Server

**Start the server:**

```bash
# Start TCP server on default port 4000
cargo run --release -- server

# Or specify a custom address
cargo run --release -- server --addr 127.0.0.1:5000
```

**Make requests from another terminal:**

```bash
# Simple ping
cargo run --release -- client --method ping

# Get balance
cargo run --release -- client \
  --method get_balance \
  --params '{"address":"0x123abc"}'

# Send transaction
cargo run --release -- client \
  --method send_tx \
  --params '{"raw_tx":"0xdeadbeef"}'
```

### Quick Start - HTTP Server

Build with HTTP support and run:

```bash
# Build with HTTP feature
cargo build --release --features http

# Run HTTP server (requires modifying main.rs or using examples)
TRANSPORT=http cargo run --release
```

**Test with curl:**

```bash
curl -X POST http://localhost:3000/rpc \
  -H "Content-Type: application/json" \
  -d '{
    "jsonrpc": "2.0",
    "method": "ping",
    "params": {},
    "id": 1
  }'
```

---

## Usage Examples

### Example 1: Basic TCP Server

```rust
use dice_rpc::*;
use std::sync::Arc;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // Create RPC server
    let server = Arc::new(RpcServer::new());
    
    // Register default handlers (ping, get_balance, send_tx)
    register_default_handlers(&server).await;
    
    // Configure and start TCP server
    let config = TcpServerConfig::new("127.0.0.1:4000", server);
    run_with_framing(config).await?;
    
    Ok(())
}
```

### Example 2: HTTP Server with Authentication

```rust
use dice_rpc::*;
use std::sync::Arc;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let server = Arc::new(RpcServer::new());
    register_default_handlers(&server).await;
    
    // Setup authentication middleware
    let auth = Arc::new(AuthMiddleware::new(AuthStrategy::ApiKeyInParams));
    auth.add_key("my-secret-key-123").await;
    auth.add_key("another-key-456").await;
    
    // Start HTTP server with authentication
    HttpTransport::new(server)
        .with_auth(auth)
        .serve("127.0.0.1:3000")
        .await?;
    
    Ok(())
}
```

**Test authenticated request:**

```bash
curl -X POST http://localhost:3000/rpc \
  -H "Content-Type: application/json" \
  -d '{
    "jsonrpc": "2.0",
    "method": "ping",
    "params": {"api_key": "my-secret-key-123"},
    "id": 1
  }'
```

### Example 3: Custom Handler with State

```rust
use dice_rpc::*;
use serde_json::json;
use std::sync::Arc;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let server = Arc::new(RpcServer::new());
    let state = Arc::new(StateStore::new());
    
    // Register a custom handler
    let state_clone = state.clone();
    server.register("get_user", move |params| {
        let state = state_clone.clone();
        async move {
            let user_id = params["user_id"]
                .as_str()
                .ok_or_else(|| RpcErrorObj {
                    code: INVALID_PARAMS,
                    message: "Missing user_id parameter".into(),
                    data: None,
                })?;
            
            // Your business logic here
            Ok(json!({
                "user_id": user_id,
                "name": "Alice",
                "balance": 1000
            }))
        }
    }).await;
    
    // Start server
    let config = TcpServerConfig::new("127.0.0.1:4000", server);
    run_with_framing(config).await?;
    
    Ok(())
}
```

### Example 4: Batch Requests

**Send multiple requests at once:**

```bash
# Using curl with HTTP transport
curl -X POST http://localhost:3000/rpc \
  -H "Content-Type: application/json" \
  -d '[
    {
      "jsonrpc": "2.0",
      "method": "ping",
      "params": {},
      "id": 1
    },
    {
      "jsonrpc": "2.0",
      "method": "get_balance",
      "params": {"address": "0xAlice"},
      "id": 2
    },
    {
      "jsonrpc": "2.0",
      "method": "get_balance",
      "params": {"address": "0xBob"},
      "id": 3
    }
  ]'
```

### Example 5: Full-Featured Server

```rust
use dice_rpc::*;
use std::sync::Arc;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // Initialize logging
    metrics::init_logging();
    
    // Create components
    let server = Arc::new(RpcServer::new());
    let state = Arc::new(StateStore::new());
    let metrics = Arc::new(Metrics::new());
    
    // Register stateful handlers
    handlers::register_stateful_handlers(&server, state.clone()).await;
    
    // Setup authentication
    let auth = Arc::new(AuthMiddleware::new(AuthStrategy::ApiKeyInParams));
    auth.add_key("dev-key-12345").await;
    
    // Start TCP server with all features
    let config = TcpServerConfig::new("127.0.0.1:4000", server)
        .with_auth(auth)
        .with_metrics(metrics);
    
    run_with_framing(config).await?;
    
    Ok(())
}
```

---

## Available Handlers

DiceRPC comes with built-in handlers for common operations:

| Method | Description | Parameters | Example |
|--------|-------------|------------|---------|
| `ping` | Health check | None | `{"method": "ping", "params": {}}` |
| `get_balance` | Get account balance | `{"address": "0x..."}` | `{"method": "get_balance", "params": {"address": "0xAlice"}}` |
| `set_balance` | Set account balance (admin) | `{"address": "0x...", "balance": 1000}` | `{"method": "set_balance", "params": {"address": "0xAlice", "balance": 5000}}` |
| `send_tx` | Submit transaction | `{"raw_tx": "0x..."}` | `{"method": "send_tx", "params": {"raw_tx": "0xdeadbeef"}}` |
| `transfer` | Transfer funds | `{"from": "0x...", "to": "0x...", "amount": 100}` | `{"method": "transfer", "params": {"from": "0xAlice", "to": "0xBob", "amount": 500}}` |
| `get_transaction` | Get transaction by ID | `{"txid": "uuid"}` | `{"method": "get_transaction", "params": {"txid": "abc-123"}}` |
| `confirm_transaction` | Confirm pending tx | `{"txid": "uuid"}` | `{"method": "confirm_transaction", "params": {"txid": "abc-123"}}` |
| `get_transactions` | Get txs for address | `{"address": "0x..."}` | `{"method": "get_transactions", "params": {"address": "0xAlice"}}` |
| `list_accounts` | List all accounts | None | `{"method": "list_accounts", "params": {}}` |

---

## Architecture

DiceRPC uses a modular, layered architecture:

```
src/
├── rpc.rs              # Core RPC server and handler registry
├── state.rs            # In-memory state store (accounts & transactions)
├── transport/          # Transport layer
│   ├── tcp.rs          # TCP with length-prefixed framing
│   ├── http_transport.rs  # HTTP via Axum
│   ├── framing.rs      # Binary framing protocol
│   └── shutdown.rs     # Graceful shutdown coordinator
├── middleware/         # Middleware layer
│   └── auth.rs         # Authentication strategies
├── server/             # Server implementations
│   ├── handlers.rs     # Business logic handlers
│   ├── metrics.rs      # Request metrics & tracing
│   └── server.rs       # Basic TCP server
├── util/               # Utilities
│   └── batch.rs        # Batch request handling
├── client/             # CLI client
│   └── client.rs       # Command-line client
└── macros.rs           # Helper macros
```

### Key Design Patterns

- **Handler Registry Pattern** — Dynamic method registration
- **Middleware Chain** — Authentication, metrics, etc.
- **Transport Abstraction** — Easy to add new protocols
- **State Management** — Thread-safe with `Arc<RwLock<>>`
- **Graceful Shutdown** — Proper cleanup on signals

> For detailed architecture diagrams and flow charts, see the [complete guide](https://hackmd.io/AJz1P0gISx6W0TEewLRJ3w?view#DiceRPC-Architecture-Overview).

---

## Feature Flags

Build with specific features to minimize binary size:

```bash
# TCP only (default, ~2MB binary)
cargo build --release

# HTTP only
cargo build --release --no-default-features --features http

# Both transports
cargo build --release --features full
```

**Available features:**
- `tcp` — TCP transport with framing (default)
- `http` — HTTP transport with Axum
- `full` — All features enabled

---

## Testing

Run the comprehensive test suite:

```bash
# Run all tests
cargo test

# Run with output
cargo test -- --nocapture

# Test specific module
cargo test rpc::tests
cargo test batch::tests
cargo test state::tests

# Run integration tests
cargo test --test '*'
```

**Test coverage includes:**
- RPC request/response parsing
- Batch processing
- Authentication flows
- State management (transfers, confirmations)
- Metrics collection
- Graceful shutdown

---

## Production Features

DiceRPC is production-ready with:

- **Graceful shutdown** — Signal handling (SIGTERM, SIGINT, Ctrl+C)
- **Request metrics** — Track requests, errors, latency
- **Structured logging** — Tracing with `tracing` crate
- **Authentication** — Pluggable API key validation
- **Error handling** — Proper error codes per JSON-RPC spec
- **State persistence** — Ready for database integration
- **Concurrent processing** — Tokio-based async/await
- **Batch support** — Process multiple requests in parallel
- **Binary protocol** — Length-prefixed framing for TCP

---

## My Roadmap

Future enhancements planned:

- [ ] WebSocket transport
- [ ] Database persistence (PostgreSQL, Redis)
- [ ] Rate limiting middleware
- [ ] Request/response compression (gzip, brotli)
- [ ] TLS/SSL support
- [ ] Prometheus metrics exporter
- [ ] OpenAPI/Swagger documentation
- [ ] Client libraries (JavaScript, Python)
- [ ] Load balancing support
- [ ] Circuit breaker pattern

---

## Contributing

Contributions are welcome! Here's how you can help:

1. **Fork the repository**
2. **Create a feature branch** (`git checkout -b feature/amazing-feature`)
3. **Commit your changes** (`git commit -m 'Add amazing feature'`)
4. **Push to the branch** (`git push origin feature/amazing-feature`)
5. **Open a Pull Request**

Please ensure:
- Code follows Rust conventions (`cargo fmt`, `cargo clippy`)
- Tests pass (`cargo test`)
- New features include tests and documentation

---

## Resources

- **[Complete Implementation Guide](https://hackmd.io/AJz1P0gISx6W0TEewLRJ3w?view)** — Deep dive into DiceRPC
- [JSON-RPC 2.0 Specification](https://www.jsonrpc.org/specification)
- [Rust Async Book](https://rust-lang.github.io/async-book/)
- [Tokio Documentation](https://tokio.rs/)
- [Axum Framework](https://docs.rs/axum/latest/axum/)

---

## License

This project is licensed under the MIT License - see the [LICENSE](LICENSE) file for details.

---

## Acknowledgments

Built with ❤️ using Rust and the amazing ecosystem:
- [Tokio](https://tokio.rs/) — Async runtime
- [Serde](https://serde.rs/) — Serialization
- [Axum](https://github.com/tokio-rs/axum) — HTTP framework
- [Tracing](https://tracing.rs/) — Structured logging

---

<div align="center">

**⭐ Star this repo if you find it useful!**

Made by [dicethedev] | [GitHub](https://github.com/dicethedev) | [Twitter](https://twitter.com/dicethedev)

</div>
