#!/usr/bin/env bash
set -euo pipefail

echo "==> Installing SmartType build dependencies..."

# ── Rust ──────────────────────────────────────────────────────────────────────
if command -v cargo &>/dev/null; then
    echo "  Rust $(rustc --version) already installed."
else
    echo "  Installing Rust via rustup..."
    curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y --default-toolchain stable
    # shellcheck source=/dev/null
    source "$HOME/.cargo/env"
fi

# ── Go ────────────────────────────────────────────────────────────────────────
if command -v go &>/dev/null; then
    echo "  Go $(go version) already installed."
else
    echo "  Go not found. Install it from https://go.dev/dl/ or run:"
    echo "    sudo apt install golang-go"
    exit 1
fi

# ── System libraries (Debian / Ubuntu / Kali) ─────────────────────────────────
if command -v apt-get &>/dev/null; then
    echo "  Installing system packages via apt..."
    sudo apt-get update -qq
    sudo apt-get install -y --no-install-recommends \
        build-essential \
        pkg-config \
        libx11-dev \
        libxrandr-dev \
        libxtst-dev
else
    echo "  Non-Debian system detected. Ensure you have:"
    echo "    build-essential / base-devel"
    echo "    libx11-dev / libX11-devel"
fi

# ── input group ───────────────────────────────────────────────────────────────
if ! id -nG "$USER" | grep -qw input; then
    echo "  Adding $USER to 'input' group..."
    sudo usermod -aG input "$USER"
    echo "  NOTE: Log out and back in (or run: newgrp input) before starting SmartType."
else
    echo "  $USER is already in the 'input' group."
fi

echo ""
echo "Dependencies ready. Run: ./scripts/install.sh"
