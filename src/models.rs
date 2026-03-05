use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Core metric: Lines of Code
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LOCMetric {
    pub file: String,
    pub timestamp: DateTime<Utc>,
    pub lines: usize,
}

/// Core metric: Code Churn (% of lines rewritten)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChurnMetric {
    pub file: String,
    pub timestamp: DateTime<Utc>,
    pub churn_percentage: f64,
}

/// Core metric: Author frequency
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuthorFrequency {
    pub file: String,
    pub author: String,
    pub commits: usize,
    pub lines_changed: usize,
}

/// Core metric: Cyclomatic complexity (estimated)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ComplexityMetric {
    pub file: String,
    pub timestamp: DateTime<Utc>,
    pub estimated_complexity: f64,
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

/// Prediction for a module
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Prediction {
    pub file: String,
    pub severity: AlertSeverity,
    pub message: String,
    pub days_to_unmaintainable: Option<i32>,
    pub confidence: f64,
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
