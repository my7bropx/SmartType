#!/usr/bin/env bash
# SmartType — install script
# Checks dependencies, builds everything, and installs to /usr/local/bin.
set -euo pipefail

REPO="$(cd "$(dirname "$0")/.." && pwd)"
BIN="/usr/local/bin"
IBUS_COMPONENT_DIR="/usr/share/ibus/component"

# ── 1. Build dependencies ─────────────────────────────────────────────────────

echo "==> Checking build dependencies..."

# Rust
if command -v cargo &>/dev/null; then
    echo "  Rust $(rustc --version) — ok"
else
    echo "  Installing Rust via rustup..."
    curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y --default-toolchain stable
    # shellcheck source=/dev/null
    source "$HOME/.cargo/env"
fi

# Go
if command -v go &>/dev/null; then
    echo "  Go $(go version) — ok"
else
    echo "  ERROR: Go not found. Install it first:"
    echo "    sudo apt install golang-go   # Debian/Ubuntu/Kali"
    echo "    https://go.dev/dl/           # upstream"
    exit 1
fi

# System packages (Debian/Ubuntu/Kali)
if command -v apt-get &>/dev/null; then
    echo "  Installing system packages..."
    sudo apt-get update -qq
    sudo apt-get install -y --no-install-recommends \
        build-essential \
        pkg-config \
        libdbus-1-dev \
        ibus
else
    echo "  Non-Debian system. Ensure you have:"
    echo "    build-essential / base-devel"
    echo "    libdbus-1-dev / dbus-devel"
    echo "    ibus"
fi

# ── 2. Build ──────────────────────────────────────────────────────────────────

echo ""
echo "==> Building Rust binary (release)..."
cd "$REPO/rust-core"
cargo build --release
echo "  smarttype-engine — ok"

echo ""
echo "==> Building Go daemon..."
cd "$REPO/go-daemon"
go mod download
go build -o smarttype-daemon -ldflags="-s -w" -trimpath .
echo "  smarttype-daemon — ok"

# ── 3. Stop any running instance ─────────────────────────────────────────────

echo ""
echo "==> Stopping any running SmartType service..."
systemctl --user stop smarttype.service 2>/dev/null || true

# ── 4. Remove stale binaries from previous installs ──────────────────────────

echo "==> Removing old binaries..."
sudo rm -f "$BIN/smarttype-hook" "$BIN/smarttype-popup" \
           "$BIN/smarttype-engine" "$BIN/smarttype-daemon"

# ── 5. Install binaries ───────────────────────────────────────────────────────

echo ""
echo "==> Installing binaries to $BIN (requires sudo)..."

sudo install -Dm755 "$REPO/rust-core/target/release/smarttype-engine" "$BIN/smarttype-engine"
sudo install -Dm755 "$REPO/go-daemon/smarttype-daemon"                 "$BIN/smarttype-daemon"

# ── 6. Register IBus engine ───────────────────────────────────────────────────

echo ""
echo "==> Registering IBus engine..."

sudo install -Dm644 "$REPO/ibus/smarttype.xml" "$IBUS_COMPONENT_DIR/smarttype.xml"

# Patch the exec path in the installed component file to the actual binary
sudo sed -i "s|<exec>.*</exec>|<exec>$BIN/smarttype-engine</exec>|" \
    "$IBUS_COMPONENT_DIR/smarttype.xml"

# Rebuild IBus component cache so ibus-daemon sees the new engine
if command -v ibus &>/dev/null; then
    ibus write-cache --system 2>/dev/null || true
    echo "  IBus component cache updated."
fi

echo "  Restart ibus-daemon and select 'SmartType' in IBus preferences."

# ── 7. Systemd user service ───────────────────────────────────────────────────

echo ""
echo "==> Installing systemd user service..."

SERVICE_DIR="$HOME/.config/systemd/user"
mkdir -p "$SERVICE_DIR"

cat > "$SERVICE_DIR/smarttype.service" << EOF
[Unit]
Description=SmartType IBus typing assistant
After=graphical-session.target

[Service]
Type=simple
ExecStart=$BIN/smarttype-daemon
Restart=on-failure
RestartSec=3

[Install]
WantedBy=default.target
EOF

systemctl --user daemon-reload
systemctl --user enable smarttype.service
systemctl --user start smarttype.service
echo "  Service enabled and started."

# ── Done ──────────────────────────────────────────────────────────────────────

echo ""
echo "Installation complete."
echo ""
echo "  $BIN/smarttype-engine   (IBus engine)"
echo "  $BIN/smarttype-daemon   (process supervisor)"
echo "  $IBUS_COMPONENT_DIR/smarttype.xml"
echo ""
echo "Next steps:"
echo "  1. Open IBus preferences and enable the 'SmartType' input method."
echo "  2. journalctl --user -u smarttype -f   (to check logs)"
