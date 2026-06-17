#!/usr/bin/env bash
set -euo pipefail

REPO="$(cd "$(dirname "$0")/.." && pwd)"

echo "==> Building Rust binaries (release)..."
cd "$REPO/rust-core"
cargo build --release

echo ""
echo "==> Building Go daemon..."
cd "$REPO/go-daemon"
go mod download
go build -o smarttype-daemon -ldflags="-s -w" -trimpath .

echo ""
echo "Build complete."
echo "  $REPO/rust-core/target/release/smarttype-hook"
echo "  $REPO/rust-core/target/release/smarttype-popup"
echo "  $REPO/go-daemon/smarttype-daemon"
