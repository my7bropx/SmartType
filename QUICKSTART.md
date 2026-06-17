# Quick Start

## Install in 4 steps

```bash
# 1. Install build tools and system libs
./scripts/install-deps.sh

# 2. Build everything and install to /usr/local/bin
sudo ./scripts/install.sh

# 3. Log out and back in  (input group permission)

# 4. Start the service
systemctl --user start smarttype
```

## Try it immediately

Open any text field and start typing. After two or more characters a suggestion bar appears above the cursor.

```
Type:   questi
Bar:    [Tab] question  [2] questions  [3] questionnaire
```

Press `Tab` to accept "question". Press `2` for "questions". Keep typing to ignore.

Type a misspelled word and press Space:

```
Type:   recieve<Space>
Bar:    [Tab] receive  [2] receiver  [3] received
```

Press `Tab` to replace it. Press `Backspace` to get "recieve" back in the buffer for manual editing.

## Key bindings

| Key | What it does |
|-----|-------------|
| `Tab` | Accept suggestion #1 |
| `1`–`5` | Accept suggestion #1–#5 |
| `Backspace` (after space) | Undo — restore the previous word for re-editing |
| Any letter | Dismiss suggestions, start a new word |
| `Esc` / arrows / `Home` / `End` | Clear suggestion bar, reset context |

## Service commands

```bash
systemctl --user start   smarttype   # start
systemctl --user stop    smarttype   # stop
systemctl --user restart smarttype   # restart
systemctl --user status  smarttype   # check
journalctl --user -u smarttype -f    # live logs
```

## Configuration

Edit `~/.config/smarttype/config.yaml`:

```yaml
enabled: true
autocorrect: true
smart_punctuation: true
min_word_length: 2

custom_typos:
  hte: the
  becuase: because
  myabbrev: "my full phrase"
```

Restart the service after editing:
```bash
systemctl --user restart smarttype
```

## Smart punctuation

| You type | Result |
|----------|--------|
| `"hello"` | `"hello"` |
| `'world'` | `'world'` |
| `--` | `—` |
| `...` | `…` |
| `don't` | `don't` |
