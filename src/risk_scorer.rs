//! Risk scoring engine for hotspot detection

use crate::models::*;
use std::collections::HashMap;
use chrono::Utc;

/// Calculate risk scores for all files in metrics
pub fn calculate_risk_scores(
    file_metrics: &HashMap<String, FileMetrics>,
    total_commits: usize,
) -> anyhow::Result<Vec<RiskScore>> {
    // Calculate baseline (dynamic)
    let mut total_risk = 0.0;
    let file_count = file_metrics.len() as f64;

    for metrics in file_metrics.values() {
        let churn = metrics.latest_churn().unwrap_or(0.0);
        let loc = metrics.latest_loc().unwrap_or(0) as f64;
        let authors = metrics.total_authors() as f64;
        total_risk += churn * loc * authors;
    }

    let baseline = if file_count > 0.0 {
        total_risk / file_count
    } else {
        1.0
    };

    // Calculate RiskScore for each file
    let mut risk_scores = Vec::new();

    for (filename, metrics) in file_metrics {
        let churn = metrics.latest_churn().unwrap_or(0.0);
        let loc = metrics.latest_loc().unwrap_or(0);
        let authors = metrics.total_authors();
        let complexity = metrics.complexity_history
            .last()
            .map(|c| c.estimated_complexity)
            .unwrap_or(1.0);

        let raw_risk = if baseline > 0.0 {
            (churn * loc as f64 * authors as f64) / baseline
        } else {
            0.0
        };

        // Apply size dampening for small files to reduce false positives
        // Small files with high churn shouldn't be marked as Critical
        let risk_value = apply_size_dampening(raw_risk, loc);

        let risk_level = classify_risk_level(risk_value);
        let trend = detect_churn_trend(&metrics.churn_history);
        let recent_commits = metrics.authors.iter()
            .map(|a| a.commits)
            .sum::<usize>();
        let recommendation = generate_recommendation(
            risk_value,
            loc,
            authors,
            &trend,
        );
        let last_modified_days_ago = calculate_days_since_modified(
            &metrics.churn_history
        );

        risk_scores.push(RiskScore {
            file: filename.clone(),
            risk_value,
            risk_level,
            churn_percentage: churn,
            loc,
            author_count: authors,
            recent_commits,
            complexity,
            trend,
            recommendation,
            last_modified_days_ago,
        });
    }

    // Sort by risk (descending)
    risk_scores.sort_by(|a, b| {
        b.risk_value.partial_cmp(&a.risk_value).unwrap()
    });

    Ok(risk_scores)
}

/// Apply size dampening to reduce false positives for small files
/// Small files with high churn shouldn't be marked as Critical
/// Formula: dampening_factor = sqrt(LOC / 50)
/// This reduces risk score for files < 50 LOC while maintaining impact
fn apply_size_dampening(risk: f64, loc: usize) -> f64 {
    const LOC_THRESHOLD: f64 = 50.0;

    if loc < 50 {
        // Dampening factor: sqrt(LOC / 50)
        // 21 LOC → factor = 0.65 (35% reduction)
        // 10 LOC → factor = 0.45 (55% reduction)
        let dampening = ((loc as f64) / LOC_THRESHOLD).sqrt();
        (risk * dampening).min(10.0)
    } else {
        risk.min(10.0)
    }
}

pub fn classify_risk_level(risk: f64) -> RiskLevel {
    match risk {
        r if r < 2.0 => RiskLevel::Safe,
        r if r < 5.0 => RiskLevel::Monitor,
        r if r < 8.0 => RiskLevel::Alert,
        _ => RiskLevel::Critical,
    }
}

fn detect_churn_trend(churn_history: &[ChurnMetric]) -> ChurnTrend {
    if churn_history.len() < 2 {
        return ChurnTrend::Stable;
    }

    let recent = churn_history[churn_history.len() - 1].churn_percentage;
    let old = churn_history[0].churn_percentage;

    if recent > old + 10.0 {
        ChurnTrend::Degrading
    } else if recent < old - 10.0 {
        ChurnTrend::Improving
    } else {
        ChurnTrend::Stable
    }
}

fn generate_recommendation(
    risk: f64,
    loc: usize,
    authors: usize,
    trend: &ChurnTrend,
) -> String {
    if risk > 8.0 && loc > 200 {
        "Refactor immediately - large, unstable file".to_string()
    } else if risk > 5.0 && authors > 3 {
        "Refactor - fragmented ownership + high churn".to_string()
    } else if risk > 5.0 {
        "Monitor - high churn detected".to_string()
    } else if *trend == ChurnTrend::Degrading {
        "Degrading - churn increasing".to_string()
    } else {
        "Safe - stable file".to_string()
    }
}

