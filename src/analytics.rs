//! Analytics engine
//!
//! Detects trends, identifies hotspots, and analyzes correlations.

use crate::models::{AnalysisResult, Trend};
use std::collections::HashMap;

pub struct AnalyticsEngine;

impl AnalyticsEngine {
    pub fn detect_trend(analysis: &AnalysisResult) -> Trend {
        if analysis.file_metrics.is_empty() {
            return Trend::Stable;
        }

        let mut all_churns = Vec::new();
        for metrics in analysis.file_metrics.values() {
            if let Some(churn) = metrics.latest_churn() {
                all_churns.push(churn);
            }
        }

        if all_churns.len() < 2 {
            return Trend::Stable;
        }

        let avg_churn = all_churns.iter().sum::<f64>() / all_churns.len() as f64;

        if avg_churn > 60.0 {
            Trend::Degrading
        } else if avg_churn < 30.0 {
            Trend::Improving
        } else {
            Trend::Stable
        }
    }

    pub fn identify_hotspots(analysis: &AnalysisResult, top_n: usize) -> Vec<(String, f64)> {
        let mut hotspots = Vec::new();

        for (file, metrics) in &analysis.file_metrics {
            let churn = metrics.latest_churn().unwrap_or(0.0);
            let loc = metrics.latest_loc().unwrap_or(0) as f64;

            let risk_score = (churn / 100.0) * (loc / 100.0) * 100.0;
            hotspots.push((file.clone(), risk_score));
        }

        hotspots.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap());
        hotspots.into_iter().take(top_n).collect()
    }

    pub fn analyze_author_patterns(analysis: &AnalysisResult) -> HashMap<String, Vec<String>> {
        let mut author_files: HashMap<String, Vec<String>> = HashMap::new();

        for metrics in analysis.file_metrics.values() {
            for author_freq in &metrics.authors {
                author_files
                    .entry(author_freq.author.clone())
                    .or_insert_with(Vec::new)
                    .push(author_freq.file.clone());
            }
        }

        author_files
    }
}

pub fn detect_trend(analysis: &AnalysisResult) -> Trend {
    AnalyticsEngine::detect_trend(analysis)
}

pub fn identify_hotspots(analysis: &AnalysisResult, top_n: usize) -> Vec<String> {
    AnalyticsEngine::identify_hotspots(analysis, top_n)
        .into_iter()
        .map(|(file, _)| file)
        .collect()
}

pub fn analyze_author_patterns(analysis: &AnalysisResult) -> anyhow::Result<()> {
    let patterns = AnalyticsEngine::analyze_author_patterns(analysis);
    for (author, files) in patterns {
        println!("{}: touches {} files", author, files.len());
    }
    Ok(())
}

pub fn compare_branches(_branch1: &str, _branch2: &str) -> anyhow::Result<()> {
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::Trend;

    #[test]
    fn test_detect_trend() {
        assert_eq!(AnalyticsEngine::detect_trend(&create_test_analysis()), Trend::Stable);
    }

    #[test]
    fn test_identify_hotspots() {
        let analysis = create_test_analysis();
        let hotspots = AnalyticsEngine::identify_hotspots(&analysis, 5);
        assert!(hotspots.is_empty() || hotspots.len() <= 5);
    }

    fn create_test_analysis() -> AnalysisResult {
        use std::collections::HashMap;
        use chrono::Utc;

        AnalysisResult {
            repository_path: ".".to_string(),
            analysis_period: "6m".to_string(),
            files_analyzed: 0,
            total_commits: 0,
            authors_count: 0,
            file_metrics: HashMap::new(),
            predictions: vec![],
            overall_trend: Trend::Stable,
            timestamp: Utc::now(),
        }
    }
}
