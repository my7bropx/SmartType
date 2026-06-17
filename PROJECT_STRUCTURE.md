# Project Structure

## Directory layout

```
SmartType/
├── rust-core/                  Rust workspace (library + 2 binaries)
│   ├── Cargo.toml
│   └── src/
│       ├── lib.rs              Public API re-exports
│       ├── bin/
│       │   ├── hook.rs         smarttype-hook entry point
│       │   └── popup.rs        smarttype-popup entry point
│       ├── hook.rs             Keyboard reader + word-state machine
│       ├── autocomplete.rs     Prefix lookup + edit-distance suggester
│       ├── engine.rs           Rule-based autocorrect engine
│       ├── dictionary.rs       Built-in typo dictionary
│       ├── smart_punctuation.rs Curly quotes, em-dash, ellipsis
│       └── config.rs           YAML config loader
│
├── go-daemon/                  Go process supervisor
│   ├── main.go                 Daemonisation, signal handling
│   └── service.go              Binary discovery, spawn, auto-restart
│
├── scripts/
│   ├── install-deps.sh         Install Rust, Go, system libs, input group
│   ├── build-all.sh            cargo build --release + go build
│   └── install.sh              Build + /usr/local/bin + udev + systemd unit
│
├── README.md
├── INSTALL.md
├── QUICKSTART.md
├── USAGE.md
├── PROJECT_STRUCTURE.md
└── LICENSE
```

## Binaries produced

| Binary | Source | Purpose |
|--------|--------|---------|
| `smarttype-hook` | `rust-core/src/bin/hook.rs` | Reads keyboard, drives suggestions + correction |
| `smarttype-popup` | `rust-core/src/bin/popup.rs` | X11 suggestion bar |
| `smarttype-daemon` | `go-daemon/` | Spawns + supervises the two Rust binaries |

## Data flow

```
User types a key
      │
      ▼
evdev /dev/input/eventN  (kernel input subsystem)
      │
      ▼
smarttype-hook
  ├─ Accumulates characters into word_buffer
  ├─ On each keypress: calls WordCompleter::suggest(prefix)
  │       ├─ BTreeMap range scan for prefix matches
  │       └─ Norvig edit-1 for near-misses when < 5 results
  ├─ On Space: calls AutocorrectEngine::correct_word(word)
  │       ├─ Checks custom_typos map
  │       └─ Checks built-in dictionary
  └─ Sends suggestion list over Unix socket → smarttype-popup
            │
            ▼
    smarttype-popup
      ├─ XQueryPointer → cursor (x, y)
      ├─ Repositions window above cursor
      └─ Draws Catppuccin-themed chip row

When user presses Tab / 1-5:
  hook → VirtualKeyboard (raw uinput ioctls)
    ├─ KEY_BACKSPACE × N  (erase typed prefix or pending word + space)
    └─ KEY_x presses      (type the accepted suggestion)
```

## Key source files

### `rust-core/src/hook.rs`

The core event loop. Responsibilities:
- `find_keyboard_devices()` — scans `/dev/input/event*`, filters by key support, skips own virtual keyboard
- `HookState` — holds `word_buffer`, `suggestions`, `pending_word`, shift/caps state
- `run_stream()` — async loop over `evdev::EventStream` (epoll via `AsyncFd`, zero CPU idle)
- `on_key()` — dispatches on key code: letters build the buffer, Space triggers correction, Tab/1-5 accept, Backspace restores pending word
- `VirtualKeyboard` — raw `/dev/uinput` via `libc::ioctl` (no libudev); `press_backspace(n)` + `type_text(s)`
- `send_to_popup()` — fire-and-forget `spawn_blocking` write to Unix socket
- `do_learn()` — updates `WordCompleter` and spawns async JSON write

### `rust-core/src/autocomplete.rs`

`WordCompleter` wraps two stores:

```rust
words:   BTreeMap<String, u32>   // built-in dictionary + learned, keyed by word
learned: HashMap<String, u32>    // overlay written to JSON on learn
```

- `suggest(prefix, max)` — `range(prefix..upper_bound)` gives prefix matches in O(log n); topped up with edit-1 results when fewer than `max` found; sorted `(edit_dist ASC, freq DESC)`
- `correct(word, max)` — for post-space correction; edit-1 always, edit-2 for short words (≤6 chars); only returns dictionary-valid candidates
- `learn(word)` — increments frequency, returns `(json_snapshot, path)` for async persistence
- `generate_edit1(word)` — Norvig: all single deletions, transpositions, substitutions, insertions

### `rust-core/src/bin/popup.rs`

Borderless X11 window using `x11rb`:
- `override_redirect = true` — bypasses window manager
- `reposition()` — `query_pointer` → centre horizontally on cursor, place above cursor with 14px gap; flips below if cursor is <60px from top
- `render()` — background chip for suggestion #1, colour-coded labels (Catppuccin Mocha palette), accurate vertical text centering from `query_font` metrics
- Listens on Unix socket; updates on every received message

### `go-daemon/service.go`

- `findBinary(name)` — resolves path via `os.Executable()` directory, `../rust-core/target/release/`, then `/usr/local/bin/`
- `startHook()` / `startPopup()` — start child, store `*exec.Cmd`, launch monitoring goroutine
- Monitoring goroutine — calls `cmd.Wait()`, checks `hookStopped` flag, restarts after 2–3s on unexpected exit
- `Stop()` — sets stopped flags, closes `stopChan`, signals both children, `wg.Wait()`
- `Reload()` — sets stopped flags → signals old children → sleep 400ms → clears flags → restarts

## Build commands

```bash
# Rust
cd rust-core
cargo build --release        # debug: cargo build
cargo test                   # run 27 unit tests

# Go
cd go-daemon
go build -o smarttype-daemon .
go vet ./...

# Both at once
./scripts/build-all.sh
```

## Runtime files

| Path | Purpose |
|------|---------|
| `/dev/input/event*` | Keyboard devices (read by hook) |
| `/dev/uinput` | Virtual keyboard device (written by hook) |
| `/tmp/smarttype-popup.sock` | Unix socket between hook and popup |
| `~/.config/smarttype/config.yaml` | User configuration |
| `~/.local/share/smarttype/learned_words.json` | Learned word frequencies |
| `~/.config/systemd/user/smarttype.service` | Systemd unit |
| `/etc/udev/rules.d/99-smarttype.rules` | Input device permissions |

## Testing

```bash
cd rust-core
cargo test
# 27 tests: autocomplete, engine, dictionary, smart_punctuation, config, lib integration
```

All tests are pure unit tests — no hardware access needed.
