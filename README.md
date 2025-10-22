
# DiceRPC — A Lightweight JSON-RPC Framework in Rust

DiceRPC is a minimal yet powerful JSON-RPC 2.0 framework built in Rust.
It allows clients and servers to communicate over HTTP or TCP using a simple request–response model, similar to how Ethereum’s `eth_call`, `eth_sendRawTransaction`, and other RPC methods work.

## Features

- 🧩 Implements JSON-RPC 2.0 — request/response spec with `id`, `method`, and `params`.

- ⚡ Concurrent request handling using `tokio`.

- Custom methods — define and register your own RPC methods like `get_balance` or `send_tx`.

- Serde-powered serialization for safe and fast JSON encoding/decoding.

- CLI client — interact with your RPC server directly from the terminal.

- Extensible architecture — easy to add transport layers (HTTP, WebSocket, or raw TCP).

## Tech Stack

- `Rust`
- `Tokio` for async I/O
- `Serde & serde_json` for data encoding
- `Hyper` or `TcpListener` for transport layer