fn calculate_days_since_modified(churn_history: &[ChurnMetric]) -> usize {
    if let Some(last) = churn_history.last() {
        let now = Utc::now();
        let duration = now.signed_duration_since(last.timestamp);
        duration.num_days().max(0) as usize
    } else {
        999
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_risk_level_classification() {
        assert_eq!(classify_risk_level(0.5), RiskLevel::Safe);
        assert_eq!(classify_risk_level(3.0), RiskLevel::Monitor);
        assert_eq!(classify_risk_level(6.5), RiskLevel::Alert);
        assert_eq!(classify_risk_level(9.0), RiskLevel::Critical);
    }

    #[test]
    fn test_trend_detection() {
        let improving = vec![
            ChurnMetric {
                file: "test.rs".to_string(),
                timestamp: Utc::now() - chrono::Duration::days(30),
                churn_percentage: 60.0,
            },
            ChurnMetric {
                file: "test.rs".to_string(),
                timestamp: Utc::now(),
                churn_percentage: 30.0,
            },
        ];
        assert_eq!(detect_churn_trend(&improving), ChurnTrend::Improving);

        let degrading = vec![
            ChurnMetric {
                file: "test.rs".to_string(),
                timestamp: Utc::now() - chrono::Duration::days(30),
                churn_percentage: 30.0,
            },
            ChurnMetric {
                file: "test.rs".to_string(),
                timestamp: Utc::now(),
                churn_percentage: 60.0,
            },
        ];
        assert_eq!(detect_churn_trend(&degrading), ChurnTrend::Degrading);
    }

    #[test]
    fn test_recommendation_generation() {
        let rec1 = generate_recommendation(9.0, 300, 2, &ChurnTrend::Stable);
        assert!(rec1.contains("Refactor immediately"));

        let rec3 = generate_recommendation(1.0, 50, 1, &ChurnTrend::Stable);
        assert!(rec3.contains("Safe"));
    }

    #[test]
    fn test_risk_scores_sorted_descending() {
        let mut metrics = HashMap::new();

        // Create test metrics
        let m1 = FileMetrics {
            file: "high_risk.rs".to_string(),
            loc_history: vec![],
            churn_history: vec![ChurnMetric {
                file: "high_risk.rs".to_string(),
                timestamp: Utc::now(),
                churn_percentage: 80.0,
            }],
            authors: vec![],
            complexity_history: vec![],
        };

        let m2 = FileMetrics {
            file: "low_risk.rs".to_string(),
            loc_history: vec![],
            churn_history: vec![ChurnMetric {
                file: "low_risk.rs".to_string(),
                timestamp: Utc::now(),
                churn_percentage: 20.0,
            }],
            authors: vec![],
            complexity_history: vec![],
        };

        metrics.insert("high_risk.rs".to_string(), m1);
        metrics.insert("low_risk.rs".to_string(), m2);

        let scores = calculate_risk_scores(&metrics, 10).unwrap();

        // Verify sorting: first should have higher risk than second
        assert!(scores[0].risk_value >= scores[1].risk_value);
    }

    #[test]
    fn test_size_dampening_for_small_files() {
        // Test that small files with high churn are dampened
        // Raw risk before dampening would be high, but should be reduced

        // 21 LOC file (like poliza-liverpool.tsx)
        let risk_small = apply_size_dampening(10.0, 21);
        // dampening factor = sqrt(21/50) = 0.65
        // expected: 10.0 * 0.65 = 6.5
        assert!(risk_small < 7.0);
        assert!(risk_small > 6.0);

        // 10 LOC file (very tiny)
        let risk_tiny = apply_size_dampening(10.0, 10);
        // dampening factor = sqrt(10/50) = 0.45
        // expected: 10.0 * 0.45 = 4.5
        assert!(risk_tiny < 5.0);
        assert!(risk_tiny > 4.0);

        // 50 LOC file (at threshold)
        let risk_threshold = apply_size_dampening(10.0, 50);
        // dampening factor = sqrt(50/50) = 1.0
        // expected: 10.0 (no dampening)
        assert_eq!(risk_threshold, 10.0);

        // 200 LOC file (no dampening)
        let risk_large = apply_size_dampening(10.0, 200);
        // No dampening for large files
        assert_eq!(risk_large, 10.0);
    }
}
