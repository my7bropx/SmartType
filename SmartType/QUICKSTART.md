# SmartType Quick Start Guide

Get SmartType up and running in 5 minutes!

## Quick Installation (Kali Linux)

```bash
# 1. Navigate to project directory
cd /home/my9broxpki/Desktop/claude/SmartType

# 2. Install dependencies
sudo ./scripts/install-deps.sh

# 3. Build SmartType
./scripts/build-all.sh

# 4. Install system-wide
sudo ./scripts/install.sh

# 5. Log out and back in (important for permissions!)
# Then start the service:
systemctl --user enable --now smarttype
```

## Verify Installation

```bash
# Check service status
smarttype-cli status

# Should show: Active: active (running)
```

## Test It Out

1. **Open any text editor** (e.g., gedit, VS Code)
2. **Type a common typo**: `teh quick borwn fox`
3. **Watch it autocorrect**: `the quick brown fox`

## Configure

```bash
# Open GUI configuration
smarttype-config
```

Configure:
- Enable/disable features
- Per-application settings
- Custom typo corrections

## Common Typos Autocorrected

SmartType automatically fixes 2000+ typos including:

| Typo | Correction |
|------|------------|
| teh | the |
| recieve | receive |
| becuase | because |
| definately | definitely |
| seperate | separate |
| accomodate | accommodate |
| occured | occurred |

## Smart Punctuation Examples

| You Type | SmartType Converts |
|----------|-------------------|
| "hello" | "hello" |
| don't | don't |
| ... | … |
| -- | — |

## Quick Commands

```bash
smarttype-cli start       # Start service
smarttype-cli stop        # Stop service
smarttype-cli status      # Check status
smarttype-cli config      # Open GUI
```

## Troubleshooting

### Service won't start?

```bash
# Check logs
journalctl --user -u smarttype -f

# Verify group membership
groups | grep input
```

If you don't see "input", log out and back in!

### Not working in terminals?

This is expected! Smart quotes are disabled in terminals by default to avoid interfering with commands. Autocorrect still works.

To configure:
```bash
smarttype-config
# Go to Applications tab → configure terminal settings
```

## Application-Specific Tips

### Firefox
✓ Full support - all features enabled

### Qterminal / Kitty
✓ Autocorrect enabled
✗ Smart quotes disabled (by design)

### VS Code
✓ Autocorrect enabled
✗ Smart quotes disabled (for code)
💡 Enable for Markdown: Edit config to enable smart_quotes

### Slack / Discord
✓ Full support - all features enabled

## Configuration File

Edit directly: `~/.config/smarttype/config.yaml`

```yaml
# Quick edits
enabled: true
smart_punctuation: true
autocorrect: true

# Add custom typos
custom_typos:
  mytypo: mycorrection
  hte: the

# Configure applications
applications:
  myapp:
    enabled: true
    autocorrect: true
```

After editing, restart:
```bash
smarttype-cli restart
```

## Keyboard Shortcut

Default hotkey to toggle SmartType: **Super+Shift+A**

Change in config or GUI.

## Performance

- **Memory:** ~15-20 MB
- **CPU:** <1% idle, <5% active typing
- **Latency:** <2ms correction time

## What's Next?

1. **Add Custom Typos**: Open GUI → Custom Corrections tab
2. **Configure Apps**: Go to Applications tab → add your apps
3. **Check Stats**: View Statistics tab to see corrections

## Need Help?

- Full docs: See `README.md`, `INSTALL.md`, `USAGE.md`
- Issues: Report at GitHub
- Logs: `journalctl --user -u smarttype -f`

## Uninstall

```bash
# Stop service
systemctl --user stop smarttype

# Remove files
sudo rm /usr/local/bin/smarttype-*
sudo rm /usr/lib/systemd/user/smarttype.service
rm -rf ~/.config/smarttype
```

---

Enjoy system-wide autocorrect on Linux! 🎉
