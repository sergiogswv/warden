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

        // Calculate days since last modification for decay factor
        let last_modified_days_ago = calculate_days_since_modified(
            &metrics.churn_history
        );

        // Count recent commits
        let recent_commits = metrics.authors.iter()
            .map(|a| a.commits)
            .sum::<usize>();

        // Apply size dampening for small files to reduce false positives
        // Small files with high churn shouldn't be marked as Critical
        let mut risk_value = apply_size_dampening(raw_risk, loc);

        // Apply decay factor for old, stable files
        // Files not modified in 30+ days are less risky
        risk_value = apply_stability_decay(risk_value, last_modified_days_ago);

        // Apply recent activity dampening
        // Isolated recent changes (1-2 commits) in small files are usually normal (bugfixes, refactors)
        risk_value = apply_recent_activity_dampening(risk_value, recent_commits, last_modified_days_ago, loc);

        let risk_level = classify_risk_level(risk_value);
        let trend = detect_churn_trend(&metrics.churn_history);
        let recommendation = generate_recommendation(
            risk_value,
            loc,
            authors,
            &trend,
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

/// Apply stability decay for files not recently modified
/// Files that haven't changed in weeks are less risky
/// Formula: decay_factor = 1 / (1 + days/30)
/// This gradually reduces risk for stable files
fn apply_stability_decay(risk: f64, days_since_modified: usize) -> f64 {
    if days_since_modified == 0 {
        return risk;
    }

    // Decay factor: 1 / (1 + days/30)
    // 30 days:  factor = 0.5 (50% reduction)
    // 60 days:  factor = 0.33 (67% reduction)
    // 90 days:  factor = 0.25 (75% reduction)
    let decay_factor = 1.0 / (1.0 + (days_since_modified as f64) / 30.0);
    (risk * decay_factor).max(0.5) // Never drop below 0.5 (Safe)
}

/// Apply dampening for isolated recent changes
/// A single recent change (1-2 commits) in a small file is usually normal:
/// - Bugfix
/// - Refactor
/// - Feature implementation
/// Not a sign of instability
fn apply_recent_activity_dampening(
    risk: f64,
    recent_commits: usize,
    days_since_modified: usize,
    loc: usize,
) -> f64 {
    // Only apply if:
    // - Modified very recently (< 7 days)
    // - Only 1-2 commits (isolated change, not constant activity)
    // - Small file (< 100 LOC)
    if days_since_modified > 0 && days_since_modified <= 7 && recent_commits <= 2 && loc < 100 {
        // Dampening factor: 0.5 (50% reduction)
        // This acknowledges that isolated recent changes are normal
        // 3.8 * 0.5 = 1.9 (Safe instead of Monitor)
        (risk * 0.5).max(0.5)
    } else {
        risk
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

    #[test]
    fn test_stability_decay_for_old_files() {
        // Test that files not recently modified get decay factor
        // Recent changes (0 days ago)
        let fresh = apply_stability_decay(6.0, 0);
        assert_eq!(fresh, 6.0); // No decay

        // 30 days without modification
        let one_month = apply_stability_decay(6.0, 30);
        // decay factor = 1 / (1 + 30/30) = 0.5
        // expected: 6.0 * 0.5 = 3.0
        assert!(one_month < 3.1);
        assert!(one_month > 2.9);

        // 60 days without modification
        let two_months = apply_stability_decay(6.0, 60);
        // decay factor = 1 / (1 + 60/30) = 0.33
        // expected: 6.0 * 0.33 = 2.0
        assert!(two_months < 2.1);
        assert!(two_months > 1.9);

        // 90 days without modification (like calendar.tsx)
        let three_months = apply_stability_decay(6.0, 90);
        // decay factor = 1 / (1 + 90/30) = 0.25
        // expected: 6.0 * 0.25 = 1.5 (Safe zone!)
        assert!(three_months < 1.6);
        assert!(three_months > 1.4);

        // Verify floor at 0.5 (Safe minimum)
        let very_old = apply_stability_decay(1.0, 365);
        // Should not go below 0.5
        assert_eq!(very_old, 0.5);
    }

    #[test]
    fn test_recent_activity_dampening() {
        // Test isolated recent changes (bugfixes, refactors)

        // Case 1: lead-detail-error.tsx scenario
        // 2 days ago, 1 commit, 21 LOC
        let isolated_recent = apply_recent_activity_dampening(3.8, 1, 2, 21);
        // Should apply 50% dampening: 3.8 * 0.5 = 1.9
        assert!(isolated_recent < 2.0);
        assert!(isolated_recent > 1.8);

        // Case 2: 2 commits in 3 days, 50 LOC
        let two_commits = apply_recent_activity_dampening(5.0, 2, 3, 50);
        // Should apply 50% dampening: 5.0 * 0.5 = 2.5
        assert!(two_commits < 2.6);
        assert!(two_commits > 2.4);

        // Case 3: NO dampening - too many commits
        let many_commits = apply_recent_activity_dampening(3.8, 5, 2, 21);
        // 5 commits = unstable, no dampening
        assert_eq!(many_commits, 3.8);

        // Case 4: NO dampening - old file
        let old_file = apply_recent_activity_dampening(3.8, 1, 30, 21);
        // > 7 days, no dampening
        assert_eq!(old_file, 3.8);

        // Case 5: NO dampening - large file
        let large_file = apply_recent_activity_dampening(3.8, 1, 2, 200);
        // > 100 LOC, no dampening
        assert_eq!(large_file, 3.8);

        // Case 6: Verify floor at 0.5 minimum
        let very_low = apply_recent_activity_dampening(0.8, 1, 2, 21);
        // 0.8 * 0.5 = 0.4, but capped at 0.5
        assert_eq!(very_low, 0.5);
    }
}
