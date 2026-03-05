#!/bin/bash
#
# Warden Update Checker
# Checks for available updates
#
# Usage: ./installers/check-updates.sh

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_DIR="$(dirname "$SCRIPT_DIR")"

INSTALLED_VERSION=$(warden --version 2>/dev/null | awk '{print $NF}')
COMPILED_VERSION=$(cat "$PROJECT_DIR/.version" 2>/dev/null || echo "unknown")

echo "╔═══════════════════════════════════════╗"
echo "║  Warden Update Check                  ║"
echo "╚═══════════════════════════════════════╝"
echo
echo "Installed version: $INSTALLED_VERSION"
echo "Compiled version:  $COMPILED_VERSION"
echo

if [ "$INSTALLED_VERSION" = "$COMPILED_VERSION" ]; then
    echo "✓ Up to date"
else
    echo "✓ Update available!"
    echo "Run: ./installers/install-linux.sh"
fi
