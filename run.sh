#!/usr/bin/env bash
set -euo pipefail

if ! command -v rustup >/dev/null 2>&1; then
  echo "rustup is required. Install Rust from https://rustup.rs and rerun ./run.sh"
  exit 1
fi

if ! command -v trunk >/dev/null 2>&1; then
  cargo install trunk
fi

rustup target add wasm32-unknown-unknown
trunk serve --open
