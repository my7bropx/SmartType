# SmartType Installation Guide

Complete installation guide for SmartType on Linux systems.

## System Requirements

### Minimum Requirements
- Linux kernel 4.0+
- 512 MB RAM
- 100 MB disk space
- X11 or Wayland display server

### Tested Distributions
- Kali Linux 2023.x+
- Ubuntu 22.04+
- Debian 12+
- Arch Linux
- Fedora 38+

## Quick Installation

### One-Line Install (Recommended)

```bash
curl -sSL https://raw.githubusercontent.com/smarttype/smarttype/main/scripts/quick-install.sh | sudo bash
```

### Manual Installation

#### Step 1: Install Dependencies

```bash
# Clone repository
git clone https://github.com/yourusername/smarttype
cd smarttype

# Install dependencies
sudo ./scripts/install-deps.sh
```

This script will automatically detect your distribution and install:
- Rust (cargo, rustc)
- Go 1.21+
- Python 3.9+
- PyQt5
- Development libraries (libevdev, libudev, libx11, etc.)

#### Step 2: Build SmartType

```bash
# Build all components
./scripts/build-all.sh
```

This builds:
1. Rust core engine (autocorrect + input hook)
2. Go daemon service
3. Python GUI configuration tool

Build time: ~5-10 minutes on typical hardware

#### Step 3: Install System-Wide

```bash
# Install SmartType
sudo ./scripts/install.sh
```

This installs:
- Binaries to `/usr/local/bin/`
- Systemd service file
- Udev rules for input device access
- Default configuration

#### Step 4: Configure Permissions

**Important:** Log out and log back in after installation for group permissions to take effect.

```bash
# Log out and back in, then verify group membership
groups | grep input
```

#### Step 5: Start SmartType

```bash
# Enable and start service
systemctl --user enable --now smarttype

# Check status
systemctl --user status smarttype
```

## Distribution-Specific Notes

### Kali Linux

Kali Linux is fully supported and tested. No special configuration needed.

```bash
sudo apt update
sudo apt install -y build-essential cargo rustc golang python3-pyqt5
```

### Ubuntu/Debian

```bash
# Install additional packages if needed
sudo apt install libevdev-dev libudev-dev libx11-dev
```

### Arch Linux

```bash
# Install from AUR (coming soon)
yay -S smarttype

# Or build manually
sudo pacman -S base-devel rust go python python-pyqt5
```

### Fedora

```bash
sudo dnf install gcc rust cargo golang python3-qt5
```

## Post-Installation Configuration

### GUI Configuration

Launch the configuration GUI:

```bash
smarttype-config
```

Features:
- Enable/disable SmartType
- Configure autocorrect and smart punctuation
- Per-application settings
- Custom typo corrections
- Statistics and monitoring

### Command-Line Interface

```bash
# Start service
smarttype-cli start

# Stop service
smarttype-cli stop

# Check status
smarttype-cli status

# Test autocorrect
smarttype-cli test "teh quick borwn fox"

# Open GUI
smarttype-cli config
```

### Manual Configuration

Edit `~/.config/smarttype/config.yaml`:

```yaml
enabled: true
smart_punctuation: true
autocorrect: true
min_word_length: 2

applications:
  firefox:
    enabled: true
    smart_quotes: true
  qterminal:
    enabled: true
    smart_quotes: false  # Disable in terminal

custom_typos:
  mytypo: mycorrection
```

## Troubleshooting

### Service Won't Start

**Check logs:**
```bash
journalctl --user -u smarttype -f
```

**Verify installation:**
```bash
which smarttype-daemon
which smarttype-hook
```

**Check permissions:**
```bash
groups | grep input
getcap /usr/local/bin/smarttype-hook
```

### Not Working in Specific Applications

**Check application name:**
```bash
xdotool getactivewindow getwindowname
```

**Add application to config:**
```yaml
applications:
  myapp:
    enabled: true
    autocorrect: true
```

### High CPU Usage

**Reduce dictionary size:**
Edit config to increase `min_word_length` to 3 or 4.

**Disable for specific apps:**
Set `enabled: false` for resource-intensive applications.

### Permission Denied Errors

**Add user to input group:**
```bash
sudo usermod -a -G input $USER
# Log out and back in
```

**Set capabilities:**
```bash
sudo setcap cap_dac_override,cap_sys_admin+ep /usr/local/bin/smarttype-hook
```

## Uninstallation

To completely remove SmartType:

```bash
# Stop and disable service
systemctl --user stop smarttype
systemctl --user disable smarttype

# Remove binaries
sudo rm /usr/local/bin/smarttype-*

# Remove systemd service
sudo rm /usr/lib/systemd/user/smarttype.service
sudo systemctl --user daemon-reload

# Remove configuration
rm -rf ~/.config/smarttype

# Remove udev rules
sudo rm /etc/udev/rules.d/99-smarttype.rules
sudo udevadm control --reload-rules
```

## Advanced Installation

### Building with Custom Features

```bash
# Build with specific features
cd rust-core
cargo build --release --features "extra-dictionaries,debug-mode"
```

### Installing to Custom Location

```bash
# Set custom prefix
PREFIX=/opt/smarttype sudo ./scripts/install.sh
```

### Running Without systemd

```bash
# Start daemon manually
smarttype-daemon -d

# Or run in foreground
smarttype-daemon
```

## Getting Help

- Documentation: https://smarttype.dev/docs
- Issues: https://github.com/yourusername/smarttype/issues
- Discord: https://discord.gg/smarttype
- Email: support@smarttype.dev

## Security Considerations

SmartType requires elevated privileges to access input devices:

1. **Input Group:** User is added to `input` group for device access
2. **Capabilities:** `smarttype-hook` has CAP_DAC_OVERRIDE capability
3. **Local Only:** All processing happens locally, no network access
4. **Open Source:** Full source code available for audit

SmartType does NOT:
- Send data to external servers
- Log keystrokes to disk
- Require root access during normal operation

## Next Steps

After installation:

1. **Configure Applications:** Add per-app rules via GUI
2. **Add Custom Typos:** Add your common typos
3. **Test Thoroughly:** Test in your daily applications
4. **Report Issues:** Help improve SmartType by reporting bugs

Enjoy system-wide autocorrect on Linux!
