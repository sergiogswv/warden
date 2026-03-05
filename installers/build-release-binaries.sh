#!/bin/bash
#
# Build release binaries for all supported platforms
# This script requires: cargo and optionally cargo-cross for cross-compilation
#
# Usage:
#   ./build-release-binaries.sh
#   ./build-release-binaries.sh 0.1.0

set -e

VERSION="${1:-0.1.0}"
RELEASE_DIR="release-$VERSION"
BUILD_DIR="target/release"

# Colors
GREEN='\033[0;32m'
BLUE='\033[0;34m'
YELLOW='\033[1;33m'
NC='\033[0m'

echo -e "${BLUE}╔════════════════════════════════════════╗${NC}"
echo -e "${BLUE}║  Warden Release Binary Builder v${VERSION}   ║${NC}"
echo -e "${BLUE}╚════════════════════════════════════════╝${NC}"
echo

# Clean previous builds
rm -rf "$RELEASE_DIR"
mkdir -p "$RELEASE_DIR"

# Detect current OS
OS=$(uname -s)
ARCH=$(uname -m)

case "$OS" in
  Linux)
    CURRENT_PLATFORM="linux-$ARCH"
    ;;
  Darwin)
    CURRENT_PLATFORM="macos-$ARCH"
    ;;
  *)
    CURRENT_PLATFORM="unknown"
    ;;
esac

echo "Building Warden v$VERSION"
echo "Current Platform: $CURRENT_PLATFORM"
echo

# Function to build and package for a platform
build_platform() {
  local target=$1
  local platform_name=$2
  local binary_name="warden-$platform_name"

  echo -e "${YELLOW}→ Building for $platform_name ($target)...${NC}"

  if cargo build --release --target "$target" 2>&1 | tail -3; then
    local source_binary="$BUILD_DIR/$target/release/warden"
    if [ -f "$source_binary" ]; then
      cp "$source_binary" "$RELEASE_DIR/$binary_name"
      chmod +x "$RELEASE_DIR/$binary_name"
      echo -e "${GREEN}✓ Built: $binary_name${NC}"
    else
      echo -e "${YELLOW}⚠ Binary not found at $source_binary${NC}"
    fi
  else
    echo -e "${YELLOW}⚠ Build failed for $target${NC}"
  fi
  echo
}

# Build for Linux x86_64 (always possible on Linux)
if [ "$OS" = "Linux" ]; then
  build_platform "x86_64-unknown-linux-gnu" "linux-x86_64"
fi

# Create tarballs for each binary
echo -e "${YELLOW}→ Creating distribution packages...${NC}"
cd "$RELEASE_DIR"

for binary in warden-*; do
  if [ -f "$binary" ]; then
    tar -czf "${binary}.tar.gz" "$binary"
    ls -lh "${binary}.tar.gz"
    echo -e "${GREEN}✓ Created: ${binary}.tar.gz${NC}"
  fi
done

cd ..

echo
echo -e "${BLUE}╔════════════════════════════════════════╗${NC}"
echo -e "${BLUE}║  Release Build Complete!              ║${NC}"
echo -e "${BLUE}╚════════════════════════════════════════╝${NC}"
echo
echo "Release files available in: $RELEASE_DIR/"
echo "Run 'ls -la $RELEASE_DIR/' to see all files"
echo
echo "Cross-compilation notes:"
echo "  • To build for macOS on Linux: install 'cargo-cross' and use targets:"
echo "    - aarch64-apple-darwin (Apple Silicon)"
echo "    - x86_64-apple-darwin (Intel Mac)"
echo
echo "  • To build for Windows on Linux: use targets:"
echo "    - x86_64-pc-windows-gnu"
echo "    - x86_64-pc-windows-msvc"
echo
echo "  • Or compile natively on the target platform"
echo
