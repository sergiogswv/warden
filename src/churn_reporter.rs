//! Churn trend reporting
//!
//! Analyzes churn evolution over time and identifies patterns.

use crate::models::AnalysisResult;
use chrono::{Duration, Utc};
use std::collections::HashMap;

#[derive(Debug, Clone)]
pub struct ChurnReport {
    pub summary: ChurnSummary,
    pub weekly_trends: Vec<WeeklyChurn>,
    pub top_churned_files: Vec<FileChurn>,
    pub patterns: Vec<Pattern>,
}

#[derive(Debug, Clone)]
pub struct ChurnSummary {
    pub total_commits: usize,
    pub avg_churn: f64,
    pub max_churn: f64,
    pub trend_direction: String, // "increasing", "stable", "decreasing"
}

#[derive(Debug, Clone)]
pub struct WeeklyChurn {
    pub week_start: String,
    pub avg_churn: f64,
    pub commit_count: usize,
    pub most_changed_file: String,
}

#[derive(Debug, Clone)]
pub struct FileChurn {
    pub file: String,
    pub total_churn: f64,
    pub change_count: usize,
}

#[derive(Debug, Clone)]
pub struct Pattern {
    pub description: String,
    pub severity: String, // "info", "warning", "critical"
}

pub struct ChurnReporter;

impl ChurnReporter {
    /// Generate comprehensive churn report
    pub fn generate_report(analysis: &AnalysisResult, weeks: usize) -> ChurnReport {
        let weekly_trends = Self::calculate_weekly_trends(analysis, weeks);
        let top_churned = Self::identify_top_churned(analysis, 15);
        let patterns = Self::detect_patterns(&weekly_trends, &top_churned);
        let summary = Self::generate_summary(analysis, &weekly_trends);

        ChurnReport {
            summary,
            weekly_trends,
            top_churned_files: top_churned,
            patterns,
        }
    }

    fn calculate_weekly_trends(analysis: &AnalysisResult, weeks: usize) -> Vec<WeeklyChurn> {
        let now = Utc::now();
        let mut weekly_data: HashMap<usize, Vec<(f64, String)>> = HashMap::new();

        // Group churn by week
        for (file, metrics) in &analysis.file_metrics {
            for churn in &metrics.churn_history {
                let days_ago = (now - churn.timestamp).num_days();
                let week_num = (days_ago / 7) as usize;

                if week_num < weeks {
                    weekly_data.entry(week_num)
                        .or_insert_with(Vec::new)
                        .push((churn.churn_percentage, file.clone()));
                }
            }
        }

        // Build weekly trends
        let mut trends: Vec<WeeklyChurn> = Vec::new();
        for week_num in 0..weeks {
            let entries = weekly_data.get(&week_num);
            if let Some(entries) = entries {
                if !entries.is_empty() {
                    let avg_churn = entries.iter().map(|(c, _)| c).sum::<f64>() / entries.len() as f64;
                    let most_changed = entries.iter()
                        .map(|(_, f)| f.clone())
                        .fold(HashMap::new(), |mut acc: HashMap<String, usize>, file| {
                            *acc.entry(file).or_insert(0) += 1;
                            acc
                        })
                        .into_iter()
                        .max_by_key(|(_, count)| *count)
                        .map(|(file, _)| file)
                        .unwrap_or_default();

                    trends.push(WeeklyChurn {
                        week_start: (now - Duration::weeks(week_num as i64)).format("%Y-%m-%d").to_string(),
                        avg_churn,
                        commit_count: entries.len(),
                        most_changed_file: most_changed,
                    });
                }
            }
        }

        trends.reverse(); // Oldest first
        trends
    }

    fn identify_top_churned(analysis: &AnalysisResult, top_n: usize) -> Vec<FileChurn> {
        let mut file_churns: Vec<FileChurn> = analysis.file_metrics
            .iter()
            .map(|(file, metrics)| {
                let total_churn = metrics.churn_history.iter()
                    .map(|c| c.churn_percentage)
                    .sum::<f64>();
                let change_count = metrics.churn_history.len();

                FileChurn {
                    file: file.clone(),
                    total_churn,
                    change_count,
                }
            })
            .collect();

        file_churns.sort_by(|a, b| b.total_churn.partial_cmp(&a.total_churn).unwrap());
        file_churns.into_iter().take(top_n).collect()
    }

