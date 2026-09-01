#!/bin/bash

set -e

# Check if Rust/Cargo is installed
if ! command -v cargo &> /dev/null; then
    echo "Error: cargo is not installed. Please install Rust first."
    echo "Visit https://rustup.rs for installation instructions."
    exit 1
fi

echo "Building sleep-on-time (release)..."
cargo build --release

echo "Installing binary to /usr/local/bin..."
sudo install -Dm755 target/release/sleep-on-time /usr/local/bin/sleep-on-time

echo "Installing desktop file..."
if [ -f "sleep-on-time.desktop" ]; then
    sudo install -Dm644 sleep-on-time.desktop /usr/share/applications/sleep-on-time.desktop
    echo "Desktop file installed to /usr/share/applications/"
else
    echo "Warning: sleep-on-time.desktop not found, skipping desktop file installation."
fi

echo "Installing icon..."
if [ -f "assets/icon-light.svg" ]; then
    sudo install -Dm644 assets/icon-light.svg /usr/share/pixmaps/sleepontime.svg
    sudo install -Dm644 assets/icon-light.svg /usr/share/icons/hicolor/scalable/apps/sleepontime.svg
    echo "Icon installed to /usr/share/pixmaps/ and hicolor theme"

    # Update icon cache for KDE
    if command -v kbuildsycoca6 &> /dev/null; then
        kbuildsycoca6 --noincremental 2>/dev/null || true
    elif command -v kbuildsycoca5 &> /dev/null; then
        kbuildsycoca5 --noincremental 2>/dev/null || true
    fi

    # Update GTK icon cache if available
    if command -v gtk-update-icon-cache &> /dev/null; then
        sudo gtk-update-icon-cache /usr/share/icons/hicolor/ 2>/dev/null || true
    fi
else
    echo "Warning: assets/icon-light.svg not found, skipping icon installation."
fi

echo "Installation complete!"
echo "You can now run 'sleep-on-time' from the terminal or find it in your applications menu."
