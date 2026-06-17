# SmartType

System-wide typing assistant for Linux. Shows word completions above the cursor as you type, autocorrects common typos on space, and learns from your writing — all keyboard-driven, zero mouse required.

## How it works

As you type, a bar appears just above the cursor showing up to 5 suggestions ranked by relevance. Accept one without lifting your hands:

| Key | Action |
|-----|--------|
| `Tab` | Accept top suggestion |
| `1`–`5` | Accept nth suggestion |
| `Backspace` | After a space: restore the previous word for re-editing |

If you type a word that looks like a misspelling and press `Space`, SmartType shows correction suggestions. Press `Tab` or `1`–`5` to replace it; start typing to dismiss.

SmartType also learns which words you use most and promotes them in future suggestions.

## Architecture

Three Rust binaries coordinated by a Go daemon:

```
smarttype-daemon (Go)
  ├── smarttype-hook   (Rust) — reads keyboard via evdev, types corrections via uinput
  └── smarttype-popup  (Rust) — draws the X11 suggestion bar above the cursor
          ↑
     Unix socket  /tmp/smarttype-popup.sock
```

- **hook** reads raw keyboard events from `/dev/input/event*` (no X11 dependency)
- **popup** is a borderless X11 overlay, repositions itself to float above the cursor
- **daemon** spawns both, watches the config file, and auto-restarts either if they crash

## Features

- Prefix autocomplete with edit-distance fallback (catches transpositions, deletions, substitutions)
- Post-space typo correction with `pending_word` state — backspace returns you to the mis-typed word
- Smart punctuation: `"` → `"…"`, `'...'` → `'…'`, `--` → `—`, `...` → `…`
- Per-word frequency tracking; learned words persist across reboots
- Catppuccin-themed suggestion bar with color-coded priorities
- Zero CPU when idle (epoll via `AsyncFd`)
- No network access, no keystroke logging, all local

## Quick start

```bash
# 1. Install build dependencies
./scripts/install-deps.sh

# 2. Build and install (requires sudo for /usr/local/bin and udev)
sudo ./scripts/install.sh

# 3. Log out and back in so the 'input' group takes effect

# 4. Start
systemctl --user start smarttype
```

See [INSTALL.md](INSTALL.md) for detailed steps and [USAGE.md](USAGE.md) for configuration.

## Project layout

```
SmartType/
├── rust-core/           Rust library + two binaries
│   └── src/
│       ├── bin/
│       │   ├── hook.rs      keyboard hook binary
│       │   └── popup.rs     X11 suggestion bar binary
│       ├── hook.rs          evdev reader, uinput writer, state machine
│       ├── autocomplete.rs  BTreeMap prefix lookup + Norvig edit-distance
│       ├── engine.rs        rule-based autocorrect engine
│       ├── dictionary.rs    built-in typo dictionary
│       ├── smart_punctuation.rs
│       ├── config.rs
│       └── lib.rs
├── go-daemon/           Go process supervisor
│   ├── main.go          daemonisation, signal handling
│   └── service.go       binary discovery, lifecycle, auto-restart
└── scripts/
    ├── install-deps.sh  install Rust, Go, system libs, input group
    ├── build-all.sh     cargo build --release + go build
    └── install.sh       build + copy to /usr/local/bin + systemd unit
```

## Requirements

- Linux kernel 4.0+ (evdev + uinput)
- X11 display server
- User in the `input` group (installer handles this)
- Rust 1.70+, Go 1.21+

## Links

- Repository: https://github.com/my7bropx/SmartType
- Issues: https://github.com/my7bropx/SmartType/issues
