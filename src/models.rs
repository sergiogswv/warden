use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fmt;

/// Core metric: Lines of Code
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LOCMetric {
    pub file: String,
    pub timestamp: DateTime<Utc>,
    pub lines: usize,
}

impl fmt::Display for LOCMetric {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}: {} LOC on {}", self.file, self.lines, self.timestamp.format("%Y-%m-%d"))
    }
}

/// Core metric: Code Churn (% of lines rewritten)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChurnMetric {
    pub file: String,
    pub timestamp: DateTime<Utc>,
    pub churn_percentage: f64,
}

impl fmt::Display for ChurnMetric {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}: {:.1}% churn on {}", self.file, self.churn_percentage, self.timestamp.format("%Y-%m-%d"))
    }
}

/// Core metric: Author frequency
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuthorFrequency {
    pub file: String,
    pub author: String,
    pub commits: usize,
    pub lines_changed: usize,
}

impl fmt::Display for AuthorFrequency {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{} modified {} ({} commits, {} lines)", self.author, self.file, self.commits, self.lines_changed)
    }
}

/// Core metric: Cyclomatic complexity (estimated)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ComplexityMetric {
    pub file: String,
    pub timestamp: DateTime<Utc>,
    pub estimated_complexity: f64,
}

impl fmt::Display for ComplexityMetric {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}: complexity {} on {}", self.file, self.estimated_complexity as u32, self.timestamp.format("%Y-%m-%d"))
    }
}

/// Aggregated metrics for a file
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileMetrics {
    pub file: String,
    pub loc_history: Vec<LOCMetric>,
    pub churn_history: Vec<ChurnMetric>,
    pub authors: Vec<AuthorFrequency>,
    pub complexity_history: Vec<ComplexityMetric>,
}

impl FileMetrics {
    pub fn latest_loc(&self) -> Option<usize> {
        self.loc_history.last().map(|m| m.lines)
    }

    pub fn latest_churn(&self) -> Option<f64> {
        self.churn_history.last().map(|m| m.churn_percentage)
    }

    pub fn total_authors(&self) -> usize {
        self.authors.iter().map(|a| &a.author).collect::<std::collections::HashSet<_>>().len()
    }
}

/// Prediction for a module
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Prediction {
    pub file: String,
    pub severity: AlertSeverity,
    pub message: String,
    pub days_to_unmaintainable: Option<i32>,
    pub confidence: f64,
}

impl fmt::Display for Prediction {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "[{:?}] {} (confidence: {:.0}%)", self.severity, self.file, self.confidence * 100.0)
    }
}

/// Alert severity levels
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord)]
pub enum AlertSeverity {
    Info,
    Warning,
    Critical,
}

/// Trend direction
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum Trend {
    Improving,
    Stable,
    Degrading,
}

impl fmt::Display for Trend {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", match self {
            Trend::Improving => "✅ Improving",
            Trend::Stable => "→ Stable",
            Trend::Degrading => "⚠️ Degrading",
        })
    }
}

/// Complete analysis result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AnalysisResult {
    pub repository_path: String,
    pub analysis_period: String,
    pub files_analyzed: usize,
    pub total_commits: usize,
    pub authors_count: usize,
    pub file_metrics: HashMap<String, FileMetrics>,
    pub predictions: Vec<Prediction>,
    pub overall_trend: Trend,
    pub timestamp: DateTime<Utc>,
}
