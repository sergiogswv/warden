#!/bin/bash
#
# Warden Installation Script for Linux
# Installs Warden CLI to /usr/local/bin
#
# Usage (Development):
#   ./installers/install-linux.sh
#   (Automatically compiles if in project directory with changes)
#
# Usage (Production/Remote):
#   curl -fsSL https://raw.githubusercontent.com/sergiogswv/warden/installers/install-linux.sh | bash
#
# Custom environment variables:
#   GITHUB_REPO="owner/repo" VERSION="v0.1.0" ./install-linux.sh

set -e

INSTALL_DIR="${INSTALL_DIR:-/usr/local/bin}"
GITHUB_REPO="${GITHUB_REPO:-sergiogswv/warden}"
VERSION="${VERSION:-latest}"

# Check for local binary in same directory as script
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_DIR="$(dirname "$SCRIPT_DIR")"
LOCAL_BINARY="$SCRIPT_DIR/dist/warden-linux-x86_64"
COMPILED_BINARY="$PROJECT_DIR/target/release/warden"

# Colors for output
GREEN='\033[0;32m'
BLUE='\033[0;34m'
RED='\033[0;31m'
NC='\033[0m' # No Color

# Function to compare versions and detect if update is available
compare_versions() {
    local version_installed=$1
    local version_compiled=$2
    local binary_compiled=$3
    local binary_installed=$4

    # If installed doesn't exist, always update
    if [ ! -f "$binary_installed" ]; then
        return 0
    fi

    # If compiled is newer (by timestamp), ask user
    if [ "$binary_compiled" -nt "$binary_installed" ]; then
        return 0
    fi

    # If versions differ, ask user
    if [ "$version_installed" != "$version_compiled" ]; then
        return 0
    fi

    return 1  # No update needed
}

echo -e "${BLUE}┌─────────────────────────────────────┐${NC}"
echo -e "${BLUE}│  Warden Installation Script (Linux) │${NC}"
echo -e "${BLUE}└─────────────────────────────────────┘${NC}"
echo

# Detect OS and architecture
OS=$(uname -s)
ARCH=$(uname -m)

case "$OS" in
  Linux)
    case "$ARCH" in
      x86_64)
        BINARY_NAME="warden-linux-x86_64"
        ;;
      aarch64)
        BINARY_NAME="warden-linux-aarch64"
        ;;
      *)
        echo -e "${RED}✗ Unsupported architecture: $ARCH${NC}"
        exit 1
        ;;
    esac
    ;;
  *)
    echo -e "${RED}✗ This installer only supports Linux. Use the manual installation method.${NC}"
    exit 1
    ;;
esac

echo "System Information:"
echo "  • OS: $OS"
echo "  • Architecture: $ARCH"
echo "  • Install Directory: $INSTALL_DIR"
echo

# Auto-detect: if in project directory, automatically compile if needed
if [ -f "$PROJECT_DIR/Cargo.toml" ]; then
  # We're in the project directory - check if compilation is needed
  if [ ! -f "$COMPILED_BINARY" ] || [ "$PROJECT_DIR/src" -nt "$COMPILED_BINARY" ] 2>/dev/null; then
    # Source code is newer than binary, or binary doesn't exist - compile
    echo -e "${BLUE}→ Auto-compiling Warden (source code changed)...${NC}"
    cd "$PROJECT_DIR"
    cargo build --release 2>&1 | tail -3
    echo -e "${GREEN}✓ Build complete${NC}"
    echo
  fi
  # Use the compiled binary (always prefer it if in project dir)
  if [ -f "$COMPILED_BINARY" ]; then
    LOCAL_BINARY="$COMPILED_BINARY"
  fi
fi

# Check if installation directory is writable
if [ ! -w "$INSTALL_DIR" ]; then
  echo -e "${RED}✗ Installation directory is not writable. Try with sudo:${NC}"
  echo "  sudo curl -fsSL https://raw.githubusercontent.com/$GITHUB_REPO/installers/install-linux.sh | bash"
  exit 1
fi

# Try to install from local binary first
if [ -f "$LOCAL_BINARY" ]; then
  VERSION_INSTALLED=$(warden --version 2>/dev/null | cut -d' ' -f2 || echo "not installed")
  VERSION_COMPILED=$(cat "$SCRIPT_DIR/../.version" 2>/dev/null || echo "unknown")

  echo -e "${BLUE}Checking for updates...${NC}"
  echo "  Installed: $VERSION_INSTALLED"
  echo "  Compiled:  $VERSION_COMPILED"
  echo

  if compare_versions "$VERSION_INSTALLED" "$VERSION_COMPILED" "$LOCAL_BINARY" "$INSTALL_DIR/warden"; then
    echo -e "${YELLOW}→ Update available!${NC}"
    echo "  Auto-installing v$VERSION_COMPILED..."
    echo

    # Auto-update without asking
    if cp "$LOCAL_BINARY" "$INSTALL_DIR/warden"; then
      chmod +x "$INSTALL_DIR/warden"
      echo -e "${GREEN}✓ Successfully updated warden to v$VERSION_COMPILED${NC}"
      echo
      echo "Installation Details:"
      echo "  • Location: $INSTALL_DIR/warden"
      "$INSTALL_DIR/warden" --version
      echo
      echo -e "${GREEN}✓ Warden is ready to use!${NC}"
      echo "  Try: warden --help"
      exit 0
    else
      echo -e "${RED}✗ Failed to update warden${NC}"
      echo "Please try running with sudo:"
      echo "  sudo $SCRIPT_DIR/install-linux.sh"
      exit 1
    fi
  else
    echo -e "${GREEN}✓ Already up to date${NC}"
    exit 0
  fi
fi

# Fallback to downloading from GitHub
echo -e "${BLUE}Downloading Warden $VERSION...${NC}"

DOWNLOAD_URL="https://github.com/$GITHUB_REPO/releases/download/$VERSION/$BINARY_NAME"

if ! command -v curl &> /dev/null; then
  echo -e "${RED}✗ curl is required but not installed${NC}"
  exit 1
fi

# GitHub repo is now configured, ready to download
echo -e "${BLUE}Attempting to download from GitHub...${NC}"

if curl -fsSL "$DOWNLOAD_URL" -o "$INSTALL_DIR/warden"; then
  chmod +x "$INSTALL_DIR/warden"
  echo -e "${GREEN}✓ Successfully installed warden${NC}"
  echo
  echo "Installation Details:"
  echo "  • Location: $INSTALL_DIR/warden"
  "$INSTALL_DIR/warden" --version
  echo
  echo -e "${GREEN}✓ Warden is ready to use!${NC}"
  echo "  Try: warden --help"
else
  echo -e "${RED}✗ Failed to download Warden${NC}"
  echo
  echo "Troubleshooting:"
  echo "  1. Check internet connection"
  echo "  2. Verify GitHub repo exists: https://github.com/$GITHUB_REPO"
  echo "  3. Ensure release version exists: $VERSION"
  echo "  4. Or place binary at: $SCRIPT_DIR/dist/warden-linux-x86_64"
  exit 1
fi