    fn detect_patterns(weekly: &[WeeklyChurn], top_files: &[FileChurn]) -> Vec<Pattern> {
        let mut patterns = Vec::new();

        // Pattern: Increasing churn trend
        if weekly.len() >= 3 {
            let recent_avg = weekly.iter().rev().take(2).map(|w| w.avg_churn).sum::<f64>() / 2.0;
            let older_avg = weekly.iter().take(2).map(|w| w.avg_churn).sum::<f64>() / 2.0;

            if recent_avg > older_avg * 1.5 {
                patterns.push(Pattern {
                    description: "La tasa de churn está incrementando (>50% vs período anterior)".to_string(),
                    severity: "warning".to_string(),
                });
            }
        }

        // Pattern: Single file dominates changes
        if let Some(top) = top_files.first() {
            let second = top_files.get(1);
            if let Some(second) = second {
                if top.total_churn > second.total_churn * 3.0 {
                    patterns.push(Pattern {
                        description: format!("{} concentra más cambios que ningún otro archivo (posible hotspot)", top.file),
                        severity: "warning".to_string(),
                    });
                }
            }
        }

        // Pattern: High overall churn
        if !weekly.is_empty() {
            let overall_avg = weekly.iter().map(|w| w.avg_churn).sum::<f64>() / weekly.len() as f64;
            if overall_avg > 60.0 {
                patterns.push(Pattern {
                    description: "Churn promedio alto (>60%) - posible deuda técnica acumulada".to_string(),
                    severity: "critical".to_string(),
                });
            }
        }

        patterns
    }

    fn generate_summary(analysis: &AnalysisResult, weekly: &[WeeklyChurn]) -> ChurnSummary {
        let all_churns: Vec<f64> = analysis.file_metrics
            .values()
            .flat_map(|m| m.churn_history.iter().map(|c| c.churn_percentage))
            .collect();

        let avg_churn = if all_churns.is_empty() {
            0.0
        } else {
            all_churns.iter().sum::<f64>() / all_churns.len() as f64
        };

        let max_churn = all_churns.iter().cloned().fold(f64::MIN, f64::max);

        let trend_direction = if weekly.len() >= 2 {
            let recent = weekly.last().map(|w| w.avg_churn).unwrap_or(0.0);
            let older = weekly.first().map(|w| w.avg_churn).unwrap_or(0.0);
            if recent > older * 1.2 {
                "increasing".to_string()
            } else if recent < older * 0.8 {
                "decreasing".to_string()
            } else {
                "stable".to_string()
            }
        } else {
                "stable".to_string()
        };

        ChurnSummary {
            total_commits: analysis.total_commits,
            avg_churn,
            max_churn,
            trend_direction,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::{AnalysisResult, FileMetrics};
    use chrono::{Duration, Utc};
    use std::collections::HashMap;

    #[test]
    fn test_generate_report_structure() {
        let analysis = create_test_analysis();
        let report = ChurnReporter::generate_report(&analysis, 4);

        assert!(report.summary.avg_churn >= 0.0);
    }

    #[test]
    fn test_trend_direction_increasing() {
        let mut analysis = create_test_analysis();
        // Add increasing churn data
        let metrics = analysis.file_metrics.get_mut("test.rs").unwrap();
        metrics.churn_history = vec![
            ChurnMetric { file: "test.rs".to_string(), timestamp: Utc::now() - Duration::weeks(3), churn_percentage: 20.0 },
            ChurnMetric { file: "test.rs".to_string(), timestamp: Utc::now() - Duration::weeks(2), churn_percentage: 40.0 },
            ChurnMetric { file: "test.rs".to_string(), timestamp: Utc::now() - Duration::weeks(1), churn_percentage: 60.0 },
        ];

        let report = ChurnReporter::generate_report(&analysis, 4);
        assert_eq!(report.summary.trend_direction, "increasing");
    }

    fn create_test_analysis() -> AnalysisResult {
        use chrono::Utc;
        use std::collections::HashMap;

        let mut file_metrics = HashMap::new();
        file_metrics.insert("test.rs".to_string(), FileMetrics {
            file: "test.rs".to_string(),
            loc_history: vec![],
            churn_history: vec![
                ChurnMetric { file: "test.rs".to_string(), timestamp: Utc::now() - Duration::days(7), churn_percentage: 30.0 },
            ],
            authors: vec![],
            complexity_history: vec![],
        });

        AnalysisResult {
            repository_path: ".".to_string(),
            analysis_period: "6m".to_string(),
            files_analyzed: 1,
            total_commits: 10,
            authors_count: 1,
            file_metrics,
            predictions: vec![],
            overall_trend: crate::models::Trend::Stable,
            timestamp: Utc::now(),
        }
    }
}
