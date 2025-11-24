#!/bin/bash
#
# SmartType Build Script
# Builds all components: Rust core, Go daemon, Python GUI
#

set -e

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(dirname "$SCRIPT_DIR")"

cd "$PROJECT_ROOT"

echo "================================"
echo " Building SmartType"
echo "================================"
echo ""

# Build Rust core
echo "[1/3] Building Rust core..."
cd rust-core
echo "  - Running cargo build --release..."
cargo build --release
echo "  ✓ Rust core built successfully"
echo ""

# Build Go daemon
cd "$PROJECT_ROOT/go-daemon"
echo "[2/3] Building Go daemon..."
echo "  - Downloading Go dependencies..."
go mod download
echo "  - Building smarttype-daemon..."
go build -o smarttype-daemon \
    -ldflags="-s -w" \
    -trimpath \
    .
echo "  ✓ Go daemon built successfully"
echo ""

# Verify Python GUI dependencies
cd "$PROJECT_ROOT/python-gui"
echo "[3/3] Verifying Python GUI..."
if ! python3 -c "import PyQt5" 2>/dev/null; then
    echo "  - Installing Python dependencies..."
    pip3 install -r requirements.txt
fi
chmod +x main.py
echo "  ✓ Python GUI ready"
echo ""

echo "================================"
echo " Build Summary"
echo "================================"
echo ""
echo "Built artifacts:"
echo "  • rust-core/target/release/smarttype-hook"
echo "  • rust-core/target/release/libsmarttype_core.so"
echo "  • go-daemon/smarttype-daemon"
echo "  • python-gui/main.py"
echo ""
echo "Next steps:"
echo "  Run: sudo ./scripts/install.sh"
echo ""
