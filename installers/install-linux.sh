#!/bin/bash
#
# Warden Installation Script for Linux
# Installs Warden CLI to /usr/local/bin
#
# Usage: curl -fsSL https://raw.githubusercontent.com/YOUR_REPO/installers/install-linux.sh | bash

set -e

INSTALL_DIR="${INSTALL_DIR:-/usr/local/bin}"
GITHUB_REPO="YOUR_GITHUB_REPO"  # Update with your repo
VERSION="${VERSION:-latest}"

# Colors for output
GREEN='\033[0;32m'
BLUE='\033[0;34m'
RED='\033[0;31m'
NC='\033[0m' # No Color

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

# Check if installation directory is writable
if [ ! -w "$INSTALL_DIR" ]; then
  echo -e "${RED}✗ Installation directory is not writable. Try with sudo:${NC}"
  echo "  sudo curl -fsSL https://raw.githubusercontent.com/$GITHUB_REPO/installers/install-linux.sh | bash"
  exit 1
fi

# Download and install
echo -e "${BLUE}Downloading Warden $VERSION...${NC}"

DOWNLOAD_URL="https://github.com/$GITHUB_REPO/releases/download/$VERSION/$BINARY_NAME"

if ! command -v curl &> /dev/null; then
  echo -e "${RED}✗ curl is required but not installed${NC}"
  exit 1
fi

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
  exit 1
fi
