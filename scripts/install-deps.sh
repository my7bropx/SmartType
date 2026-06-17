#!/bin/bash
#
# SmartType Dependency Installation Script
# Installs all required dependencies for building SmartType
#

set -e

echo "================================"
echo " SmartType Dependency Installer"
echo "================================"
echo ""

# Check if running as root
if [ "$EUID" -ne 0 ]; then
    echo "This script must be run as root (use sudo)"
    exit 1
fi

# Detect distribution
if [ -f /etc/os-release ]; then
    . /etc/os-release
    DISTRO=$ID
else
    echo "Cannot detect Linux distribution"
    exit 1
fi

echo "Detected distribution: $DISTRO"
echo ""

# Install dependencies based on distribution
case $DISTRO in
    debian|ubuntu|kali)
        echo "Installing dependencies for Debian/Ubuntu/Kali..."
        apt-get update
        apt-get install -y \
            build-essential \
            curl \
            git \
            pkg-config \
            libevdev-dev \
            libudev-dev \
            libx11-dev \
            libxdo-dev \
            libxkbcommon-dev \
            python3 \
            python3-pip \
            python3-pyqt5 \
            golang-go \
            cargo \
            rustc
        ;;

    arch|manjaro)
        echo "Installing dependencies for Arch/Manjaro..."
        pacman -Syu --noconfirm \
            base-devel \
            curl \
            git \
            pkg-config \
            libevdev \
            systemd-libs \
            libx11 \
            xdotool \
            libxkbcommon \
            python \
            python-pip \
            python-pyqt5 \
            go \
            rust \
            cargo
        ;;

    fedora|rhel|centos)
        echo "Installing dependencies for Fedora/RHEL/CentOS..."
        dnf install -y \
            gcc \
            gcc-c++ \
            make \
            curl \
            git \
            pkg-config \
            libevdev-devel \
            systemd-devel \
            libX11-devel \
            xdotool \
            libxkbcommon-devel \
            python3 \
            python3-pip \
            python3-qt5 \
            golang \
            rust \
            cargo
        ;;

    *)
        echo "Unsupported distribution: $DISTRO"
        echo "Please install dependencies manually:"
        echo "  - Rust (cargo, rustc)"
        echo "  - Go 1.21+"
        echo "  - Python 3.9+"
        echo "  - PyQt5"
        echo "  - libevdev, libudev, libx11, libxdo"
        exit 1
        ;;
esac

echo ""
echo "Installing Python dependencies..."
pipx install pip
pipx install PyQt5 PyYAML psutil

echo ""
echo "Verifying Rust installation..."
if ! command -v cargo &> /dev/null; then
    echo "Rust not found, installing via rustup..."
    curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y
    source $HOME/.cargo/env
fi

rustc --version
cargo --version

echo ""
echo "Verifying Go installation..."
go version

echo ""
echo "Verifying Python installation..."
python3 --version
pipx --version
pip3 --version
echo ""
echo "================================"
echo " All dependencies installed!"
echo "================================"
echo ""
echo "Next steps:"
echo "  1. Run ./scripts/build-all.sh to build SmartType"
echo "  2. Run sudo ./scripts/install.sh to install system-wide"
echo ""
