# SmartType

System-wide autocomplete and autocorrect for Linux, implemented as an **IBus input method engine**. As you type, SmartType buffers your current word as inline preedit text, shows ranked suggestions in IBus's native candidate window (positioned at your caret by the framework), and commits the chosen text into the focused application — no custom X11 drawing, no evdev snooping.

---

## Architecture

```
┌──────────────────────────────────────────────────────────┐
│                     User keyboard                        │
└───────────────────────────┬──────────────────────────────┘
                            │  key events
                            ▼
┌──────────────────────────────────────────────────────────┐
│                      ibus-daemon                         │
│              (system-wide IME framework)                 │
└──────────┬───────────────────────────────┬───────────────┘
           │ D-Bus (ProcessKeyEvent)        │ renders
           ▼                               ▼
┌──────────────────────┐       ┌───────────────────────┐
│   smarttype-engine   │       │  IBus candidate window │
│  (Rust, this repo)   │       │  (positioned at caret  │
│                      │       │   by IBus / the app)   │
│  ProcessKeyEvent()   │       └───────────────────────┘
│  · buffer preedit         CommitText() ──────────────────►
│  · WordCompleter     │    (text delivered to focused app)
│  · AutocorrectEngine │
│                      │    UpdateLookupTable() ───────────►
│  CandidateClicked()  │    (suggestions shown in panel)
│  · commit chosen     │
└──────────────────────┘
           ▲
┌──────────┴───────────┐
│  smarttype-daemon    │
│  (Go, this repo)     │
│  · daemonizes        │
│  · spawns engine     │
│  · auto-restarts     │
│  · hot-reloads cfg   │
└──────────────────────┘
```

### How a keystroke flows

1. IBus intercepts the keypress and calls `ProcessKeyEvent` on the engine over D-Bus.
2. The engine appends the character to an in-memory word buffer and calls `UpdatePreEditText` (the buffer appears inline in the focused field, underlined).
3. If the buffer is ≥ 2 characters, `WordCompleter::suggest` runs a BTreeMap prefix scan topped up with edit-distance-1 near-misses; the engine calls `UpdateLookupTable` so IBus shows up to 5 ranked candidates at the caret.
4. **Accept** — user presses `Tab`, `1`–`5`, or clicks a candidate: engine clears the buffer, calls `CommitText(chosen_word)`, and IBus delivers it to the app.
5. **Word boundary** — user presses `Space` or `Enter`: engine passes the buffer through `AutocorrectEngine` (custom rules + built-in typo dictionary), then commits `corrected_word + delimiter`.
6. **Navigation / Escape** — engine flushes the buffer as-is, hides the candidate window, and lets the key through to the app.

---

## Features

- Prefix autocomplete with edit-distance-1 fallback (catches transpositions while typing)
- Post-word autocorrect on `Space` / `Enter` (custom rules + built-in typo dictionary)
- Frequency-based ranking — accepted words float higher in future suggestions
- Learning persists across reboots (`~/.local/share/smarttype/learned_words.json`)
- Smart punctuation: `"..."` → `"…"`, `--` → `—`, `...` → `…`
- Candidate window position and rendering handled entirely by IBus
- No evdev, no uinput, no X11 drawing code, no extra permissions needed

---

## Key bindings

| Key | Action |
|-----|--------|
| `Tab` | Accept suggestion #1 |
| `1` – `5` | Accept suggestion #1 – #5 |
| `Space` / `Enter` | Commit current word (autocorrect applied), pass delimiter |
| `Backspace` | Remove last character from preedit buffer |
| `Esc` / arrows / `Home` / `End` | Flush preedit as-is, pass key through |
| Mouse click on candidate | Accept that candidate |

---

## Requirements

