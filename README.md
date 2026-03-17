# 👮 WARDEN

> **Real-Time Security & Vulnerability Guardian**

<p align="center">
  <img src="https://img.shields.io/badge/version-0.5.0-blue.svg" alt="Version">
  <img src="https://img.shields.io/badge/rust-2024-orange.svg" alt="Rust">
  <img src="https://img.shields.io/badge/license-MIT-green.svg" alt="License">
</p>

---

## 🚀 Key Responsibilities

**Warden** is an independent CLI tool that analyzes Git history to track technical debt over time and predict architecture problems before they happen.

Complements **Sentinel** (real-time monitoring) and **Architect Linter** (pre-commit validation) with **historical insights and predictive analysis**.

### ✨ Key Features

- 📊 **Technical Debt Tracking** - Visualize code quality evolution
- 🔮 **Predictive Alerts** - Know which modules will become unmaintainable (7/14-day forecasts)
- 🏆 **Hotspot Detection** - Identify files with high churn and complexity
- ✅ **Refactoring Detection** - Recognize successful refactoring (≥30% LOC reduction) and avoid false alerts
- 🎯 **Contextual Analysis** - Distinguish between refactoring, degradation, and growth patterns
- 👤 **Author Analytics** - See who touches what and when
- 🔀 **Branch Comparison** - Compare current branch vs main
- 📈 **Interactive Reports** - Menu-driven terminal interface
- 💾 **Local Caching** - Fast subsequent runs with `.warden-cache`
- 🎯 **Configurable History** - Analyze last 3 months, 6 months, 1 year, or custom period

---

## 📋 Core Metrics

Warden calculates four core metrics to identify code quality risks:

### 1. **LOC (Lines of Code)**
- **What it measures:** Total lines of code in a file
- **Why it matters:** Larger files are harder to understand and maintain
- **Interpretation:**
  - < 50 LOC: ✅ Easy to understand
  - 50-200 LOC: ✅ Normal, manageable
  - 200-500 LOC: ⚠️ Consider refactoring
  - > 500 LOC: 🔴 Likely needs decomposition

### 2. **Churn (Code Rotation)**
- **What it measures:** How much of the file was rewritten recently
- **Formula:** `(lines_added + lines_deleted) / total_lines × 100%`
- **Interpretation:**
  - < 20% churn: ✅ Stable, mature code
  - 20-50% churn: ⚠️ Normal development
  - 50-80% churn: 🔴 Highly unstable
  - > 80% churn: 🔴🔴 Critical instability or very new file

**Example:**
```
If a file has 100 lines and in the last 6 months:
- 30 lines were added
- 20 lines were deleted
- Churn = (30 + 20) / 100 = 50%
```

### 3. **Author Frequency**
- **What it measures:** How many different developers modified the file
- **Why it matters:** Files touched by many authors are harder to maintain (knowledge fragmentation)
- **Interpretation:**
  - 1 author: ✅ Clear ownership
  - 2-3 authors: ✅ Normal collaboration
  - 4+ authors: ⚠️ Possible coordination issues

### 4. **Cyclomatic Complexity (Estimated)**
- **What it measures:** Code complexity estimation based on file size
- **Formula:** `min(LOC / 50, 10)` (capped at 10.0)
- **Why it matters:** Complex code is error-prone and hard to test
- **Interpretation:**
  - 1-3: ✅ Simple, easy to test
  - 3-7: ✅ Normal complexity
  - 7-10: 🔴 Complex, needs refactoring

---

## 🎯 Risk Score (Hotspot Detection)

Warden combines all metrics into a **Risk Score** to identify true problem files:

**Formula:**
```
Risk Score = (churn% × LOC × author_count) / baseline
```

**Risk Levels:**
- 0-2: ✅ Safe zone - no action needed
- 2-5: ⚠️ Monitor - watch for changes
- 5-8: 🔴 Alert - consider refactoring
- > 8: 🔴🔴 Critical - refactor immediately

**Example:**
```
File A: Dockerfile
- LOC: 5
- Churn: 100%
- Authors: 1
- Risk: (100 × 5 × 1) / baseline = 0.5 ✅ Safe
→ Ignore: Config files change frequently, size doesn't matter

File B: src/api/client.ts
- LOC: 450
- Churn: 85%
- Authors: 3
- Risk: (85 × 450 × 3) / baseline = 11.4 🔴 Critical
→ Action: Large file, highly modified by multiple people
```

---

## 🔧 Refactoring Detection (v0.5.0+)

Warden now **automatically detects when files have been refactored** and applies intelligent analysis to avoid false positives.

### How It Works

When a file's LOC has been **reduced by ≥30%** compared to its historical maximum, Warden recognizes this as refactoring:

**Example:**
```
File: deal-migration.service.ts
- Historical LOC peak: 321 lines
- Current LOC: 142 lines
- Reduction: 56% → ✅ Refactoring Detected!

Instead of: 🔴 Critical (254% churn alert)
Shows: ✅ Refactoring detected (LOC -56%)
Risk Score: Attenuated by 0.6× to account for positive change
```

### Contextual Recommendations

The system now provides context-aware recommendations:

| Pattern | Recommendation |
|---------|---|
| ✅ Refactoring detected (LOC -X%) | Monitor stabilization after improvements |
| 📈 Growing with high churn | Refactor needed - expansion causing instability |
| 🔴 High churn, stable LOC | Code instability - consider refactoring |
| ⚠️ Degrading trend | Churn increasing - quality declining |
| 🔴 Large + fragmented | Refactor - multiple authors, high churn |

---

### Key Metrics

---

## 🔧 Quick Start

```bash
# Install via Go
git clone https://github.com/sergiogswv/warden.git
cd warden
go build -o warden main.go
```

---

## 📦 Installation

### From Source (Development)

```bash
# Clone and build
git clone https://github.com/sergiogswv/warden.git
cd warden
cargo build --release

# Install locally
./installers/install-linux.sh
```

### From Releases (Production)

```bash
# Linux
curl -fsSL https://raw.githubusercontent.com/sergiogswv/warden/installers/install-linux.sh | bash

# macOS
brew tap sergiogswv/warden
brew install warden

# Windows
powershell -Command "& { $(irm https://raw.githubusercontent.com/sergiogswv/warden/installers/install-windows.ps1) }"
```

See [Installation Guide](installers/README.md) for more details.

---

## 🔄 Version Management & Updates

### Quick Update (Recommended)

From the Warden project directory - just run:

```bash
./installers/install-linux.sh

# Or with sudo if you have permission issues
sudo ./installers/install-linux.sh
```

The installer automatically:
- ✅ Detects if you're in the project directory
- ✅ Compiles automatically if code changed
- ✅ Detects version changes
- ✅ Updates `/usr/local/bin/warden`
- ✅ Verifies the installation

**No flags, no complexity!** Just execute the script.

### Check for Updates

```bash
# Check installed vs compiled version
warden check-updates

# Or use the helper script
./installers/check-updates.sh
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
