# Warden - Design Document

**Date**: 2026-03-04
**Status**: Approved
**Author**: Brainstorming Session

---

## 📋 Executive Summary

**Warden** is an independent CLI tool written in Rust that analyzes Git history to extract technical debt metrics and predict future architecture problems. It complements Sentinel (real-time monitoring) and Architect Linter (pre-commit validation) by providing **historical insights and predictive analysis**.

**Key Differentiator**: Completely independent - no dependencies on Sentinel or Architect Linter. Works with any TypeScript/JavaScript project.

---

## 🎯 Goals

1. **Track Technical Debt Over Time** - Visualize how code quality evolves
2. **Predict Architecture Problems** - Alert on unmaintainable modules 2-3 weeks in advance
3. **Identify Hotspots** - Show which files/modules have the most churn and complexity
4. **Empower Teams** - Data-driven decisions on refactoring priorities

---

## 📊 Core Metrics (4 Pillars)

### 1. **Complexity per File (LOC)**
- Lines of Code tracked over time
- Histogram: "How many files have 50-100 LOC?"
- Trend: Growing/shrinking files

### 2. **Churn (Code Rotation)**
- Ratio: `(deleted_lines + modified_lines) / total_lines` per week
- High churn = instability (code rewritten frequently)
- Formula: Calculate per file, aggregate by module

### 3. **Author Frequency**
- Which developers modify which files
- Identifies "hotspots" where multiple devs interfere
- Measures: commits_per_developer, files_touched, patterns

### 4. **Cyclomatic Complexity (Approximated)**
- Estimated via regex on file diffs
- Count: function length, conditional depth
- Predictor of bug potential

---

## 🏗️ Architecture

```
┌─────────────────────────────────────────────┐
│      Warden CLI (Rust)                      │
├─────────────────────────────────────────────┤
│                                             │
│  INPUT: Git Repository                      │
│    ↓                                        │
│  [Git Parser] → [Metrics Engine]            │
│    • git log parsing                │ LOC   │
│    • diff extraction                │ Churn │
│    • author tracking                │ Freq  │
│                                     │ Cmplx │
│    ↓                                        │
│  [Analytics Engine]                         │
│    • Trend detection                        │
│    • Hotspot identification                 │
│    • Correlation analysis                   │
│    ↓                                        │
│  [Prediction Module]                        │
│    • Linear regression                      │
│    • Problem forecasting                    │
│    ↓                                        │
│  [Interactive Terminal UI]                  │
│    • Menu-driven navigation                 │
│    • ASCII charts                           │
│    • Colored alerts                         │
│    • JSON export                            │
└─────────────────────────────────────────────┘
```

---

## 📁 Project Structure

```
warden/
├── src/
│   ├── main.rs              # CLI entry point
│   ├── git_parser.rs        # git log parsing + diff extraction
│   ├── metrics.rs           # LOC, Churn, Complexity calculations
│   ├── analytics.rs         # Trend detection, hotspot identification
│   ├── prediction.rs        # Linear regression for forecasting
│   ├── ui.rs                # Interactive terminal UI
│   ├── cache.rs             # Local caching (.warden-cache)
│   └── models.rs            # Data structures
├── Cargo.toml
├── README.md
└── tests/
    └── integration_tests.rs
```

---

## 🎮 User Interaction - Interactive Menu

**Default Command:**
```bash
$ warden
```

**Output:**
```
╔════════════════════════════════════╗
║   Warden v1.0                      ║
║   Code Quality Historical Analysis ║
╚════════════════════════════════════╝

📊 Analyzing last 6 months of Git...
   ✓ 324 commits processed
   ✓ 58 files analyzed
   ✓ 7 authors identified

What would you like to see?
  1. 📈 Technical Debt Trends
  2. ⚠️  Predictive Alerts
  3. 🏆 Top 10 Problem Modules
  4. 👤 Author Statistics
  5. 🔀 Compare main vs current branch
  6. ⚙️  Settings (period, filters)
  x. Exit

> _
```

### **Example Output: Technical Debt Trends**

```
📊 TECHNICAL DEBT (6 MONTHS)
═════════════════════════════════

Code Churn Rate (% rewritten):

 100% ┤
      │                     ╱╲
  75% ┤                   ╱  ╲╱─╮
      │                 ╱        ╲
  50% ┤               ╱            ╲
      │    ╭─╮    ╱
  25% ┤───╱   ╰──╱
       ┴──────────────────────────────
       Dec   Jan   Feb   Mar   Apr   May

✅ TREND: IMPROVING (-15% since March)
🎯 Projection: Normal churn in 3 weeks
```

