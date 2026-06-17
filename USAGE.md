# Usage Guide

## Suggestion bar

The popup appears just above the cursor whenever there are completions to show. It stays out of the way when there are none.

```
┌─────────────────────────────────────────────────┐
│ Tab question │ 2 questions │ 3 questionnaire … │
└─────────────────────────────────────────────────┘
```

- The first suggestion has a highlighted background — it is the highest-ranked match.
- Colors shift from yellow → green → blue → purple → teal for suggestions 1–5.
- Numbers shown correspond directly to the `1`–`5` keys.

## Typing flow

### Autocomplete (prefix matching)

Suggestions appear after you type 2+ characters of a word. The bar shows prefix matches first, then near-matches (edit distance 1) to catch partial misspellings while you are still in the middle of a word.

Accept a suggestion:
- `Tab` — inserts the top suggestion, erasing what you typed so far
- `1`–`5` — inserts the nth suggestion

### Post-space correction (typo detection)

When you press `Space` after a word that is not in the dictionary, SmartType shows correction suggestions. The word and the space are still in the buffer — nothing has been retyped yet.

- `Tab` / `1`–`5` — erase the original word + space, type the correction + space
- `Backspace` — erase the space that was typed and restore the original word to the buffer so you can fix it manually
- Any letter — dismisses corrections and starts the next word normally

### Learning

Every word you accept via `Tab` or `1`–`5` is added to the learned-words store at `~/.local/share/smarttype/learned_words.json`. Learned words appear earlier in suggestions. The frequency count rises each time you use a word, so your most-used vocabulary floats to the top.

## Configuration file

`~/.config/smarttype/config.yaml` — created automatically on first run.

```yaml
enabled: true               # master switch
autocorrect: true           # rule-based typo correction on Space
smart_punctuation: true     # curly quotes, em-dash, ellipsis
min_word_length: 2          # skip correction for very short words

custom_typos:               # your own correction rules
  hte: the
  becuase: because
  kali: "Kali Linux"

hotkey: "Super+Shift+A"     # reserved; not yet active
```

After editing, restart the service:
```bash
systemctl --user restart smarttype
```

### Per-application settings

The config supports per-application overrides, but the current hook reads all keyboard input at the device level (independent of the active window), so application-specific rules are not enforced at runtime yet. The fields are parsed and stored for a future release.

## Smart punctuation

Runs on every word boundary (Space, Enter):

| Input | Output | Rule |
|-------|--------|------|
| `"text"` | `"text"` | Straight double quotes → curly |
| `'text'` | `'text'` | Straight single quotes → curly |
| `don't` | `don't` | Contraction apostrophe → right single quote |
| ` -- ` | ` — ` | Spaced double hyphen → em dash |
| `...` | `…` | Three dots → ellipsis character |

Disable globally:
```yaml
smart_punctuation: false
```

## Autocorrect dictionary

Built-in entries cover the most common English typos (`teh`, `recieve`, `becuase`, etc.). Case is preserved:

| Typed | Corrected |
|-------|-----------|
| `teh` | `the` |
| `Teh` | `The` |
| `TEH` | `THE` |

Add your own via `custom_typos` in the config.

## Debugging

Run with verbose logging:
```bash
systemctl --user stop smarttype
RUST_LOG=info smarttype-daemon
```

Or check journal output:
```bash
journalctl --user -u smarttype -f
```

Common log events:
- `Keyboard: /dev/input/eventN` — a device was found and opened
- `Stream started` — listening for key events
- `Autocorrect: X → Y` — a typo was corrected on Space
- `Device removed` — keyboard was unplugged; will rediscover in 2s
- `Hook exited unexpectedly, restarting in 2s` — crash recovery kicked in

## Known limitations

- X11 only — no Wayland support
- Suggestions track cursor via mouse pointer position (`XQueryPointer`), not caret position. In some editors the bar may appear slightly offset.
- Per-application rules are parsed but not enforced at runtime yet.
- Only the Latin alphabet is handled in the key table; non-ASCII input passes through uncorrected.
