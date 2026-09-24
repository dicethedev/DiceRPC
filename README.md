# DiceRPC

An approachable, asynchronous RPC toolkit for Rust, built around JSON-RPC 2.0-style messages.

DiceRPC gives you a small handler registry, HTTP and TCP transports, batch requests, shared state, metrics, and examples you can build on. The project is under active development and welcomes contributors of every experience level.

> **Project status:** DiceRPC is currently `0.1.0` and is best suited to learning, prototypes, and internal experimentation. Its API may change as protocol compliance and production hardening improve. Read [Security](#security) before exposing a server to untrusted networks.

## What you can build

- JSON-RPC services with custom asynchronous methods
- HTTP APIs with Axum
- Newline-delimited or length-prefixed TCP services
- Stateful services backed by the included in-memory store
- Batch request handlers
- CLI clients and integration tests

## Use cases

DiceRPC is useful when you need a small, customizable RPC layer rather than a full web framework:

- **Internal service APIs** — expose typed business operations to trusted services over HTTP or TCP.
- **Blockchain tooling and simulators** — prototype node-style methods, transaction flows, wallets, and test harnesses.
- **Developer tools** — control local processes, emulators, indexers, or background workers through a simple RPC interface.
- **Protocol experiments** — compare HTTP, newline-delimited TCP, and framed TCP behavior around the same method registry.
- **Teaching and testing** — demonstrate async handlers, middleware, shared state, batch calls, metrics, and graceful shutdown.

DiceRPC is not yet intended to replace a hardened public API gateway. Put internet-facing deployments behind TLS, authentication, authorization, and edge-level abuse controls.

## Features

- Async method registration and concurrent execution with Tokio
- HTTP endpoints at `/` and `/rpc`
- TCP transport with legacy newline-delimited and length-prefixed modes
- Single and batch request processing
- API-key middleware for HTTP headers, request parameters, and framed TCP requests
- Configurable body/frame, batch, connection/concurrency, and timeout limits
- Thread-safe in-memory account and transaction state
- Request counts, errors, latency metrics, and tracing
- Health and metrics endpoints for metrics-enabled HTTP servers
- Graceful shutdown support for the framed TCP server
- CLI server and client commands
- Examples and integration tests for the main workflows

## Requirements

- [Rust](https://www.rust-lang.org/tools/install) 1.85 or newer (the crate uses Rust 2024 edition)
- Cargo
- Git, if you are cloning the repository

## Quick start

Clone and build the project:

```bash
git clone https://github.com/dicethedev/DiceRPC.git
cd DiceRPC
cargo build
```

### Try the TCP server

Start the basic newline-delimited TCP server:

```bash
cargo run -- server
```

In another terminal, call its `ping` method:

```bash
cargo run -- client --method ping
```

Expected output:

```text
Response: {"jsonrpc":"2.0","result":"pong","error":null,"id":1}
```

You can also pass JSON parameters:

```bash
cargo run -- client \
  --method get_balance \
  --params '{"address":"0x123abc"}'
```

### Try the HTTP server

Start the CLI HTTP server:

```bash
cargo run -- http-server
```

Then send a request from another terminal:

```bash
curl --request POST http://127.0.0.1:3000/rpc \
  --header 'Content-Type: application/json' \
  --data '{"jsonrpc":"2.0","method":"ping","params":{},"id":1}'
```

The CLI HTTP server includes stateful demonstration methods and exposes:

- `POST /` and `POST /rpc` for RPC requests
- `GET /health` for a basic health response
- `GET /metrics` for an in-memory metrics snapshot

Run `cargo run -- --help` to see all CLI commands and options.

## Register your own method

Create an `RpcServer`, register an async handler, and attach a transport:

```rust
use dice_rpc::{RpcErrorObj, RpcServer};
use dice_rpc::transport::HttpTransport;
use serde_json::{json, Value};
use std::sync::Arc;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let server = Arc::new(RpcServer::new());

    server
        .register("greet", |params: Value| async move {
            let name = params
                .get("name")
                .and_then(Value::as_str)
                .ok_or_else(|| RpcErrorObj {
                    code: -32602,
                    message: "Missing or invalid 'name' parameter".into(),
                    data: None,
                })?;

            Ok(json!({ "message": format!("Hello, {name}!") }))
        })
        .await;

    HttpTransport::new(server)
        .serve("127.0.0.1:3000")
        .await
}
```

Call the method with:

```bash
curl --request POST http://127.0.0.1:3000/rpc \
  --header 'Content-Type: application/json' \
  --data '{"jsonrpc":"2.0","method":"greet","params":{"name":"Dice"},"id":1}'
```

Handlers receive a `serde_json::Value` and return `Result<Value, RpcErrorObj>`. This keeps the core API small while allowing every method to validate or deserialize its own parameter type.

## Request and response format

A typical request looks like this:

```json
{
  "jsonrpc": "2.0",
  "method": "get_balance",
  "params": { "address": "0xAlice" },
  "id": 1
}
```

A successful response contains `result`:

```json
{
  "jsonrpc": "2.0",
  "result": { "address": "0xAlice", "balance": "100000" },
  "id": 1
}
```

An unsuccessful response contains an error object instead. DiceRPC follows the main JSON-RPC request/response shape, but full specification compliance—including notification behavior and every standard error case—is still a work in progress.

## Batch requests

The HTTP and framed TCP paths accept multiple requests in one JSON array:

```bash
curl --request POST http://127.0.0.1:3000/rpc \
  --header 'Content-Type: application/json' \
  --data '[
    {"jsonrpc":"2.0","method":"ping","params":{},"id":1},
    {"jsonrpc":"2.0","method":"get_balance","params":{"address":"0xAlice"},"id":2}
  ]'
```

Batch entries are processed concurrently. DiceRPC applies a default batch limit of 100; tune this and the concurrency limits for your workload before accepting untrusted traffic.

## Resource limits

Network-facing transports have conservative defaults and builder methods for tuning them:

| Transport | Default limits | Builder methods |
| --- | --- | --- |
| HTTP | 1 MiB body, 100 batch entries, 256 concurrent requests, 30-second timeout | `with_max_body_size`, `with_max_batch_size`, `with_max_concurrency`, `with_request_timeout` |
| Framed TCP | 1 MiB frame, 100 batch entries, 1,024 connections, 30-second timeout | `with_max_frame_size`, `with_max_batch_size`, `with_max_connections`, `with_request_timeout` |

These are application-level safeguards, not a replacement for proxy, firewall, and operating-system limits.

## Authentication

For HTTP, header authentication keeps credentials out of the JSON-RPC parameters:

```bash
export API_KEYS='replace-with-a-long-random-secret'
cargo run -- http-server --auth

curl --request POST http://127.0.0.1:3000/rpc \
  --header 'Content-Type: application/json' \
  --header 'x-api-key: replace-with-a-long-random-secret' \
  --data '{"jsonrpc":"2.0","method":"ping","params":{},"id":1}'
```

Framed TCP uses `ApiKeyInParams`, so each request includes `"api_key"` in its `params` object. `ApiKeyInHeader` is available only to HTTP because TCP requests do not contain HTTP headers.

API keys are compared in constant time. Authentication does not provide per-method authorization, TLS, identity roles, or key storage; applications must add those controls for their environment.

## Included demonstration methods

Two groups of handlers are included for learning and testing:

| Handler set | Methods |
| --- | --- |
| Basic | `ping`, `get_balance`, `send_tx` |
| Stateful | `ping`, `get_balance`, `set_balance`, `transfer`, `get_transaction`, `confirm_transaction`, `get_transactions`, `list_accounts` |

These methods contain demonstration logic, not a blockchain implementation. In particular, balances and transactions live only in memory and disappear when the process stops.

## Feature flags

| Feature | Default | Purpose |
| --- | --- | --- |
| `tcp` | Yes | TCP transport and framing |
| `http` | Yes | Axum HTTP transport and endpoints |
| `full` | No | Explicitly enables both transports |

Useful build commands:

```bash
# Default build: HTTP and TCP
cargo build

# TCP only
cargo build --no-default-features --features tcp

# HTTP only
cargo build --no-default-features --features http

# All supported transports
cargo build --features full
```

## Examples

The [`examples`](examples) directory is the fastest way to explore individual features:

| Example | Purpose |
| --- | --- |
| `http_basic` | Minimal HTTP server |
| `http_client` | HTTP client requests |
| `http_batch_requests` | HTTP batch calls |
| `http_with_auth` | Parameter-based API keys over HTTP |
| `http_with_header_auth` | Recommended `x-api-key` HTTP authentication |
| `http_with_no_auth` | Explicit unauthenticated HTTP setup |
| `http_with_state` | Shared application state over HTTP |
| `http_full_featured` | HTTP features used together |
| `production_http` | Environment-driven auth and resource limits |
| `tcp_basic` | Minimal TCP server |
| `tcp_framed` | Length-prefixed TCP messages |
| `tcp_with_auth` | Parameter-based authentication over framed TCP |
| `tcp_with_state` | Shared application state over TCP |
| `tcp_client_advanced` | Advanced TCP client usage |
| `tcp_full_featured` | TCP features used together |

Run one with:

```bash
cargo run --example http_basic
```

Some examples start a server and keep running until you stop them with `Ctrl+C`.

For a longer architecture walkthrough, see the [DiceRPC implementation guide](https://hackmd.io/AJz1P0gISx6W0TEewLRJ3w?view).

## Project structure

```text
src/
├── client/       # CLI client
├── middleware/   # Authentication middleware
├── rpc/          # Request, response, and handler registry
├── server/       # Server helpers, stateful handlers, and metrics
├── transport/    # HTTP, TCP, framing, metrics endpoints, and shutdown
├── util/         # Batch request handling
├── lib.rs        # Public library exports
├── main.rs       # CLI entry point
├── macros.rs     # Helper macros
└── state.rs      # In-memory accounts and transactions
```

## Testing and quality checks

Run the same checks before submitting a pull request:

```bash
cargo fmt --all -- --check
cargo clippy --all-targets --all-features
cargo test --all-features
cargo build --examples --all-features
```

When fixing a bug, add a regression test that fails without the fix. When adding a public feature, include an example or update this README so people can discover it.

## Contributing

Contributions are welcome—documentation fixes and focused beginner changes are just as valuable as large features. See [CONTRIBUTING.md](CONTRIBUTING.md) for development setup, quality checks, commit guidance, pull-request expectations, and licensing terms.

For significant features, security-sensitive work, or breaking API changes, please open an issue before implementation so the approach can be discussed.

## Security

DiceRPC is experimental and has not received an independent security audit. Read [SECURITY.md](SECURITY.md) for supported versions, known limitations, safer deployment guidance, and private vulnerability-reporting instructions.

Please report suspected vulnerabilities through [GitHub Security Advisories](https://github.com/dicethedev/DiceRPC/security/advisories/new), not a public issue.

## Roadmap

The roadmap is intentionally flexible while the core API matures:

- [ ] Complete JSON-RPC 2.0 compliance tests
- [x] Enforce authentication consistently across transports
- [x] Add header-based HTTP authentication
- [x] Add configurable request, batch, connection, and timeout limits
- [ ] Add per-method authorization hooks
- [ ] Add TLS and reverse-proxy deployment documentation
- [ ] Add persistent state adapters
- [ ] Add WebSocket transport
- [ ] Add Prometheus-compatible metrics
- [ ] Add fuzzing, benchmarks, and load tests
- [ ] Publish versioned API documentation and migration notes

Have another idea? Open an issue and describe the problem it solves.

## License

DiceRPC is dual-licensed under your choice of:

- the [MIT License](LICENSE-MIT); or
- the [Apache License, Version 2.0](LICENSE-APACHE).

You may use, modify, and distribute the project under either license. See the license files for the complete terms.

## Resources

- [JSON-RPC 2.0 specification](https://www.jsonrpc.org/specification)
- [Tokio documentation](https://tokio.rs/)
- [Axum documentation](https://docs.rs/axum/)
- [Serde documentation](https://serde.rs/)
- [Rust API Guidelines](https://rust-lang.github.io/api-guidelines/)

---

Built by [dicethedev](https://github.com/dicethedev). If DiceRPC helps you learn or build something useful, consider starring the repository or contributing an improvement.
