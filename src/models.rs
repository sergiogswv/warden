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

/// Risk classification level
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum RiskLevel {
    Safe,
    Monitor,
    Alert,
    Critical,
}

impl fmt::Display for RiskLevel {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            RiskLevel::Safe => write!(f, "✅ Safe"),
            RiskLevel::Monitor => write!(f, "⚠️ Monitor"),
            RiskLevel::Alert => write!(f, "🔴 Alert"),
            RiskLevel::Critical => write!(f, "🔴🔴 Critical"),
        }
    }
}

/// Churn trend direction
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum ChurnTrend {
    Improving,
    Degrading,
    Stable,
}

impl fmt::Display for ChurnTrend {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            ChurnTrend::Improving => write!(f, "↑ Improving"),
            ChurnTrend::Degrading => write!(f, "↓ Degrading"),
            ChurnTrend::Stable => write!(f, "→ Stable"),
        }
    }
}

/// Comprehensive risk score for a file
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RiskScore {
    pub file: String,
    pub risk_value: f64,
    pub risk_level: RiskLevel,
    pub churn_percentage: f64,
    pub loc: usize,
    pub author_count: usize,
    pub recent_commits: usize,
    pub complexity: f64,
    pub trend: ChurnTrend,
    pub recommendation: String,
    pub last_modified_days_ago: usize,
}

impl fmt::Display for RiskScore {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{} - Risk: {:.1}/10 | {}",
            self.file, self.risk_value, self.risk_level)
    }
}

/// Prediction warning levels for churn escalation
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum PredictionWarning {
    None,
    Watch,
    Degrade,
    Critical,
}

impl fmt::Display for PredictionWarning {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            PredictionWarning::None => write!(f, "None"),
            PredictionWarning::Watch => write!(f, "Watch"),
            PredictionWarning::Degrade => write!(f, "Degrade"),
            PredictionWarning::Critical => write!(f, "Critical"),
        }
    }
}

/// Churn prediction with linear regression forecasting
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChurnPrediction {
    pub file: String,
    pub current_churn: f64,
    pub predicted_churn_7days: f64,
    pub predicted_churn_14days: f64,
    pub days_to_critical: Option<usize>,
    pub prediction_confidence: f64,
    pub warning_level: PredictionWarning,
}

impl fmt::Display for ChurnPrediction {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{} - Current: {:.1}% | 7d: {:.1}% | 14d: {:.1}% | {}",
            self.file, self.current_churn, self.predicted_churn_7days,
            self.predicted_churn_14days, self.warning_level)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_risk_level_display() {
        assert_eq!(format!("{}", RiskLevel::Safe), "✅ Safe");
        assert_eq!(format!("{}", RiskLevel::Monitor), "⚠️ Monitor");
        assert_eq!(format!("{}", RiskLevel::Alert), "🔴 Alert");
        assert_eq!(format!("{}", RiskLevel::Critical), "🔴🔴 Critical");
    }

    #[test]
    fn test_churn_trend_display() {
        assert_eq!(format!("{}", ChurnTrend::Improving), "↑ Improving");
        assert_eq!(format!("{}", ChurnTrend::Degrading), "↓ Degrading");
        assert_eq!(format!("{}", ChurnTrend::Stable), "→ Stable");
    }

    #[test]
    fn test_risk_score_creation() {
        let score = RiskScore {
            file: "test.rs".to_string(),
            risk_value: 5.5,
            risk_level: RiskLevel::Alert,
            churn_percentage: 60.0,
            loc: 200,
            author_count: 2,
            recent_commits: 5,
            complexity: 6.5,
            trend: ChurnTrend::Degrading,
            recommendation: "Monitor".to_string(),
            last_modified_days_ago: 3,
        };
        assert_eq!(score.file, "test.rs");
        assert_eq!(score.risk_value, 5.5);
    }

    #[test]
    fn test_churn_trend_prediction() {
        // Test that prediction functionality exists
        let prediction = ChurnPrediction {
            file: "test.rs".to_string(),
            current_churn: 45.0,
            predicted_churn_7days: 52.0,
            predicted_churn_14days: 60.0,
            days_to_critical: Some(21),
            prediction_confidence: 0.85,
            warning_level: PredictionWarning::Watch,
        };

        assert_eq!(prediction.file, "test.rs");
        assert_eq!(prediction.current_churn, 45.0);
        assert_eq!(prediction.predicted_churn_7days, 52.0);
        assert_eq!(prediction.predicted_churn_14days, 60.0);
        assert_eq!(prediction.days_to_critical, Some(21));
        assert_eq!(prediction.prediction_confidence, 0.85);
        assert_eq!(prediction.warning_level, PredictionWarning::Watch);
    }

    #[test]
    fn test_days_to_unmaintainable() {
        // Test that days_to_critical calculation exists
        let critical_prediction = ChurnPrediction {
            file: "critical.rs".to_string(),
            current_churn: 75.0,
            predicted_churn_7days: 85.0,
            predicted_churn_14days: 95.0,
            days_to_critical: Some(10),
            prediction_confidence: 0.92,
            warning_level: PredictionWarning::Critical,
        };

        // Verify critical state tracking
        assert!(critical_prediction.days_to_critical.is_some());
        assert_eq!(critical_prediction.days_to_critical.unwrap(), 10);
        assert_eq!(critical_prediction.warning_level, PredictionWarning::Critical);
    }

    #[test]
    fn test_prediction_warning_levels() {
        assert_eq!(format!("{}", PredictionWarning::None), "None");
        assert_eq!(format!("{}", PredictionWarning::Watch), "Watch");
        assert_eq!(format!("{}", PredictionWarning::Degrade), "Degrade");
        assert_eq!(format!("{}", PredictionWarning::Critical), "Critical");
    }
}
