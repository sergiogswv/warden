#!/bin/bash
#
# Warden Release Publisher
# Publishes a new version to GitHub Releases
#
# Usage: ./installers/release.sh 0.2.0

set -e

VERSION="${1:-}"
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_DIR="$(dirname "$SCRIPT_DIR")"

# Colors
GREEN='\033[0;32m'
BLUE='\033[0;34m'
RED='\033[0;31m'
YELLOW='\033[1;33m'
NC='\033[0m'

echo -e "${BLUE}╔════════════════════════════════════════╗${NC}"
echo -e "${BLUE}║  Warden Release Publisher             ║${NC}"
echo -e "${BLUE}╚════════════════════════════════════════╝${NC}"
echo

# Validate version argument
if [ -z "$VERSION" ]; then
    echo -e "${RED}✗ Version required${NC}"
    echo "Usage: ./installers/release.sh 0.2.0"
    exit 1
fi

# Remove 'v' prefix if present
VERSION="${VERSION#v}"
GIT_TAG="v$VERSION"

echo "Release version: $VERSION"
echo

# Step 1: Validate version in files
echo -e "${YELLOW}→ Validating version files...${NC}"

VERSION_FILE="$PROJECT_DIR/.version"
if [ ! -f "$VERSION_FILE" ]; then
    echo -e "${RED}✗ .version file not found at $VERSION_FILE${NC}"
    exit 1
fi

VERSION_IN_FILE=$(cat "$VERSION_FILE")
if [ "$VERSION_IN_FILE" != "$VERSION" ]; then
    echo -e "${RED}✗ Version mismatch:${NC}"
    echo "  .version: $VERSION_IN_FILE"
    echo "  Requested: $VERSION"
    echo
    echo "Fix with:"
    echo "  echo \"$VERSION\" > .version"
    exit 1
fi

echo -e "${GREEN}✓ Version validated${NC}"
echo

# Step 2: Check for git tag
echo -e "${YELLOW}→ Checking git tags...${NC}"

if git -C "$PROJECT_DIR" rev-parse "$GIT_TAG" >/dev/null 2>&1; then
    echo -e "${RED}✗ Git tag $GIT_TAG already exists${NC}"
    echo "Create new release with a different version"
    exit 1
fi

echo -e "${GREEN}✓ Tag $GIT_TAG ready to create${NC}"
echo

# Step 3: Verify binary exists
echo -e "${YELLOW}→ Checking compiled binary...${NC}"

BINARY="$PROJECT_DIR/target/release/warden"
if [ ! -f "$BINARY" ]; then
    echo -e "${YELLOW}⚠ Binary not found at $BINARY${NC}"
    echo "Building..."
    cd "$PROJECT_DIR"
    cargo build --release
    echo -e "${GREEN}✓ Build complete${NC}"
fi

echo -e "${GREEN}✓ Binary ready${NC}"
echo

# Step 4: Create release directory
echo -e "${YELLOW}→ Preparing release package...${NC}"

RELEASE_DIR="$SCRIPT_DIR/release-$VERSION"
rm -rf "$RELEASE_DIR"
mkdir -p "$RELEASE_DIR"

# Copy binary with version suffix
cp "$BINARY" "$RELEASE_DIR/warden-linux-x86_64"
cd "$RELEASE_DIR"

# Create tarball
tar -czf "warden-linux-x86_64.tar.gz" "warden-linux-x86_64"

# Create checksums
sha256sum warden-linux-x86_64 > warden-linux-x86_64.sha256
sha256sum "warden-linux-x86_64.tar.gz" >> checksums.txt

echo -e "${GREEN}✓ Release package created${NC}"
echo

# Step 5: Create git tag
echo -e "${YELLOW}→ Creating git tag...${NC}"

cd "$PROJECT_DIR"
git tag -a "$GIT_TAG" -m "Release $VERSION"

echo -e "${GREEN}✓ Git tag created: $GIT_TAG${NC}"
echo

# Step 6: Summary
echo -e "${BLUE}╔════════════════════════════════════════╗${NC}"
echo -e "${BLUE}║  Release Prepared Successfully        ║${NC}"
echo -e "${BLUE}╚════════════════════════════════════════╝${NC}"
echo

echo "Release package: $RELEASE_DIR"
echo "Files created:"
ls -lh "$RELEASE_DIR"

echo
echo "Next steps:"
echo "1. Review the release files"
echo "2. Push the tag: git push origin $GIT_TAG"
echo "3. Create GitHub Release: gh release create $GIT_TAG $(ls $RELEASE_DIR/*) --title \"Warden $VERSION\""
echo

echo -e "${GREEN}✓ Ready to publish!${NC}"
