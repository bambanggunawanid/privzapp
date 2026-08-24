#!/usr/bin/env bash
# The full verification battery. Run before every commit; CI runs the same.
set -euo pipefail
cd "$(dirname "$0")/.."

echo "==> cargo fmt --check"
cargo fmt --all -- --check

echo "==> cargo clippy (workspace, all targets, warnings are errors)"
cargo clippy --workspace --all-targets -- -D warnings

echo "==> cargo test (workspace)"
cargo test --workspace --quiet

echo "==> wasm32 check (the app must always compile for the web)"
cargo check -p privzapp --target wasm32-unknown-unknown --quiet

echo "All green ✔"
