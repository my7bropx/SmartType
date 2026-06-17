#!/bin/bash
#
# SmartType Installation Script
# Installs SmartType system-wide
#

set -e

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(dirname "$SCRIPT_DIR")"

# Check if running as root
if [ "$EUID" -ne 0 ]; then
    echo "This script must be run as root (use sudo)"
    exit 1
fi

cd "$PROJECT_ROOT"

echo "================================"
echo " Installing SmartType"
echo "================================"
echo ""

# Check if built
if [ ! -f "rust-core/target/release/smarttype-hook" ]; then
    echo "Error: SmartType not built yet!"
    echo "Please run: ./scripts/build-all.sh"
    exit 1
fi

# Install binaries
echo "[1/5] Installing binaries..."
install -Dm755 rust-core/target/release/smarttype-hook /usr/local/bin/smarttype-hook
install -Dm755 go-daemon/smarttype-daemon /usr/local/bin/smarttype-daemon
install -Dm755 python-gui/main.py /usr/local/bin/smarttype-config

# Create wrapper script
cat > /usr/local/bin/smarttype-cli << 'EOF'
#!/bin/bash
# SmartType CLI wrapper

case "$1" in
    status)
        systemctl --user status smarttype
        ;;
    start)
        systemctl --user start smarttype
        ;;
    stop)
        systemctl --user stop smarttype
        ;;
    restart)
        systemctl --user restart smarttype
        ;;
    enable)
        systemctl --user enable smarttype
        ;;
    disable)
        systemctl --user disable smarttype
        ;;
    test)
        echo "Testing autocorrect: $2"
        # Would call Rust library to test
        ;;
    config)
        smarttype-config
        ;;
    *)
        echo "SmartType CLI"
        echo ""
        echo "Usage: smarttype-cli <command>"
        echo ""
        echo "Commands:"
        echo "  status     - Show service status"
        echo "  start      - Start SmartType service"
        echo "  stop       - Stop SmartType service"
        echo "  restart    - Restart SmartType service"
        echo "  enable     - Enable service at startup"
        echo "  disable    - Disable service at startup"
        echo "  test TEXT  - Test autocorrect on text"
        echo "  config     - Open GUI configuration"
        ;;
esac
EOF

chmod +x /usr/local/bin/smarttype-cli
echo "  ✓ Binaries installed to /usr/local/bin/"
echo ""

# Install systemd service
echo "[2/5] Installing systemd service..."
mkdir -p /usr/lib/systemd/user/

cat > /usr/lib/systemd/user/smarttype.service << 'EOF'
[Unit]
Description=SmartType Autocorrect Daemon
After=graphical-session.target

[Service]
Type=simple
ExecStart=/usr/local/bin/smarttype-daemon
Restart=on-failure
RestartSec=5

[Install]
WantedBy=default.target
EOF

echo "  ✓ Systemd service installed"
echo ""

# Set up permissions
echo "[3/5] Setting up permissions..."

# Add user to input group (if exists)
if getent group input > /dev/null; then
    echo "  - Adding current user to 'input' group..."
    usermod -a -G input $SUDO_USER
    echo "    Note: You'll need to log out and back in for group changes to take effect"
fi

# Set capabilities for input hook
echo "  - Setting capabilities for smarttype-hook..."
setcap cap_dac_override,cap_sys_admin+ep /usr/local/bin/smarttype-hook

echo "  ✓ Permissions configured"
echo ""

# Create udev rules for input devices
echo "[4/5] Creating udev rules..."
cat > /etc/udev/rules.d/99-smarttype.rules << 'EOF'
# Allow members of input group to access input devices
KERNEL=="event*", SUBSYSTEM=="input", GROUP="input", MODE="0660"

# Grant access to evdev devices
SUBSYSTEM=="input", KERNEL=="event[0-9]*", GROUP="input", MODE="0660"
EOF

udevadm control --reload-rules
udevadm trigger

echo "  ✓ Udev rules created"
echo ""

# Create default configuration
echo "[5/5] Creating default configuration..."
SUDO_HOME=$(eval echo ~$SUDO_USER)
CONFIG_DIR="$SUDO_HOME/.config/smarttype"

mkdir -p "$CONFIG_DIR"

if [ ! -f "$CONFIG_DIR/config.yaml" ]; then
    cat > "$CONFIG_DIR/config.yaml" << 'EOF'
enabled: true
smart_punctuation: true
autocorrect: true
min_word_length: 2

applications:
  firefox:
    enabled: true
    smart_quotes: true
    autocorrect: true
  qterminal:
    enabled: true
    smart_quotes: false
    autocorrect: true
  kitty:
    enabled: true
    smart_quotes: false
    autocorrect: true
  alacritty:
    enabled: true
    smart_quotes: false
    autocorrect: true
  code:
    enabled: true
    smart_quotes: false
    autocorrect: true

custom_typos:
  hte: the
  becuase: because

hotkey: "Super+Shift+A"
EOF
fi

chown -R $SUDO_USER:$SUDO_USER "$CONFIG_DIR"
echo "  ✓ Configuration created at $CONFIG_DIR/config.yaml"
echo ""

echo "================================"
echo " Installation Complete!"
echo "================================"
echo ""
echo "SmartType has been installed successfully."
echo ""
echo "Next steps:"
echo "  1. Log out and log back in (for group permissions)"
echo "  2. Enable and start the service:"
echo "     systemctl --user enable --now smarttype"
echo ""
echo "  3. Open configuration GUI:"
echo "     smarttype-config"
echo ""
echo "  4. Check status:"
echo "     smarttype-cli status"
echo ""
echo "Usage:"
echo "  smarttype-cli       - Command-line interface"
echo "  smarttype-config    - GUI configuration tool"
echo ""
echo "Configuration file: $CONFIG_DIR/config.yaml"
echo ""
echo "For help and documentation:"
echo "  https://github.com/yourusername/smarttype"
echo ""
