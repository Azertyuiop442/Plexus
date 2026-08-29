#!/usr/bin/env bash
set -euo pipefail

echo "======================================================"
echo "          Installing Plexus                           "
echo "======================================================"

OS="$(uname -s)"

echo "-> Checking Nerd Font installation..."
install_nerd_font_macos() {
    if command -v brew >/dev/null 2>&1; then
        echo "   Installing JetBrainsMono Nerd Font via Homebrew..."
        brew install --cask font-jetbrains-mono-nerd-font >/dev/null 2>&1 || true
    else
        echo "   Downloading JetBrainsMono Nerd Font to ~/Library/Fonts/..."
        mkdir -p ~/Library/Fonts
        curl -fsSL "https://github.com/ryanoasis/nerd-fonts/releases/latest/download/JetBrainsMono.tar.xz" -o /tmp/JetBrainsMono.tar.xz
        tar -xf /tmp/JetBrainsMono.tar.xz -C ~/Library/Fonts/ "*.ttf" 2>/dev/null || true
        rm -f /tmp/JetBrainsMono.tar.xz
    fi
}

install_nerd_font_linux() {
    echo "   Downloading JetBrainsMono Nerd Font to ~/.local/share/fonts/..."
    mkdir -p ~/.local/share/fonts
    curl -fsSL "https://github.com/ryanoasis/nerd-fonts/releases/latest/download/JetBrainsMono.tar.xz" -o /tmp/JetBrainsMono.tar.xz
    tar -xf /tmp/JetBrainsMono.tar.xz -C ~/.local/share/fonts/ "*.ttf" 2>/dev/null || true
    rm -f /tmp/JetBrainsMono.tar.xz
    if command -v fc-cache >/dev/null 2>&1; then
        fc-cache -f ~/.local/share/fonts >/dev/null 2>&1 || true
    fi
}

if [[ "$OS" == "Darwin" ]]; then
    if ! fc-list 2>/dev/null | grep -qi "Nerd Font" && ! ls ~/Library/Fonts/*Nerd* >/dev/null 2>&1 && ! ls /Library/Fonts/*Nerd* >/dev/null 2>&1; then
        install_nerd_font_macos
    else
        echo "   Nerd Font already detected."
    fi
elif [[ "$OS" == "Linux" ]]; then
    if ! fc-list 2>/dev/null | grep -qi "Nerd Font" && ! ls ~/.local/share/fonts/*Nerd* >/dev/null 2>&1; then
        install_nerd_font_linux
    else
        echo "   Nerd Font already detected."
    fi
fi

echo "-> Checking Rust & Cargo..."
if ! command -v cargo >/dev/null 2>&1; then
    echo "   Cargo not found. Installing Rust via rustup..."
    curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y
    source "$HOME/.cargo/env" 2>/dev/null || true
fi

DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
cd "$DIR"

echo "-> Compiling release binaries..."
cargo build --release

if [[ "$OS" == "Darwin" ]]; then
    xattr -c target/release/cc-mux 2>/dev/null || true
    codesign -s - -f target/release/cc-mux 2>/dev/null || true
    if [ -f target/release/cc-dashboard ]; then
        xattr -c target/release/cc-dashboard 2>/dev/null || true
        codesign -s - -f target/release/cc-dashboard 2>/dev/null || true
    fi
fi

echo "-> Deploying atomically to ~/.commandcode/bin/..."
mkdir -p ~/.commandcode/bin ~/.local/bin

cp target/release/cc-mux ~/.commandcode/bin/.cc-mux.tmp.$$
chmod 755 ~/.commandcode/bin/.cc-mux.tmp.$$
mv -f ~/.commandcode/bin/.cc-mux.tmp.$$ ~/.commandcode/bin/cc-mux
ln -sf cc-mux ~/.commandcode/bin/plexus

if [ -f target/release/cc-dashboard ]; then
    cp target/release/cc-dashboard ~/.commandcode/bin/.cc-dashboard.tmp.$$
    chmod 755 ~/.commandcode/bin/.cc-dashboard.tmp.$$
    mv -f ~/.commandcode/bin/.cc-dashboard.tmp.$$ ~/.commandcode/bin/cc-dashboard
fi

ln -sf ~/.commandcode/bin/cc-mux ~/.local/bin/plexus
ln -sf ~/.commandcode/bin/cc-mux ~/.local/bin/cc-mux
ln -sf ~/.commandcode/bin/cc-dashboard ~/.local/bin/cc-dashboard 2>/dev/null || true

echo "======================================================"
echo " [OK] Plexus successfully installed and synchronized! "
echo "    - Standalone: Run 'plexus' or 'cc-mux'            "
echo "    - In Command Code: Type '/dashboard'              "
echo "======================================================"
