#!/bin/bash
# Bump version in all project files.
# Usage: ./scripts/bump-version.sh 0.3.0

set -euo pipefail

if [ $# -ne 1 ]; then
  echo "Usage: $0 <version>"
  echo "Example: $0 0.3.0"
  exit 1
fi

VERSION="$1"
ROOT="$(cd "$(dirname "$0")/.." && pwd)"

# Validate version format
if ! echo "$VERSION" | grep -qE '^[0-9]+\.[0-9]+\.[0-9]+$'; then
  echo "Error: version must be in semver format (e.g., 0.3.0)"
  exit 1
fi

echo "Bumping version to $VERSION..."

# 1. package.json
sed -i '' "s/\"version\": \".*\"/\"version\": \"$VERSION\"/" "$ROOT/package.json"

# 2. src-tauri/Cargo.toml (only the first version line)
sed -i '' "0,/^version = \".*\"/s//version = \"$VERSION\"/" "$ROOT/src-tauri/Cargo.toml"

# 3. src-tauri/tauri.conf.json
sed -i '' "s/\"version\": \".*\"/\"version\": \"$VERSION\"/" "$ROOT/src-tauri/tauri.conf.json"

echo "Updated:"
echo "  - package.json"
echo "  - src-tauri/Cargo.toml"
echo "  - src-tauri/tauri.conf.json"
echo ""
echo "Version is now $VERSION"
