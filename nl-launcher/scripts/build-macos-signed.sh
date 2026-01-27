#!/bin/bash

# =============================================================================
# NL Mirror - macOS Signed Build Script
# =============================================================================
# 
# Sử dụng:
#   ./scripts/build-macos-signed.sh [arm64|x64|universal]
#
# Environment variables (xem .env.example để biết chi tiết):
#   APPLE_SIGNING_IDENTITY  - Signing identity (hoặc "-" cho ad-hoc)
#   APPLE_ID                - Apple ID email (cho notarization)
#   APPLE_PASSWORD          - App-specific password (cho notarization)
#   APPLE_TEAM_ID           - Team ID (cho notarization)
#
# Tham khảo: https://tauri.app/distribute/sign/macos/
#
# =============================================================================

set -e

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

# Get script directory
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_DIR="$(dirname "$SCRIPT_DIR")"

cd "$PROJECT_DIR"

# Load .env file if exists
if [ -f "$PROJECT_DIR/.env" ]; then
    echo -e "${BLUE}📄 Loading environment from .env file...${NC}"
    set -a
    source "$PROJECT_DIR/.env"
    set +a
fi

# Parse arguments
ARCH="${1:-arm64}"

case "$ARCH" in
    arm64)
        TARGET="aarch64-apple-darwin"
        ;;
    x64)
        TARGET="x86_64-apple-darwin"
        ;;
    universal)
        TARGET="universal-apple-darwin"
        ;;
    *)
        echo -e "${RED}❌ Invalid architecture: $ARCH${NC}"
        echo "Usage: $0 [arm64|x64|universal]"
        exit 1
        ;;
esac

echo -e "${BLUE}━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━${NC}"
echo -e "${BLUE}🚀 NL Mirror - macOS Build${NC}"
echo -e "${BLUE}━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━${NC}"
echo ""

# Display signing configuration
if [ -n "$APPLE_SIGNING_IDENTITY" ]; then
    if [ "$APPLE_SIGNING_IDENTITY" = "-" ]; then
        echo -e "${YELLOW}🔐 Signing: Ad-hoc (app will need manual approval in System Settings)${NC}"
    else
        echo -e "${GREEN}🔐 Signing: $APPLE_SIGNING_IDENTITY${NC}"
    fi
else
    echo -e "${YELLOW}🔐 Signing: None (using ad-hoc)${NC}"
    export APPLE_SIGNING_IDENTITY="-"
fi

# Display notarization configuration
# Tauri sẽ tự động notarize nếu có các biến môi trường này
if [ -n "$APPLE_ID" ] && [ -n "$APPLE_PASSWORD" ] && [ -n "$APPLE_TEAM_ID" ]; then
    echo -e "${GREEN}📋 Notarization: Enabled (Tauri will handle automatically)${NC}"
elif [ -n "$APPLE_API_KEY" ] && [ -n "$APPLE_API_ISSUER" ] && [ -n "$APPLE_API_KEY_PATH" ]; then
    echo -e "${GREEN}📋 Notarization: Enabled via App Store Connect API${NC}"
else
    echo -e "${YELLOW}📋 Notarization: Disabled (set APPLE_ID, APPLE_PASSWORD, APPLE_TEAM_ID to enable)${NC}"
fi

echo ""
echo -e "${BLUE}📦 Target: $TARGET${NC}"
echo ""

# Update binaries first
echo -e "${BLUE}🔄 Updating binaries...${NC}"
sh scripts/update-binaries.sh

# Build with Tauri
# Tauri sẽ tự động:
# - Ký app với APPLE_SIGNING_IDENTITY
# - Notarize nếu có credentials (APPLE_ID/APPLE_PASSWORD/APPLE_TEAM_ID)
echo -e "${BLUE}🔨 Building application...${NC}"
npm run tauri:ci build -- --target "$TARGET"

# Find the built app
APP_PATH="$PROJECT_DIR/src-tauri/target/$TARGET/release/bundle/macos/NL Mirror.app"
DMG_PATH="$PROJECT_DIR/src-tauri/target/$TARGET/release/bundle/dmg"

if [ ! -d "$APP_PATH" ]; then
    # Try without target subdirectory for default builds
    APP_PATH="$PROJECT_DIR/src-tauri/target/release/bundle/macos/NL Mirror.app"
    DMG_PATH="$PROJECT_DIR/src-tauri/target/release/bundle/dmg"
fi

if [ -d "$APP_PATH" ]; then
    echo ""
    echo -e "${GREEN}✅ Build completed successfully!${NC}"
    echo -e "${BLUE}📱 App: $APP_PATH${NC}"
    
    # Remove quarantine attribute for local testing (ad-hoc signing)
    if [ "$APPLE_SIGNING_IDENTITY" = "-" ]; then
        echo -e "${BLUE}🔓 Removing quarantine attribute for local testing...${NC}"
        xattr -cr "$APP_PATH" 2>/dev/null || true
    fi
    
    # Show DMG location if exists
    if [ -d "$DMG_PATH" ]; then
        echo ""
        echo -e "${BLUE}💿 DMG:${NC}"
        ls -la "$DMG_PATH"/*.dmg 2>/dev/null || true
    fi
else
    echo -e "${RED}❌ Build failed - app not found${NC}"
    exit 1
fi

echo ""
echo -e "${BLUE}━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━${NC}"
echo -e "${GREEN}🎉 Done!${NC}"
echo -e "${BLUE}━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━${NC}"
