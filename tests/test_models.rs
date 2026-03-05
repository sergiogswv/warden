#[cfg(test)]
mod model_tests {
    use chrono::Utc;
    use warden::models::*;

    #[test]
    fn test_loc_metric_display() {
        let metric = LOCMetric {
            file: "src/main.rs".to_string(),
            timestamp: Utc::now(),
            lines: 150,
        };
        let display = format!("{}", metric);
        assert!(display.contains("src/main.rs"));
        assert!(display.contains("150 LOC"));
    }

    #[test]
    fn test_churn_metric_display() {
        let metric = ChurnMetric {
            file: "test.rs".to_string(),
            timestamp: Utc::now(),
            churn_percentage: 45.5,
        };
        let display = format!("{}", metric);
        assert!(display.contains("test.rs"));
        assert!(display.contains("45.5%"));
    }

    #[test]
    fn test_file_metrics_latest_loc() {
        let file_metrics = FileMetrics {
            file: "test.rs".to_string(),
            loc_history: vec![
                LOCMetric { file: "test.rs".to_string(), timestamp: Utc::now(), lines: 100 },
                LOCMetric { file: "test.rs".to_string(), timestamp: Utc::now(), lines: 150 },
            ],
            churn_history: vec![],
            authors: vec![],
            complexity_history: vec![],
        };
        assert_eq!(file_metrics.latest_loc(), Some(150));
    }

    #[test]
    fn test_file_metrics_latest_churn() {
        let file_metrics = FileMetrics {
            file: "test.rs".to_string(),
            loc_history: vec![],
            churn_history: vec![
                ChurnMetric { file: "test.rs".to_string(), timestamp: Utc::now(), churn_percentage: 30.0 },
                ChurnMetric { file: "test.rs".to_string(), timestamp: Utc::now(), churn_percentage: 60.0 },
            ],
            authors: vec![],
            complexity_history: vec![],
        };
        assert_eq!(file_metrics.latest_churn(), Some(60.0));
    }

    #[test]
    fn test_alert_severity_ordering() {
        assert!(AlertSeverity::Info < AlertSeverity::Warning);
        assert!(AlertSeverity::Warning < AlertSeverity::Critical);
    }

    #[test]
    fn test_trend_display() {
        let improving = Trend::Improving;
        let stable = Trend::Stable;
        let degrading = Trend::Degrading;

        assert!(format!("{}", improving).contains("✅"));
        assert!(format!("{}", stable).contains("→"));
        assert!(format!("{}", degrading).contains("⚠️"));
    }
}
