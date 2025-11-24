# SmartType Usage Guide

Complete guide to using SmartType for system-wide autocorrect on Linux.

## Getting Started

### First-Time Setup

1. **Start SmartType:**
   ```bash
   systemctl --user start smarttype
   ```

2. **Enable at startup:**
   ```bash
   systemctl --user enable smarttype
   ```

3. **Open configuration GUI:**
   ```bash
   smarttype-config
   ```

### Basic Usage

SmartType runs in the background and automatically corrects typos as you type. No keyboard shortcuts or special actions needed - just type naturally!

## Features

### Autocorrect

Automatically fixes 2000+ common typos:

| You Type | SmartType Fixes |
|----------|-----------------|
| teh      | the             |
| recieve  | receive         |
| becuase  | because         |
| definately | definitely    |
| seperate | separate        |

**Case Preservation:**
- `Teh` → `The`
- `TEH` → `THE`
- `teh` → `the`

### Smart Punctuation

Automatically converts straight quotes to typographic quotes:

| You Type | SmartType Converts |
|----------|-------------------|
| "hello"  | "hello"           |
| 'world'  | 'world'           |
| don't    | don't             |
| ...      | …                 |
| --       | —                 |

**Smart Detection:**
- Quotes: Contextually determines opening vs closing
- Apostrophes: Automatically detected in contractions
- Dashes: Double hyphen becomes em dash
- Ellipsis: Three dots become single character

## Configuration

### GUI Configuration Tool

Launch with: `smarttype-config`

#### General Tab

- **Enable SmartType:** Master switch for all features
- **Enable Autocorrect:** Turn typo correction on/off
- **Enable Smart Punctuation:** Turn smart quotes on/off
- **Minimum Word Length:** Only correct words ≥ this length
- **Toggle Hotkey:** Keyboard shortcut to enable/disable

#### Applications Tab

Configure per-application behavior:

```
Application  | Enabled | Smart Quotes | Autocorrect
-------------|---------|--------------|------------
firefox      | ✓       | ✓            | ✓
qterminal    | ✓       | ✗            | ✓
code         | ✓       | ✗            | ✓
```

**Add New Application:**
1. Click "Add Application"
2. Enter application name (e.g., "slack")
3. Configure options
4. Save configuration

**Finding Application Names:**
```bash
# Get active window name
xdotool getactivewindow getwindowname
```

#### Custom Corrections Tab

Add your own typo corrections:

1. Click "Add Correction"
2. Enter typo (e.g., "tpyo")
3. Enter correction (e.g., "typo")
4. Save configuration

**Bulk Import:**
Edit `~/.config/smarttype/config.yaml`:
```yaml
custom_typos:
  mytypo1: mycorrection1
  mytypo2: mycorrection2
  customabbrev: "custom expansion"
```

#### Statistics Tab

View autocorrect statistics:
- Total corrections (all time)
- Session corrections (since start)
- Uptime
- Dictionary size

### Command-Line Interface

#### Service Management

```bash
# Start service
smarttype-cli start

# Stop service
smarttype-cli stop

# Restart service
smarttype-cli restart

# Check status
smarttype-cli status

# Enable at startup
smarttype-cli enable

# Disable at startup
smarttype-cli disable
```

#### Testing

```bash
# Test autocorrect
smarttype-cli test "teh quick borwn fox"
# Output: the quick brown fox

# Test with smart punctuation
smarttype-cli test '"Hello world"'
# Output: "Hello world"
```

#### Configuration

```bash
# Open GUI
smarttype-cli config

# Edit config file directly
$EDITOR ~/.config/smarttype/config.yaml

# Reload configuration
smarttype-cli restart
```

### Configuration File

Located at: `~/.config/smarttype/config.yaml`

#### Example Configuration

```yaml
# Master switch
enabled: true

# Feature toggles
smart_punctuation: true
autocorrect: true
min_word_length: 2

# Keyboard shortcut to toggle SmartType
hotkey: "Super+Shift+A"

# Per-application settings
applications:
  firefox:
    enabled: true
    smart_quotes: true
    autocorrect: true

  qterminal:
    enabled: true
    smart_quotes: false  # Disable smart quotes in terminal
    autocorrect: true

  kitty:
    enabled: true
    smart_quotes: false
    autocorrect: true

  code:  # VS Code
    enabled: true
    smart_quotes: false
    autocorrect: true

  slack:
    enabled: true
    smart_quotes: true
    autocorrect: true

  telegram:
    enabled: true
    smart_quotes: true
    autocorrect: true

# Custom typo corrections
custom_typos:
  # Common typos
  hte: the
  becuase: because
  recieve: receive

  # Personal abbreviations
  btw: "by the way"
  afaik: "as far as I know"
  imho: "in my humble opinion"

  # Work-specific
  acme: "ACME Corporation"
  proj: "Project Management"
```

## Application-Specific Guides

### Firefox

SmartType works automatically in all text fields:
- Address bar
- Search boxes
- Web forms
- Text areas

**Recommended Settings:**
```yaml
firefox:
  enabled: true
  smart_quotes: true
  autocorrect: true
```

