# Warden Installation Guide

This directory contains installation scripts and tools for distributing Warden across different platforms.

## Quick Start

### Linux (Recommended)

```bash
curl -fsSL https://raw.githubusercontent.com/sergiogswv/warden/installers/install-linux.sh | bash
```

Or with manual installation:

```bash
# Download the latest release
curl -L -o /tmp/warden-linux.tar.gz https://github.com/sergiogswv/warden/releases/download/v0.2.0/warden-linux-x86_64.tar.gz
tar -xzf /tmp/warden-linux.tar.gz
sudo mv warden-linux-x86_64 /usr/local/bin/warden
chmod +x /usr/local/bin/warden

# Verify installation
warden --version
```

### macOS

#### Option 1: Homebrew (Recommended)

First, set up the tap:

```bash
brew tap sergiogswv/warden
brew install warden
```

#### Option 2: Manual Installation

```bash
# Download the appropriate binary
# For Apple Silicon:
curl -L -o /tmp/warden-macos.tar.gz https://github.com/sergiogswv/warden/releases/download/v0.2.0/warden-macos-aarch64.tar.gz

# For Intel Mac:
curl -L -o /tmp/warden-macos.tar.gz https://github.com/sergiogswv/warden/releases/download/v0.2.0/warden-macos-x86_64.tar.gz

tar -xzf /tmp/warden-macos.tar.gz
sudo mv warden-macos-* /usr/local/bin/warden
chmod +x /usr/local/bin/warden

# Verify installation
warden --version
```

### Windows

#### Option 1: PowerShell Script (Recommended)

```powershell
powershell -Command "& { $(irm https://raw.githubusercontent.com/sergiogswv/warden/installers/install-windows.ps1) }"
```

#### Option 2: Manual Installation

1. Download: `warden-windows-x64.exe` from the latest release
2. Create a folder: `%LOCALAPPDATA%\warden\bin`
3. Move the executable there
4. Add to PATH:
   - Press `Win + X` → System
   - Click "Advanced system settings"
   - Click "Environment Variables"
   - Under User variables, select PATH and click Edit
   - Add: `%LOCALAPPDATA%\warden\bin`
5. Restart PowerShell/CMD
6. Verify: `warden --version`

## Version Management

Warden uses a single version source (`.version` file) for all components.

### Quick Update (Development)

**Recommended method - Build and auto-install in one command:**

```bash
# From project root or installers directory
./installers/install-linux.sh --build

# Or with sudo if needed
sudo ./installers/install-linux.sh --build
```

This automatically:
- ✅ Compiles the latest version
- ✅ Detects version changes
- ✅ Updates `/usr/local/bin/warden`
- ✅ Verifies installation

### Checking for Updates

```bash
./installers/check-updates.sh
```

### Releasing New Version

```bash
# 1. Update version
echo "0.2.0" > .version

# 2. Build and test
cargo build --release
./target/release/warden --version

# 3. Publish
./installers/release.sh 0.2.0
```

## Building Release Binaries

### Prerequisites

- Rust toolchain installed ([rustup.rs](https://rustup.rs))
- For cross-compilation: `cargo install cross`

### Build for Current Platform

```bash
./build-release-binaries.sh 0.1.0
```

This creates optimized binaries in the `release-0.1.0/` directory.

### Build for Multiple Platforms (Linux)

If you have `cargo-cross` installed:

```bash
# Setup targets
rustup target add x86_64-unknown-linux-gnu aarch64-unknown-linux-gnu

# Then run:
./build-release-binaries.sh 0.1.0
```

### Cross-Compilation

To build for other platforms on Linux:

```bash
# macOS (requires macOS development files)
rustup target add aarch64-apple-darwin x86_64-apple-darwin
cargo build --release --target aarch64-apple-darwin
cargo build --release --target x86_64-apple-darwin

# Windows (MinGW)
rustup target add x86_64-pc-windows-gnu
cargo build --release --target x86_64-pc-windows-gnu
```

Or use `cargo-cross`:

```bash
cargo install cross

# Build for various platforms
cross build --release --target aarch64-unknown-linux-gnu
cross build --release --target x86_64-apple-darwin
```

## Installation Script Details

### `install-linux.sh`

- **Platform**: Linux (x86_64, aarch64)
- **Destination**: `/usr/local/bin/warden`
- **Features**: Auto-detects architecture, color output, error handling
- **Requires**: `curl`

### `install-windows.ps1`

- **Platform**: Windows
- **Destination**: `%LOCALAPPDATA%\warden\bin`
- **Features**: Auto-adds to PATH, error handling, requires .NET/PowerShell 5.1+
- **Permissions**: Can run as user (installs to LOCALAPPDATA) or admin (installs to Program Files)

### `warden.rb`

- **Platform**: macOS
- **Type**: Homebrew formula
- **Usage**: Place in homebrew-core fork or use with `brew tap`
- **Requires**: Setting correct SHA256 hashes for releases

## Verifying Installation

After installation, verify it works:

```bash
# Show version
warden --version

# Show help
warden --help

# Analyze current directory
warden
```

## Publishing Releases

### GitHub Releases

1. Create a new release tag:

```bash
git tag -a v0.2.0 -m "Warden v0.2.0"
git push origin v0.2.0
```

2. Build binaries:

```bash
./build-release-binaries.sh 0.1.0
```

3. Upload to GitHub releases:

```bash
gh release create v0.2.0 release-0.1.0/* --title "Warden v0.2.0" --notes "See CHANGELOG for details"
```

### Update Homebrew Formula

For macOS releases:

1. Calculate SHA256 of macOS binaries:

```bash
sha256sum release-0.1.0/warden-macos-*
```

2. Update `warden.rb` with the new version and SHA256 hashes
3. Test locally: `brew tap sergiogswv/warden --clone`
4. Submit to homebrew-core via PR

## Troubleshooting

### "Permission denied" when running warden

```bash
# Fix: Ensure binary is executable
chmod +x /usr/local/bin/warden
# Or on Windows (PowerShell as Admin):
# The installer should handle this automatically
```

### "command not found: warden"

- **Linux/macOS**: Ensure `/usr/local/bin` is in your PATH: `echo $PATH`
- **Windows**: Restart PowerShell after installation

### Download fails

- Check internet connection
- Verify the download URL is correct
- Try downloading manually from GitHub releases

## Support & Contributions

For issues or improvements:
- Report bugs: [GitHub Issues](https://github.com/sergiogswv/warden/issues)
- Contribute: [GitHub Pull Requests](https://github.com/sergiogswv/warden/pulls)

---

**Version**: 0.1.0
**Last Updated**: 2026-03-05
