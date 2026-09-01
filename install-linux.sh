#!/bin/bash

set -e

# Prebuilt-package installer: installs the binary produced by the CI release
# workflow (https://github.com/emachado88/sleep-on-time/actions/workflows/release.yml).
# Run this from an extracted release tarball, where the binary, the desktop
# file and the assets/ directory all live next to this script.
# Source checkouts should use build-install.sh instead.

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"

if [ ! -f "$SCRIPT_DIR/sleep-on-time" ]; then
    echo "Error: sleep-on-time binary not found next to this script."
    echo "This installer is for prebuilt release packages."
    echo "If you are in a source checkout, run ./build-install.sh instead."
    exit 1
fi

echo "Installing binary to /usr/local/bin..."
sudo install -Dm755 "$SCRIPT_DIR/sleep-on-time" /usr/local/bin/sleep-on-time

echo "Installing desktop file..."
if [ -f "$SCRIPT_DIR/sleep-on-time.desktop" ]; then
    sudo install -Dm644 "$SCRIPT_DIR/sleep-on-time.desktop" /usr/share/applications/sleep-on-time.desktop
    echo "Desktop file installed to /usr/share/applications/"
else
    echo "Warning: sleep-on-time.desktop not found, skipping desktop file installation."
fi

echo "Installing icon..."
if [ -f "$SCRIPT_DIR/assets/icon-light.svg" ]; then
    sudo install -Dm644 "$SCRIPT_DIR/assets/icon-light.svg" /usr/share/pixmaps/sleepontime.svg
    sudo install -Dm644 "$SCRIPT_DIR/assets/icon-light.svg" /usr/share/icons/hicolor/scalable/apps/sleepontime.svg
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