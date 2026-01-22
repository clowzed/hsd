#!/bin/bash
set -e

APP_NAME="Honest Sign Scanner"
VERSION="0.1.0"
DMG_NAME="HonestSignScanner-${VERSION}"

APP_DIR="target/release/${APP_NAME}.app"
DMG_DIR="target/release/dmg"
DMG_FILE="target/release/${DMG_NAME}.dmg"

if [ ! -d "${APP_DIR}" ]; then
    echo "Error: App bundle not found at ${APP_DIR}"
    echo "Run ./scripts/build-macos.sh first"
    exit 1
fi

echo "Creating DMG..."

# Create temporary DMG directory
rm -rf "${DMG_DIR}"
mkdir -p "${DMG_DIR}"

# Copy app bundle
cp -R "${APP_DIR}" "${DMG_DIR}/"

# Create symlink to Applications
ln -s /Applications "${DMG_DIR}/Applications"

# Create DMG
rm -f "${DMG_FILE}"

# Check if create-dmg is available
if command -v create-dmg &> /dev/null; then
    create-dmg \
        --volname "${APP_NAME}" \
        --volicon "assets/icon.icns" \
        --window-pos 200 120 \
        --window-size 600 400 \
        --icon-size 100 \
        --icon "${APP_NAME}.app" 150 185 \
        --hide-extension "${APP_NAME}.app" \
        --app-drop-link 450 185 \
        "${DMG_FILE}" \
        "${DMG_DIR}"
else
    # Fallback: use hdiutil
    echo "Note: create-dmg not found, using hdiutil (simpler DMG)"
    hdiutil create -volname "${APP_NAME}" \
        -srcfolder "${DMG_DIR}" \
        -ov -format UDZO \
        "${DMG_FILE}"
fi

# Cleanup
rm -rf "${DMG_DIR}"

echo ""
echo "DMG created: ${DMG_FILE}"
echo ""
echo "To install create-dmg for better DMG layout:"
echo "  brew install create-dmg"
