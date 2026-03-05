#!/bin/bash
#
# Warden Local Installation Script
# Installs Warden from a local build directory
#
# Usage:
#   sudo bash install-local.sh
#   Or with custom install directory:
#   INSTALL_DIR=$HOME/.local/bin bash install-local.sh (no sudo needed)

set -e

INSTALL_DIR="${INSTALL_DIR:-/usr/local/bin}"
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
BINARY_SRC="$SCRIPT_DIR/dist/warden-linux-x86_64"

# Colors
GREEN='\033[0;32m'
BLUE='\033[0;34m'
RED='\033[0;31m'
NC='\033[0m'

echo -e "${BLUE}┌─────────────────────────────────┐${NC}"
echo -e "${BLUE}│ Warden Local Installation       │${NC}"
echo -e "${BLUE}└─────────────────────────────────┘${NC}"
echo

# Check if binary exists
if [ ! -f "$BINARY_SRC" ]; then
  echo -e "${RED}✗ Binary not found at: $BINARY_SRC${NC}"
  echo
  echo "Please build Warden first:"
  echo "  cd $(dirname "$SCRIPT_DIR")"
  echo "  cargo build --release"
  exit 1
fi

# Check install directory permissions
if [ ! -w "$INSTALL_DIR" ]; then
  echo -e "${RED}✗ Installation directory is not writable: $INSTALL_DIR${NC}"
  echo
  if [ "$EUID" -ne 0 ]; then
    echo "Run with sudo:"
    echo "  sudo bash $0"
  else
    echo "Try a different directory:"
    echo "  INSTALL_DIR=\$HOME/.local/bin bash $0"
  fi
  exit 1
fi

echo "System Information:"
echo "  • Binary: $BINARY_SRC"
echo "  • Destination: $INSTALL_DIR/warden"
echo

# Install
echo -e "${BLUE}Installing Warden...${NC}"

if cp "$BINARY_SRC" "$INSTALL_DIR/warden"; then
  chmod +x "$INSTALL_DIR/warden"
  echo -e "${GREEN}✓ Successfully installed warden${NC}"
  echo

  # Verify installation
  echo "Installation Details:"
  echo "  • Location: $INSTALL_DIR/warden"
  if command -v "$INSTALL_DIR/warden" &> /dev/null; then
    "$INSTALL_DIR/warden" --version
  fi
  echo

  # Check if in PATH
  if command -v warden &> /dev/null; then
    echo -e "${GREEN}✓ Warden is in your PATH and ready to use!${NC}"
    echo "  Try: warden --help"
  else
    echo -e "${BLUE}ℹ Warden installed but not in PATH yet${NC}"
    if [ "$INSTALL_DIR" != "/usr/local/bin" ]; then
      echo "  Add to your shell profile:"
      echo "    export PATH=\"$INSTALL_DIR:\$PATH\""
    else
      echo "  Restart your terminal or run: source ~/.bashrc"
    fi
  fi
else
  echo -e "${RED}✗ Failed to install Warden${NC}"
  exit 1
fi