| Requirement | Notes |
|-------------|-------|
| Linux | Tested on Kali Linux / Debian / Ubuntu |
| IBus | `ibus-daemon` must be running |
| Rust 1.70+ | `cargo` in PATH; install via [rustup](https://rustup.rs) |
| Go 1.21+ | `go` in PATH; `sudo apt install golang-go` |
| `libdbus-1-dev` | Build dependency for the `zbus` crate |

---

## Installation

```bash
sudo ./scripts/install.sh
```

The script handles everything in order:

1. **Checks** Rust, Go, and system packages (`libdbus-1-dev`, `ibus`); installs missing ones.
2. **Builds** `smarttype-engine` (Rust, release) and `smarttype-daemon` (Go).
3. **Installs** both binaries to `/usr/local/bin/`.
4. **Registers** the IBus engine by installing `ibus/smarttype.xml` to `/usr/share/ibus/component/` and running `ibus write-cache`.
5. **Creates** `~/.config/systemd/user/smarttype.service` and enables it.

After install:

```bash
# Enable the engine in IBus preferences
ibus-setup     # → Input Method tab → Add → SmartType

# Start the daemon
systemctl --user start smarttype

# Watch logs
journalctl --user -u smarttype -f
```

### Build only (no install)

```bash
# Rust engine
cd rust-core && cargo build --release

# Go daemon
cd go-daemon && go build -o smarttype-daemon .
```

### Uninstall

```bash
systemctl --user stop smarttype
systemctl --user disable smarttype
rm -f ~/.config/systemd/user/smarttype.service
systemctl --user daemon-reload

sudo rm -f /usr/local/bin/smarttype-engine \
           /usr/local/bin/smarttype-daemon \
           /usr/share/ibus/component/smarttype.xml

ibus write-cache --system

# Optional: remove learned words
rm -rf ~/.local/share/smarttype
```

---

## Configuration

`~/.config/smarttype/config.yaml` — created automatically on first run:

```yaml
enabled: true
autocorrect: true
smart_punctuation: true
min_word_length: 2

custom_typos:
  hte: the
  becuase: because
```

The daemon watches this file and reloads without restart when it changes. You can also send `SIGHUP` manually:

```bash
kill -HUP "$(cat /tmp/smarttype.pid)"
```

---

## Project layout

```
SmartType/
├── rust-core/                   Rust library + engine binary
│   ├── Cargo.toml
│   └── src/
│       ├── bin/
│       │   └── engine.rs        smarttype-engine — IBus engine entry point
│       ├── autocomplete.rs      BTreeMap prefix lookup + Norvig edit-distance
│       ├── engine.rs            Rule-based autocorrect engine
│       ├── dictionary.rs        Built-in English typo dictionary
│       ├── smart_punctuation.rs Curly quotes, em-dash, ellipsis
│       ├── config.rs            YAML config loader
│       └── lib.rs               Library re-exports
│
├── go-daemon/                   Go process supervisor
│   ├── main.go                  Daemonisation and signal handling
│   └── service.go               Binary discovery, spawn, auto-restart, config watch
│
├── ibus/
│   └── smarttype.xml            IBus component descriptor
│                                (install to /usr/share/ibus/component/)
│
├── scripts/
│   └── install.sh               Unified: deps + build + install + IBus register
│
├── .github/workflows/
│   └── rust.yml                 CI — cargo build + test on push
│
├── README.md
└── LICENSE
```

---

## Development

```bash
# Run tests
cd rust-core && cargo test

# Check without building
cd rust-core && cargo check

# Run engine directly (IBus must be running)
RUST_LOG=info cargo run --bin smarttype-engine
```

IBus address is read from `$IBUS_ADDRESS` (set by IBus when it launches the engine) or auto-discovered from `~/.config/ibus/bus/<machine-id>-unix-<display>-0`.

---

## Troubleshooting

**Engine does not appear in IBus preferences**

```bash
sudo ibus write-cache --system
ibus restart
```

**Engine fails to connect — "IBus not running"**

```bash
ibus-daemon -drx
systemctl --user restart smarttype
```

**Checking logs**

```bash
journalctl --user -u smarttype -f
# or run directly:
RUST_LOG=debug smarttype-engine
```

---

## Links

- Repository: https://github.com/my7bropx/SmartType
- Issues: https://github.com/my7bropx/SmartType/issues
