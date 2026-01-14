#!/bin/bash
# scripts/fetch-binaries.sh
# Automates fetching ADB binaries for macOS, Linux, and Windows
# and renames them for Tauri sidecar support.

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
ROOT_DIR="$(cd "$SCRIPT_DIR/../.." && pwd)"
BIN_DIR="$ROOT_DIR/nl-launcher/src-tauri/binaries"
echo "Binaries Directory: $BIN_DIR"
mkdir -p "$BIN_DIR"

# URLs for Platform Tools
MAC_URL="https://dl.google.com/android/repository/platform-tools-latest-darwin.zip"
LINUX_URL="https://dl.google.com/android/repository/platform-tools-latest-linux.zip"
WIN_URL="https://dl.google.com/android/repository/platform-tools-latest-windows.zip"

download_adb() {
    PLATFORM=$1
    URL=$2
    TRIPLES=("${@:3}") # Array of triples for this platform

    echo "------------------------------------------------"
    echo "Processing $PLATFORM..."
    
    # Create temp dir
    TMP_DIR=$(mktemp -d)
    
    echo "Downloading $URL..."
    curl -L -o "$TMP_DIR/tools.zip" "$URL"
    
    echo "Unzipping..."
    unzip -q "$TMP_DIR/tools.zip" -d "$TMP_DIR"
    
    ADB_SRC="$TMP_DIR/platform-tools/adb"
    if [ "$PLATFORM" == "Windows" ]; then
        ADB_SRC="$TMP_DIR/platform-tools/adb.exe"
    fi
    
    if [ ! -f "$ADB_SRC" ]; then
        echo "Error: ADB binary not found in downloaded zip for $PLATFORM"
        return
    fi
    
    for TRIPLE in "${TRIPLES[@]}"; do
        TARGET_NAME="adb-$TRIPLE"
        if [ "$PLATFORM" == "Windows" ]; then
            TARGET_NAME="$TARGET_NAME.exe"
        fi
        
        echo "Copying to $TARGET_NAME"
        cp "$ADB_SRC" "$BIN_DIR/$TARGET_NAME"
        chmod +x "$BIN_DIR/$TARGET_NAME"
    done
    
    rm -rf "$TMP_DIR"
    echo "$PLATFORM Done."
}

# Check for platform argument
TARGET_PLATFORM=$1

# 1. macOS (Intel + ARM)
if [ -z "$TARGET_PLATFORM" ] || [ "$TARGET_PLATFORM" == "macos" ]; then
    download_adb "macOS" "$MAC_URL" "x86_64-apple-darwin" "aarch64-apple-darwin"
fi

# 2. Linux (x64)
if [ -z "$TARGET_PLATFORM" ] || [ "$TARGET_PLATFORM" == "linux" ]; then
    download_adb "Linux" "$LINUX_URL" "x86_64-unknown-linux-gnu"
fi

# 3. Windows (x64)
if [ -z "$TARGET_PLATFORM" ] || [ "$TARGET_PLATFORM" == "windows" ]; then
    download_adb "Windows" "$WIN_URL" "x86_64-pc-windows-msvc"
fi

echo "------------------------------------------------"
echo "Setup Complete. ADB Binaries are ready."
echo "NOTE: You still need to compile 'nl-host' for each platform and place it in $BIN_DIR manually or via CI."
ls -lh "$BIN_DIR"
