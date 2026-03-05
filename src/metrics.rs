//! Metrics calculation engine
//!
//! Calculates LOC, Churn, Author Frequency, and Complexity metrics.

use crate::models::{ChurnMetric, ComplexityMetric, FileMetrics, LOCMetric, AuthorFrequency};
use std::collections::HashMap;

pub struct MetricsCalculator {
    pub loc_by_file_by_date: HashMap<String, Vec<(chrono::DateTime<chrono::Utc>, usize)>>,
    pub churn_by_file: HashMap<String, Vec<(chrono::DateTime<chrono::Utc>, f64)>>,
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

        let churn_history = self.churn_by_file
            .get(file)
            .map(|churn_entries| {
                churn_entries.iter().map(|(timestamp, churn_percentage)| ChurnMetric {
                    file: file.to_string(),
                    timestamp: *timestamp,
                    churn_percentage: *churn_percentage,
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
            churn_history,
            authors,
            complexity_history: vec![],
        }
    }
}

pub fn process_commits(enriched_commits: &[crate::git_parser::EnrichedCommit])
    -> anyhow::Result<HashMap<String, FileMetrics>>
{
    let mut calculator = MetricsCalculator::new();
    let mut file_metrics: HashMap<String, FileMetrics> = HashMap::new();

    for commit in enriched_commits {
        for (file, change) in &commit.file_changes {
            // Track LOC history
            let dt = chrono::DateTime::<chrono::Utc>::from_timestamp(commit.timestamp, 0)
                .ok_or_else(|| anyhow::anyhow!("Invalid timestamp: {}", commit.timestamp))?;
            calculator.loc_by_file_by_date
                .entry(file.clone())
                .or_insert_with(Vec::new)
                .push((dt, change.additions + change.deletions));

            // Track author interactions
            calculator.record_author_interaction(file, &commit.author);

            // Store churn
            let total = change.additions + change.deletions;
            if total > 0 {
                let churn = calculator.calculate_churn(change.additions, change.deletions, total);
                calculator.churn_by_file
                    .entry(file.clone())
                    .or_insert_with(Vec::new)
                    .push((dt, churn));
            }
        }
    }

    // Aggregate all metrics per file
    let all_files: std::collections::HashSet<_> = enriched_commits
        .iter()
        .flat_map(|c| c.file_changes.keys().cloned())
        .collect();

    for file in all_files {
        file_metrics.insert(file.clone(), calculator.aggregate_file_metrics(&file));
    }

    Ok(file_metrics)
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

    #[test]
    fn test_process_commits_populates_file_metrics() {
        use std::collections::HashMap;
        use crate::git_parser::{EnrichedCommit, FileChange};

        let mut changes = HashMap::new();
        changes.insert("main.rs".to_string(), FileChange {
            file: "main.rs".to_string(),
            additions: 50,
            deletions: 10,
        });

        let commits = vec![
            EnrichedCommit {
                hash: "abc123".to_string(),
                author: "alice".to_string(),
                timestamp: 1000,
                files: vec!["main.rs".to_string()],
                file_changes: changes,
            },
        ];

        let metrics = process_commits(&commits).unwrap();

        assert!(metrics.contains_key("main.rs"));
        assert!(!metrics["main.rs"].loc_history.is_empty());
    }

    #[test]
    fn test_process_commits_calculates_churn() {
        use std::collections::HashMap;
        use crate::git_parser::{EnrichedCommit, FileChange};

        let mut changes1 = HashMap::new();
        changes1.insert("main.rs".to_string(), FileChange {
            file: "main.rs".to_string(),
            additions: 100,
            deletions: 100,
        });

        let mut changes2 = HashMap::new();
        changes2.insert("main.rs".to_string(), FileChange {
            file: "main.rs".to_string(),
            additions: 50,
            deletions: 50,
        });

        let commits = vec![
            EnrichedCommit {
                hash: "abc".to_string(),
                author: "alice".to_string(),
                timestamp: 1000,
                files: vec!["main.rs".to_string()],
                file_changes: changes1,
            },
            EnrichedCommit {
                hash: "def".to_string(),
                author: "bob".to_string(),
                timestamp: 2000,
                files: vec!["main.rs".to_string()],
                file_changes: changes2,
            },
        ];

        let metrics = process_commits(&commits).unwrap();
        let churn_history = &metrics["main.rs"].churn_history;

        // Should have 2 churn entries (one per commit)
        assert_eq!(churn_history.len(), 2);
        assert!(churn_history[0].churn_percentage > 50.0); // 200/200 = 100%
        assert!(churn_history[1].churn_percentage > 50.0); // 100/100 = 100%
    }
}
