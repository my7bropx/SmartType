# Installation

## Prerequisites

| Tool | Minimum | Notes |
|------|---------|-------|
| Linux kernel | 4.0+ | evdev + uinput required |
| X11 | any | Wayland not yet supported |
| Rust / cargo | 1.70+ | via rustup |
| Go | 1.21+ | via apt or go.dev |

## Step 1 — Install build dependencies

```bash
./scripts/install-deps.sh
```

This installs Rust (via rustup if absent), checks for Go, installs system libraries (`libx11-dev`, `libxrandr-dev`, `libxtst-dev`), and adds your user to the `input` group.

For Kali / Debian / Ubuntu only. On other distros install the equivalents manually:

```bash
# Arch
sudo pacman -S rust go libx11 libxrandr libxtst

# Fedora
sudo dnf install rust cargo golang libX11-devel libXrandr-devel libXtst-devel
```

## Step 2 — Build and install

```bash
sudo ./scripts/install.sh
```

This script:
1. Runs `build-all.sh` (cargo release build + go build)
2. Copies binaries to `/usr/local/bin/`
3. Writes `/etc/udev/rules.d/99-smarttype.rules` (input device permissions)
4. Installs a systemd user unit at `~/.config/systemd/user/smarttype.service`
5. Enables the unit (`systemctl --user enable smarttype`)

## Step 3 — Re-login

The installer adds your user to the `input` group. This only takes effect after logging out and back in (or running `newgrp input` in the current shell).

```bash
# Verify group membership
groups | grep input
```

## Step 4 — Start

```bash
systemctl --user start smarttype
systemctl --user status smarttype
```

The service auto-starts on every login because the unit was enabled in step 2.

## Build only (no install)

```bash
./scripts/build-all.sh
```

Produces:
- `rust-core/target/release/smarttype-hook`
- `rust-core/target/release/smarttype-popup`
- `go-daemon/smarttype-daemon`

## Manual install (skip sudo)

If you prefer to install to a user-writable location:

```bash
./scripts/build-all.sh

mkdir -p ~/.local/bin
cp rust-core/target/release/smarttype-hook   ~/.local/bin/
cp rust-core/target/release/smarttype-popup  ~/.local/bin/
cp go-daemon/smarttype-daemon                ~/.local/bin/
```

Then edit `~/.config/systemd/user/smarttype.service` to point `ExecStart` to `~/.local/bin/smarttype-daemon`.

## Uninstall

```bash
systemctl --user stop smarttype
systemctl --user disable smarttype

sudo rm -f /usr/local/bin/smarttype-hook \
           /usr/local/bin/smarttype-popup \
           /usr/local/bin/smarttype-daemon

sudo rm -f /etc/udev/rules.d/99-smarttype.rules
sudo udevadm control --reload-rules

rm -f ~/.config/systemd/user/smarttype.service
systemctl --user daemon-reload

# Optionally remove learned words
rm -rf ~/.local/share/smarttype
```

## Troubleshooting

**Service fails to start — "No keyboard devices found"**

Your user is not in the `input` group yet. Log out and back in, or:
```bash
newgrp input
systemctl --user restart smarttype
```

**Popup does not appear**

Check that `DISPLAY` is set:
```bash
echo $DISPLAY          # should print :0 or similar
systemctl --user restart smarttype
```

**Checking logs**

```bash
journalctl --user -u smarttype -f
# or run the daemon directly to see output:
RUST_LOG=info smarttype-daemon
```