### Terminals (Qterminal, Kitty, Alacritty)

Smart quotes disabled by default (interferes with commands):

**Recommended Settings:**
```yaml
qterminal:
  enabled: true
  smart_quotes: false  # Keep false for terminals
  autocorrect: true    # Safe for autocorrect
```

**Selective Disable:**
If autocorrect interferes with commands, add exceptions:
```yaml
custom_typos:
  git: git  # Prevent correction of git commands
  ssh: ssh
```

### VS Code / Text Editors

Works in all editor windows:

**Recommended Settings:**
```yaml
code:
  enabled: true
  smart_quotes: false  # Disable for code
  autocorrect: true    # Useful for comments
```

**For Markdown/Writing:**
```yaml
code:
  enabled: true
  smart_quotes: true   # Enable for prose
  autocorrect: true
```

### Chat Applications (Slack, Discord, Telegram)

Full support with smart punctuation:

```yaml
slack:
  enabled: true
  smart_quotes: true
  autocorrect: true
```

### LibreOffice / Office Suites

Works in all office applications:

```yaml
libreoffice:
  enabled: true
  smart_quotes: true
  autocorrect: true
```

## Advanced Usage

### Temporary Disable

**Using Hotkey:**
Press configured hotkey (default: `Super+Shift+A`) to toggle

**Using CLI:**
```bash
smarttype-cli stop
# ... do work ...
smarttype-cli start
```

**Using GUI:**
Open `smarttype-config` and uncheck "Enable SmartType"

### Disable for Specific Window

**Temporary:**
Stop SmartType, work in window, restart

**Permanent:**
Add application to config with `enabled: false`

### Custom Dictionaries

Create specialized dictionaries for different contexts:

**Technical Writing:**
```yaml
custom_typos:
  api: API
  json: JSON
  http: HTTP
  db: database
```

**Academic Writing:**
```yaml
custom_typos:
  eg: "e.g."
  ie: "i.e."
  et al: "et al."
```

### Debugging

**Enable Debug Logging:**
```bash
# Set environment variable
export RUST_LOG=debug

# Restart service
systemctl --user restart smarttype

# View logs
journalctl --user -u smarttype -f
```

**Check Hook Status:**
```bash
# Find hook process
ps aux | grep smarttype-hook

# Check capabilities
getcap /usr/local/bin/smarttype-hook
```

**Monitor Input Events:**
```bash
# Requires root
sudo evtest
```

## Tips & Tricks

### Best Practices

1. **Start Minimal:** Begin with default config, add customizations gradually
2. **Per-App Config:** Configure different settings for different apps
3. **Review Stats:** Check statistics to see what's being corrected
4. **Backup Config:** Keep a backup of `~/.config/smarttype/config.yaml`

### Common Customizations

**Disable for Passwords:**
```yaml
# Add password managers to config
keepassxc:
  enabled: false
```

**Programming Shortcuts:**
```yaml
custom_typos:
  func: function
  ret: return
  const: const
```

**Writing Shortcuts:**
```yaml
custom_typos:
  spp: "smart punctuation"
  autocorrect: "autocorrect"
```

### Performance Optimization

**Reduce CPU Usage:**
```yaml
min_word_length: 3  # Increase to skip short words
```

**Reduce Memory:**
Disable SmartType for resource-intensive applications

### Integration with Other Tools

**With Text Expanders:**
SmartType complements tools like Autokey and Espanso

**With Spell Checkers:**
Works alongside browser/editor spell checkers

**With Input Methods:**
Compatible with IBus, fcitx for multilingual typing

## Troubleshooting

### Common Issues

**Autocorrect Not Working:**
1. Check service status: `systemctl --user status smarttype`
2. Verify permissions: `groups | grep input`
3. Check application config: `smarttype-cli config`

**Wrong Corrections:**
1. Add exception in custom typos
2. Increase min_word_length
3. Disable for specific application

**Performance Issues:**
1. Increase min_word_length
2. Reduce custom typos
3. Disable for heavy applications

**Not Working in Terminal:**
1. Check terminal application name
2. Ensure terminal in config
3. Verify smart_quotes disabled for terminals

## FAQ

**Q: Does SmartType log my keystrokes?**
A: No. SmartType processes in real-time and doesn't log.

**Q: Can I use SmartType on Wayland?**
A: Partial support via XWayland. Full Wayland support coming soon.

**Q: Does it work with non-English keyboards?**
A: Currently optimized for English. Multi-language support planned.

**Q: Can I sync settings across machines?**
A: Yes, sync `~/.config/smarttype/config.yaml` via git or cloud storage.

**Q: Does it work in virtual machines?**
A: Yes, when running Linux as host or guest.

## Getting Help

- **Documentation:** https://smarttype.dev/docs
- **GitHub Issues:** https://github.com/yourusername/smarttype/issues
- **Discord Community:** https://discord.gg/smarttype
- **Email Support:** support@smarttype.dev

## Contributing

Help improve SmartType:
- Report bugs and issues
- Suggest features
- Submit typo corrections
- Contribute code

See CONTRIBUTING.md for details.

---

Happy typing with SmartType! 🚀
