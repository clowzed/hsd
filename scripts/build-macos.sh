#!/bin/bash
set -e

APP_NAME="Honest Sign Scanner"
BUNDLE_ID="com.honest-sign.scanner"
VERSION="0.1.0"

# Build for the current architecture
echo "Building for current architecture..."
cargo build --release

# Create app bundle structure
APP_DIR="target/release/${APP_NAME}.app"
CONTENTS_DIR="${APP_DIR}/Contents"
MACOS_DIR="${CONTENTS_DIR}/MacOS"
RESOURCES_DIR="${CONTENTS_DIR}/Resources"

echo "Creating app bundle at ${APP_DIR}..."
rm -rf "${APP_DIR}"
mkdir -p "${MACOS_DIR}"
mkdir -p "${RESOURCES_DIR}"

# Copy binary
cp "target/release/honest-sign-scanner" "${MACOS_DIR}/"

# Copy Info.plist
cp "macos/Info.plist" "${CONTENTS_DIR}/"

# Copy icon if exists
if [ -f "assets/icon.icns" ]; then
    cp "assets/icon.icns" "${RESOURCES_DIR}/AppIcon.icns"
fi

# Create PkgInfo
echo "APPL????" > "${CONTENTS_DIR}/PkgInfo"

# Sign the app (ad-hoc signing for local development)
echo "Signing app bundle..."
codesign --force --deep --sign - "${APP_DIR}"

# Apply entitlements
if [ -f "macos/entitlements.plist" ]; then
    echo "Applying entitlements..."
    codesign --force --entitlements "macos/entitlements.plist" --sign - "${APP_DIR}"
fi

echo ""
echo "App bundle created: ${APP_DIR}"
echo ""
echo "To run the app:"
echo "  open \"${APP_DIR}\""
echo ""
echo "To create DMG (requires create-dmg):"
echo "  ./scripts/create-dmg.sh"
