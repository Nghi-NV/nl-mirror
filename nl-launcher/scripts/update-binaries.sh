#!/bin/bash

# Determine paths
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(cd "$SCRIPT_DIR/../.." && pwd)"
LAUNCHER_ROOT="$PROJECT_ROOT/nl-launcher"
BINARIES_DIR="$LAUNCHER_ROOT/src-tauri/binaries"
NL_ANDROID_DIR="$PROJECT_ROOT/nl-android"
NL_HOST_DIR="$PROJECT_ROOT/nl-host"

# Detect Architecture
ARCH=$(uname -m)
if [ "$ARCH" == "arm64" ]; then
    ARCH="aarch64"
fi
HOST_TRIPLE="$ARCH-apple-darwin"

# Target files
TARGET_APK="$BINARIES_DIR/nl-mirror.apk"
TARGET_HOST="$BINARIES_DIR/nl-host-$HOST_TRIPLE"

# Ensure binaries directory exists
mkdir -p "$BINARIES_DIR"

# --- Function to update nl-mirror.apk ---
update_apk() {
    echo "Checking nl-mirror.apk..."
    local source_apk="$NL_ANDROID_DIR/build/outputs/apk/debug/nl-mirror-debug.apk"
    local needs_build=false

    # Check if target exists
    if [ ! -f "$TARGET_APK" ]; then
        echo "  -> nl-mirror.apk missing."
        needs_build=true
    fi

    # Check if we need to rebuild based on source changes (simple check on src dir)
    # Ideally checking against the build output timestamp vs src timestamp, 
    # but here we check if source is newer than target.
    # A simpler approach: Let Gradle handle "up-to-date" checks. We just run build.
    # However, running gradle on every dev start might be slow.
    # We will check if the source directory has any file newer than the target apk.
    
    if [ "$needs_build" = false ]; then
        if [ -n "$(find "$NL_ANDROID_DIR/src" -type f -newer "$TARGET_APK" 2>/dev/null | head -n 1)" ]; then
             echo "  -> nl-android source is newer than binary."
             needs_build=true
        fi
    fi

    if [ "$needs_build" = true ]; then
        echo "  -> Building nl-android..."
        (cd "$NL_ANDROID_DIR" && ./gradlew assembleDebug)
        
        if [ $? -eq 0 ]; then
            echo "  -> Copying APK..."
            cp "$source_apk" "$TARGET_APK"
            echo "  -> Updated nl-mirror.apk"
        else
            echo "  -> Build failed!"
            exit 1
        fi
    else
        echo "  -> nl-mirror.apk is up to date."
    fi
}

# --- Function to update nl-host ---
update_host() {
    echo "Checking nl-host..."
    local source_bin="$NL_HOST_DIR/target/release/nl-host"
    local needs_build=false

    if [ ! -f "$TARGET_HOST" ]; then
        echo "  -> nl-host binary missing."
        needs_build=true
    fi

    if [ "$needs_build" = false ]; then
        if [ -n "$(find "$NL_HOST_DIR/src" -type f -newer "$TARGET_HOST" 2>/dev/null | head -n 1)" ]; then
             echo "  -> nl-host source is newer than binary."
             needs_build=true
        fi
    fi

    if [ "$needs_build" = true ]; then
        echo "  -> Building nl-host..."
        (cd "$NL_HOST_DIR" && cargo build --release)
        
        if [ $? -eq 0 ]; then
            echo "  -> Copying binary..."
            cp "$source_bin" "$TARGET_HOST"
            echo "  -> Updated nl-host"
        else
            echo "  -> Build failed!"
            exit 1
        fi
    else
        echo "  -> nl-host is up to date."
    fi
}

update_apk
update_host

echo "Binary check complete."