### **Example Output: Predictive Alerts**

```
⚠️  PREDICTIVE ALERTS
════════════════════════════

🔴 CRITICAL (2 weeks):
   src/services/auth.service.ts
   → Size: 95 LOC (growing +3/week)
   → Churn: 78% (high instability)
   → Prediction: Unmaintainable in 12 days
   → Action: Refactor recommended

🟡 WARNING (4 weeks):
   src/repositories/user.ts
   → Churn: 70% (frequent rewrites)
   → Stability: Degrading
   → Action: Monitor closely
```

---

## 🔧 CLI Commands (MVP)

```bash
# Interactive menu (default)
warden

# Analyze with custom period
warden --history 3m              # Last 3 months
warden --history 1y              # Last year
warden --since "2025-01-01"     # Since specific date

# Compare branches
warden --compare main origin/develop

# Output formats
warden --json                    # Structured JSON
warden --format json             # Same as above
warden --format markdown         # Markdown report

# Specific reports
warden --only-predictions        # Only alerts
warden --only-hotspots          # Only top modules
warden --only-trends            # Only trends

# Help
warden --help
warden --version
```

---

## 📊 Data Analysis Pipeline

### **Phase 1: Git Parsing**
- Execute: `git log --pretty=format:...` with custom format
- Extract: commit hash, author, date, files changed, lines added/removed
- Cache results in `.warden-cache` (JSON)

### **Phase 2: Metrics Calculation**
- **LOC Trend**: Group by file, week → line count
- **Churn**: `(added + deleted) / total` per week per file
- **Author Freq**: Count commits per author per file
- **Complexity Est**: Regex on diffs (function length, conditionals)

### **Phase 3: Analytics**
- **Trend Detection**: Linear regression on metrics
- **Hotspot ID**: Top 10 files by churn + complexity
- **Correlation**: Which authors touch risky files?

### **Phase 4: Prediction**
- **Linear Regression**: Last 12 data points → next 4 weeks
- **Confidence Score**: R² value (0-1)
- **Alert Thresholds**:
  - Critical: Churn > 80% AND LOC growing
  - Warning: Churn > 60% OR LOC > 200

---

## 🛠️ Technical Stack

**Language**: Rust 2024 edition

**Key Dependencies**:
- `git2` - Git repository access
- `dialoguer` - Interactive terminal menus
- `prettytable-rs` - Terminal tables
- `serde/serde_json` - Serialization
- `indicatif` - Progress bars
- `regex` - Simple complexity analysis
- `chrono` - Date handling

**Storage**: SQLite (optional, v1.1+) or JSON cache

---

## 🎯 MVP Scope (v1.0)

### ✅ Included
- Git parsing for TypeScript/JavaScript
- 4 core metrics (LOC, Churn, Author Freq, Complexity Est)
- Trend calculation (simple linear regression)
- Interactive CLI menu
- Predictive alerts (basic)
- Top 10 hotspots
- Branch comparison
- JSON export

### ❌ Not Included (v1.1+)
- Tree-sitter for multi-language support
- Real cyclomatic complexity analysis
- HTML/PDF reports
- CI/CD integration
- Web dashboard
- Advanced ML predictions

---

## 📈 Success Criteria

A developer using Warden can:
1. ✅ See debt trends at a glance (ASCII charts)
2. ✅ Know which modules need attention (top 10)
3. ✅ Get warned 2-3 weeks before problems (predictions)
4. ✅ Identify author patterns (who touches risky code)
5. ✅ Make data-driven refactoring decisions

---

## 🔄 Ecosystem Integration

**Sentinel** (Real-time during dev) → **Warden** (Historical insights) → **Architect Linter** (Pre-commit validation)

- **Sentinel**: "Your code might violate SRP" (now)
- **Warden**: "This module had 80% churn last month, trend is bad" (historical)
- **Architect Linter**: "This violates the architecture rule" (blocking)

**Each tool has a distinct role, no redundancy.**

---

## ✅ Design Approved

- Independent CLI: ✅
- Git-based analysis: ✅
- 4 core metrics: ✅
- Interactive menu: ✅
- TypeScript/JS MVP: ✅
- Predictive alerts: ✅
- Configurable history: ✅

---

**Next Step**: Implementation Plan
