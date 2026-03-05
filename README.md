# Warden 🛡️

**Historical code quality analysis and predictive architecture insights for modern development teams.**

<p align="center">
  <img src="https://img.shields.io/badge/version-0.1.0-blue.svg" alt="Version">
  <img src="https://img.shields.io/badge/rust-2024-orange.svg" alt="Rust">
  <img src="https://img.shields.io/badge/license-MIT-green.svg" alt="License">
</p>

---

## 🚀 What is Warden?

**Warden** is an independent CLI tool that analyzes Git history to track technical debt over time and predict architecture problems before they happen.

Complements **Sentinel** (real-time monitoring) and **Architect Linter** (pre-commit validation) with **historical insights and predictive analysis**.

### ✨ Key Features

- 📊 **Technical Debt Tracking** - Visualize code quality evolution
- 🔮 **Predictive Alerts** - Know which modules will become unmaintainable
- 🏆 **Hotspot Detection** - Identify files with high churn and complexity
- 👤 **Author Analytics** - See who touches what and when
- 🔀 **Branch Comparison** - Compare current branch vs main
- 📈 **Interactive Reports** - Menu-driven terminal interface
- 💾 **Local Caching** - Fast subsequent runs with `.warden-cache`
- 🎯 **Configurable History** - Analyze last 3 months, 6 months, 1 year, or custom period

---

## 📋 Core Metrics

1. **LOC (Lines of Code)** - Track file size growth/shrinkage over time
2. **Churn** - Code rotation ratio: `(deleted + modified) / total`
3. **Author Frequency** - Which developers touch which files
4. **Cyclomatic Complexity (Est.)** - Function length and conditional depth

---

## 🔧 Quick Start

```bash
# Build the project
cargo build --release

# Run Warden (interactive menu)
./target/release/warden

# Analyze last 3 months
./target/release/warden --history 3m

# Compare branches
./target/release/warden --compare main origin/develop

# Export JSON
./target/release/warden --json > metrics.json
```

---

## 📦 Installation

### From Source (Development)

```bash
# Clone and build
git clone https://github.com/YOUR_REPO/warden.git
cd warden
cargo build --release

# Install locally
./installers/install-linux.sh
```

### From Releases (Production)

```bash
# Linux
curl -fsSL https://raw.githubusercontent.com/YOUR_REPO/installers/install-linux.sh | bash

# macOS
brew tap YOUR_ORG/warden
brew install warden

# Windows
powershell -Command "& { $(irm https://raw.githubusercontent.com/YOUR_REPO/installers/install-windows.ps1) }"
```

See [Installation Guide](installers/README.md) for more details.

---

## 🔄 Version Management & Updates

### Check for Updates

```bash
# Check installed vs available version
warden check-updates

# Or use the helper script
./installers/check-updates.sh
```

### Update Warden

```bash
# Development: recompile and install
cargo build --release
./installers/install-linux.sh  # Prompts if new version available

# Production: download latest release
warden update
```

### Release Process

```bash
# Update version
echo "0.2.0" > .version

# Build and test
cargo build --release
./target/release/warden --version

# Publish (automated)
./installers/release.sh 0.2.0
```

See [Versioning Guide](docs/VERSIONING.md) for complete details.

---

## 📖 Documentation

- **[Installation Guide](installers/README.md)** - Install Warden on any platform
- **[Version Management](docs/VERSIONING.md)** - Versioning and release process
- **[Design Document](docs/plans/2026-03-05-installation-system-design.md)** - System architecture
- **[CLI Commands](docs/commands.md)** - Detailed command reference
- **[Configuration](docs/configuration.md)** - Settings and customization

---

## 🏗️ Architecture

```
Git Repository
    ↓
[Git Parser] → [Metrics Calculator]
    ↓
[Analytics Engine] → [Prediction Module]
    ↓
[Interactive Terminal UI]
```

---

## 🔄 Ecosystem Integration

Part of the Sergio's development tools ecosystem:

- **Sentinel** - Real-time assistant during development
- **Architect Linter** - Pre-commit architecture validation
- **Warden** - Historical analysis & predictions ← YOU ARE HERE

Each tool serves a distinct purpose with zero overlap.

---

## 📊 Project Status

```
Phase 1: 🚧 MVP Development
├─ Git parser
├─ Metrics calculation
├─ Basic analytics
├─ Interactive UI
└─ Predictive alerts
```

---

## 📝 License

MIT License - See LICENSE file for details

---

## 👤 Author

**Sergio Guadarrama**

---

<p align="center">
  Made with ❤️ using Rust
</p>
