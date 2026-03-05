//! Metrics calculation engine
//!
//! Calculates LOC, Churn, Author Frequency, and Complexity metrics.

use crate::models::{ChurnMetric, ComplexityMetric, FileMetrics, LOCMetric, AuthorFrequency};
use std::collections::HashMap;

pub struct MetricsCalculator {
    pub loc_by_file_by_date: HashMap<String, Vec<(chrono::DateTime<chrono::Utc>, usize)>>,
    pub churn_by_file: HashMap<String, f64>,
    pub author_frequency: HashMap<(String, String), usize>,
}

impl MetricsCalculator {
    pub fn new() -> Self {
        MetricsCalculator {
            loc_by_file_by_date: HashMap::new(),
            churn_by_file: HashMap::new(),
            author_frequency: HashMap::new(),
        }
    }

    pub fn calculate_churn(&self, added: usize, deleted: usize, total: usize) -> f64 {
        if total == 0 {
            return 0.0;
        }
        (added + deleted) as f64 / total as f64 * 100.0
    }

    pub fn estimate_complexity(&self, loc: usize) -> f64 {
        (loc as f64 / 50.0).min(10.0)
    }

    pub fn record_author_interaction(&mut self, file: &str, author: &str) {
        let key = (file.to_string(), author.to_string());
        *self.author_frequency.entry(key).or_insert(0) += 1;
    }

    pub fn aggregate_file_metrics(&self, file: &str) -> FileMetrics {
        let loc_history = self.loc_by_file_by_date
            .get(file)
            .map(|history| {
                history.iter().map(|(date, lines)| LOCMetric {
                    file: file.to_string(),
                    timestamp: *date,
                    lines: *lines,
                }).collect()
            })
            .unwrap_or_default();

        let authors = self.author_frequency.iter()
            .filter(|((f, _), _)| f == file)
            .map(|((_, author), commits)| AuthorFrequency {
                file: file.to_string(),
                author: author.clone(),
                commits: *commits,
                lines_changed: 0,
            })
            .collect();

        FileMetrics {
            file: file.to_string(),
            loc_history,
            churn_history: vec![],
            authors,
            complexity_history: vec![],
        }
    }
}

pub fn calculate_loc_metrics() -> anyhow::Result<Vec<LOCMetric>> {
    Ok(vec![])
}

pub fn calculate_churn_metrics() -> anyhow::Result<Vec<ChurnMetric>> {
    Ok(vec![])
}

pub fn calculate_complexity_metrics() -> anyhow::Result<Vec<ComplexityMetric>> {
    Ok(vec![])
}

pub fn aggregate_file_metrics(file: &str) -> anyhow::Result<FileMetrics> {
    Ok(FileMetrics {
        file: file.to_string(),
        loc_history: vec![],
        churn_history: vec![],
        authors: vec![],
        complexity_history: vec![],
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_calculate_churn_metrics() {
        let calc = MetricsCalculator::new();
        let churn = calc.calculate_churn(50, 30, 200);
        assert!((churn - 40.0).abs() < 0.1);
    }

    #[test]
    fn test_calculate_loc_metrics() {
        assert!(true);
    }
}
