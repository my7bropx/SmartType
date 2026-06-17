#!/usr/bin/env bash
set -euo pipefail

REPO="$(cd "$(dirname "$0")/.." && pwd)"
BIN="/usr/local/bin"

# ── Build ─────────────────────────────────────────────────────────────────────
"$REPO/scripts/build-all.sh"

# ── Install binaries ──────────────────────────────────────────────────────────
echo ""
echo "==> Installing to $BIN (requires sudo)..."

sudo install -Dm755 "$REPO/rust-core/target/release/smarttype-hook"  "$BIN/smarttype-hook"
sudo install -Dm755 "$REPO/rust-core/target/release/smarttype-popup" "$BIN/smarttype-popup"
sudo install -Dm755 "$REPO/go-daemon/smarttype-daemon"               "$BIN/smarttype-daemon"

# ── Input-device permissions ──────────────────────────────────────────────────
echo ""
echo "==> Setting up permissions..."

# udev rule so /dev/input/event* is readable by the 'input' group
UDEV_RULE='/etc/udev/rules.d/99-smarttype.rules'
if [ ! -f "$UDEV_RULE" ]; then
    sudo tee "$UDEV_RULE" > /dev/null << 'EOF'
SUBSYSTEM=="input", KERNEL=="event[0-9]*", GROUP="input", MODE="0660"
EOF
    sudo udevadm control --reload-rules
    sudo udevadm trigger
    echo "  udev rule written: $UDEV_RULE"
fi

# Add current user to 'input' group so hook can read keyboard events
TARGET_USER="${SUDO_USER:-$USER}"
if ! id -nG "$TARGET_USER" | grep -qw input; then
    sudo usermod -aG input "$TARGET_USER"
    echo "  Added $TARGET_USER to 'input' group — log out/in for it to take effect."
else
    echo "  $TARGET_USER is already in the 'input' group."
fi

# ── systemd user service ──────────────────────────────────────────────────────
echo ""
echo "==> Installing systemd user service..."

SERVICE_DIR="$HOME/.config/systemd/user"
mkdir -p "$SERVICE_DIR"

cat > "$SERVICE_DIR/smarttype.service" << EOF
[Unit]
Description=SmartType typing assistant
After=graphical-session.target

[Service]
Type=simple
ExecStart=$BIN/smarttype-daemon
Restart=on-failure
RestartSec=3
Environment=DISPLAY=:0

[Install]
WantedBy=default.target
EOF

systemctl --user daemon-reload
systemctl --user enable smarttype.service
echo "  Service enabled. Start now with: systemctl --user start smarttype"

# ── Done ──────────────────────────────────────────────────────────────────────
echo ""
echo "Installation complete."
echo ""
echo "  smarttype-hook    -> $BIN/smarttype-hook"
echo "  smarttype-popup   -> $BIN/smarttype-popup"
echo "  smarttype-daemon  -> $BIN/smarttype-daemon"
echo ""
echo "Quick start:"
echo "  systemctl --user start smarttype"
echo "  systemctl --user status smarttype"
