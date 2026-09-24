#!/usr/bin/env bash

set -euo pipefail

echo "Checking formatting..."
cargo fmt --all -- --check

echo "Running Clippy..."
cargo clippy --all-targets --all-features -- -D warnings

echo "Running tests..."
cargo test --all-features

echo "Building examples..."
cargo build --examples --all-features

echo "All DiceRPC checks passed."
