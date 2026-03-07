//! Risk scoring engine for hotspot detection

use crate::models::*;
use crate::predictor::predict_churn_trajectory;
use chrono::Utc;
use std::collections::HashMap;

/// Detect if a file has undergone recent refactoring based on LOC reduction
/// Returns Some(pct_reduction) if LOC has been reduced by ≥30%, None otherwise
fn detect_refactoring(loc_history: &[LOCMetric]) -> Option<f64> {
    if loc_history.len() < 2 {
        return None;
    }

    let current_loc = loc_history.last().map(|l| l.lines)?;
    let max_loc = loc_history.iter().map(|l| l.lines).max()?;

    if current_loc == 0 || max_loc <= current_loc {
        return None;
    }

    let reduction_pct = (max_loc - current_loc) as f64 / max_loc as f64 * 100.0;

    if reduction_pct >= 30.0 {
        Some(reduction_pct)
    } else {
        None
    }
}

/// Calculate risk scores for all files in metrics
pub fn calculate_risk_scores(
    file_metrics: &HashMap<String, FileMetrics>,
    _total_commits: usize,
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
        let complexity = metrics
            .complexity_history
            .last()
            .map(|c| c.estimated_complexity)
            .unwrap_or(1.0);

        // Cap author impact for small files
        // Fragmenting a 21-line file among 2 authors is not a real risk
        // Formula: author_impact = min(authors, LOC / 20)
        // - 21 LOC, 2 authors: impact = min(2, 1.05) = 1.05 (minimal)
        // - 500 LOC, 5 authors: impact = min(5, 25) = 5 (full impact)
        let author_impact = ((authors as f64).min((loc as f64) / 20.0)).max(1.0);

        let raw_risk = if baseline > 0.0 {
            (churn * loc as f64 * author_impact) / baseline
        } else {
            0.0
        };

        // Calculate days since last modification for decay factor
        let last_modified_days_ago = calculate_days_since_modified(&metrics.churn_history);

        // Count recent commits
        let recent_commits = metrics.authors.iter().map(|a| a.commits).sum::<usize>();

        // Apply size dampening for small files to reduce false positives
        // Small files with high churn shouldn't be marked as Critical
        let mut risk_value = apply_size_dampening(raw_risk, loc);

        // Apply decay factor for old, stable files
        // Files not modified in 30+ days are less risky
        risk_value = apply_stability_decay(risk_value, last_modified_days_ago);

        // Apply recent activity dampening
        // Isolated recent changes (1-2 commits) in small files are usually normal (bugfixes, refactors)
        risk_value = apply_recent_activity_dampening(
            risk_value,
            recent_commits,
            last_modified_days_ago,
            loc,
        );

        // Detect refactoring and apply attenuation if detected
        let refactor_detected = detect_refactoring(&metrics.loc_history);
        if refactor_detected.is_some() {
            // Apply additional attenuation factor × 0.6 for refactored files
            // Refactoring is a positive signal, not a degradation
            risk_value = risk_value * 0.6;
        }

        let risk_level = classify_risk_level(risk_value);
        let trend = detect_churn_trend(&metrics.churn_history);
        let recommendation = generate_recommendation(
            risk_value,
            loc,
            authors,
            &trend,
            &metrics.loc_history,
            refactor_detected,
        );

        // Calculate prediction from churn history
        // Extract churn percentages from churn_history
        let churn_history_values: Vec<f64> = metrics
            .churn_history
            .iter()
            .map(|m| m.churn_percentage)
            .collect();

        // Call predict_churn_trajectory, handle errors gracefully
        let prediction = predict_churn_trajectory(&filename, &churn_history_values).ok();

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
            prediction,
            refactor_detected,
        });
    }

    // Sort by risk (descending)
    risk_scores.sort_by(|a, b| b.risk_value.partial_cmp(&a.risk_value).unwrap());

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
    loc_history: &[LOCMetric],
    refactor_detected: Option<f64>,
) -> String {
    // Case 1: Refactoring detected
    if let Some(pct_reduction) = refactor_detected {
        return format!(
            "Refactoring detected (LOC -{}%). Monitor stabilization",
            pct_reduction as u32
        );
    }

    // Case 2: High churn with growing LOC (instability + expansion = bad)
    if risk > 5.0 && loc > 100 {
        let is_growing = loc_history.len() >= 2
            && loc_history[loc_history.len() - 1].lines > loc_history[0].lines;

        if is_growing {
            return "Growing with high churn - refactor needed".to_string();
        }
    }

    // Case 3: High churn with stable/shrinking LOC (okay, file is under control)
    if risk > 5.0 && loc < 100 {
        return "Monitor - code instability, consider refactoring".to_string();
    }

    // Case 4: Degrading trend (churn increasing)
    if *trend == ChurnTrend::Degrading {
        return "Degrading - churn increasing".to_string();
    }

    // Case 5: Large file with multiple authors and high risk
    if risk > 5.0 && authors > 3 {
        return "Refactor - fragmented ownership + high churn".to_string();
    }

    // Case 6: Very high risk with large file
    if risk > 8.0 && loc > 200 {
        return "Refactor immediately - large, unstable file".to_string();
    }

    // Default
    if risk > 5.0 {
        "Monitor - high churn detected".to_string()
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
    fn test_detect_refactoring() {
        // Test: 30% reduction (threshold) - should detect
        let loc_history = vec![
            LOCMetric {
                file: "test.rs".to_string(),
                timestamp: Utc::now() - chrono::Duration::days(10),
                lines: 100,
            },
            LOCMetric {
                file: "test.rs".to_string(),
                timestamp: Utc::now(),
                lines: 70, // 30% reduction
            },
        ];
        assert_eq!(detect_refactoring(&loc_history), Some(30.0));

        // Test: 56% reduction (like 321→142) - should detect
        let loc_history2 = vec![
            LOCMetric {
                file: "service.ts".to_string(),
                timestamp: Utc::now() - chrono::Duration::days(30),
                lines: 321,
            },
            LOCMetric {
                file: "service.ts".to_string(),
                timestamp: Utc::now(),
                lines: 142, // 56% reduction
            },
        ];
        let result = detect_refactoring(&loc_history2);
        assert!(result.is_some());
        let pct = result.unwrap();
        assert!(pct > 55.0 && pct < 57.0);

        // Test: 20% reduction - should NOT detect
        let loc_history3 = vec![
            LOCMetric {
                file: "test.rs".to_string(),
                timestamp: Utc::now() - chrono::Duration::days(5),
                lines: 100,
            },
            LOCMetric {
                file: "test.rs".to_string(),
                timestamp: Utc::now(),
                lines: 80, // 20% reduction
            },
        ];
        assert_eq!(detect_refactoring(&loc_history3), None);

        // Test: LOC growth - should NOT detect
        let loc_history4 = vec![
            LOCMetric {
                file: "test.rs".to_string(),
                timestamp: Utc::now() - chrono::Duration::days(5),
                lines: 100,
            },
            LOCMetric {
                file: "test.rs".to_string(),
                timestamp: Utc::now(),
                lines: 150, // 50% increase
            },
        ];
        assert_eq!(detect_refactoring(&loc_history4), None);

        // Test: Single point history - should NOT detect
        let loc_history5 = vec![LOCMetric {
            file: "test.rs".to_string(),
            timestamp: Utc::now(),
            lines: 100,
        }];
        assert_eq!(detect_refactoring(&loc_history5), None);
    }

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
        // Test: Large, unstable file
        let rec1 = generate_recommendation(9.0, 300, 2, &ChurnTrend::Stable, &[], None);
        assert!(rec1.contains("Refactor immediately"));

        // Test: Safe, stable file
        let rec3 = generate_recommendation(1.0, 50, 1, &ChurnTrend::Stable, &[], None);
        assert!(rec3.contains("Safe"));

        // Test: Refactoring detected
        let loc_history = vec![
            LOCMetric {
                file: "test.rs".to_string(),
                timestamp: Utc::now(),
                lines: 200,
            },
            LOCMetric {
                file: "test.rs".to_string(),
                timestamp: Utc::now(),
                lines: 100,
            },
        ];
        let rec_refactor = generate_recommendation(
            5.0,
            100,
            2,
            &ChurnTrend::Stable,
            &loc_history,
            Some(50.0),
        );
        assert!(rec_refactor.contains("Refactoring detected"));
    }

    #[test]
    fn test_risk_scores_sorted_descending() {
        let mut metrics = HashMap::new();

        // Create test metrics with enough history for prediction
        let m1 = FileMetrics {
            file: "high_risk.rs".to_string(),
            loc_history: vec![],
            churn_history: vec![
                ChurnMetric {
                    file: "high_risk.rs".to_string(),
                    timestamp: Utc::now() - chrono::Duration::days(1),
                    churn_percentage: 60.0,
                },
                ChurnMetric {
                    file: "high_risk.rs".to_string(),
                    timestamp: Utc::now(),
                    churn_percentage: 80.0,
                },
            ],
            authors: vec![],
            complexity_history: vec![],
        };

        let m2 = FileMetrics {
            file: "low_risk.rs".to_string(),
            loc_history: vec![],
            churn_history: vec![
                ChurnMetric {
                    file: "low_risk.rs".to_string(),
                    timestamp: Utc::now() - chrono::Duration::days(1),
                    churn_percentage: 15.0,
                },
                ChurnMetric {
                    file: "low_risk.rs".to_string(),
                    timestamp: Utc::now(),
                    churn_percentage: 20.0,
                },
            ],
            authors: vec![],
            complexity_history: vec![],
        };

        metrics.insert("high_risk.rs".to_string(), m1);
        metrics.insert("low_risk.rs".to_string(), m2);

        let scores = calculate_risk_scores(&metrics, 10).unwrap();

        // Verify sorting: first should have higher risk than second
        assert!(scores[0].risk_value >= scores[1].risk_value);
        // Verify prediction field exists for both
        assert!(scores[0].prediction.is_some());
        assert!(scores[1].prediction.is_some());
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

    #[test]
    fn test_author_impact_capping() {
        // Test that author count is capped for small files
        // Fragmenting a small file is not a real risk

        // Case 1: poliza-liverpool.tsx scenario
        // 21 LOC, 2 authors → impact = min(2, 21/20) = min(2, 1.05) = 1.05
        // raw_risk = (100 × 21 × 1.05) / baseline
        // If baseline = 400, risk = 2205 / 400 = 5.5, then with stability decay → ~1.5
        // This file should be Safe, not Monitor

        // Case 2: Medium file, multiple authors
        // 100 LOC, 4 authors → impact = min(4, 100/20) = min(4, 5) = 4
        // Normal impact (no capping)

        // Case 3: Large file, many authors
        // 500 LOC, 6 authors → impact = min(6, 500/20) = min(6, 25) = 6
        // Full impact

        // Testing the capping directly:
        // Small file: 21 LOC
        let impact_tiny = ((2_f64).min((21_f64) / 20.0)).max(1.0);
        // min(2, 1.05) = 1.05
        assert!(impact_tiny > 1.0);
        assert!(impact_tiny < 1.1);

        // Medium file: 100 LOC
        let impact_medium = ((4_f64).min((100_f64) / 20.0)).max(1.0);
        // min(4, 5) = 4
        assert_eq!(impact_medium, 4.0);

        // Large file: 500 LOC
        let impact_large = ((6_f64).min((500_f64) / 20.0)).max(1.0);
        // min(6, 25) = 6
        assert_eq!(impact_large, 6.0);

        // Single author on any file
        let impact_single = ((1_f64).min((21_f64) / 20.0)).max(1.0);
        // min(1, 1.05) = 1.0
        assert_eq!(impact_single, 1.0);
    }

    // Tests for prediction integration

    #[test]
    fn test_risk_score_includes_prediction_field() {
        // Test that RiskScore struct includes prediction data
        let mut metrics = HashMap::new();

        let m1 = FileMetrics {
            file: "test.rs".to_string(),
            loc_history: vec![],
            churn_history: vec![
                ChurnMetric {
                    file: "test.rs".to_string(),
                    timestamp: Utc::now() - chrono::Duration::days(1),
                    churn_percentage: 30.0,
                },
                ChurnMetric {
                    file: "test.rs".to_string(),
                    timestamp: Utc::now(),
                    churn_percentage: 40.0,
                },
            ],
            authors: vec![],
            complexity_history: vec![],
        };

        metrics.insert("test.rs".to_string(), m1);

        let scores = calculate_risk_scores(&metrics, 10).unwrap();
        assert_eq!(scores.len(), 1);

        // Verify prediction field exists and is populated
        let score = &scores[0];
        assert_eq!(score.file, "test.rs");
        assert!(score.prediction.is_some(), "prediction should be populated");

        let pred = score.prediction.as_ref().unwrap();
        assert_eq!(pred.file, "test.rs");
        assert_eq!(pred.current_churn, 40.0);
        assert!(pred.predicted_churn_7days >= 0.0);
        assert!(pred.predicted_churn_14days >= 0.0);
    }

    #[test]
    fn test_prediction_field_none_with_insufficient_data() {
        // Test that prediction is None if churn history has less than 2 points
        let mut metrics = HashMap::new();

        let m1 = FileMetrics {
            file: "single_point.rs".to_string(),
            loc_history: vec![],
            churn_history: vec![ChurnMetric {
                file: "single_point.rs".to_string(),
                timestamp: Utc::now(),
                churn_percentage: 30.0,
            }],
            authors: vec![],
            complexity_history: vec![],
        };

        metrics.insert("single_point.rs".to_string(), m1);

        let scores = calculate_risk_scores(&metrics, 10).unwrap();
        assert_eq!(scores.len(), 1);

        // Verify prediction is None due to insufficient data
        let score = &scores[0];
        assert!(
            score.prediction.is_none(),
            "prediction should be None with insufficient churn data"
        );
    }

    #[test]
    fn test_prediction_populated_with_sufficient_data() {
        // Test that prediction is populated with sufficient churn history
        let mut metrics = HashMap::new();

        let m1 = FileMetrics {
            file: "adequate.rs".to_string(),
            loc_history: vec![],
            churn_history: vec![
                ChurnMetric {
                    file: "adequate.rs".to_string(),
                    timestamp: Utc::now() - chrono::Duration::days(4),
                    churn_percentage: 20.0,
                },
                ChurnMetric {
                    file: "adequate.rs".to_string(),
                    timestamp: Utc::now() - chrono::Duration::days(3),
                    churn_percentage: 25.0,
                },
                ChurnMetric {
                    file: "adequate.rs".to_string(),
                    timestamp: Utc::now() - chrono::Duration::days(2),
                    churn_percentage: 30.0,
                },
                ChurnMetric {
                    file: "adequate.rs".to_string(),
                    timestamp: Utc::now() - chrono::Duration::days(1),
                    churn_percentage: 35.0,
                },
                ChurnMetric {
                    file: "adequate.rs".to_string(),
                    timestamp: Utc::now(),
                    churn_percentage: 40.0,
                },
            ],
            authors: vec![],
            complexity_history: vec![],
        };

        metrics.insert("adequate.rs".to_string(), m1);

        let scores = calculate_risk_scores(&metrics, 10).unwrap();
        assert_eq!(scores.len(), 1);

        // Verify prediction is fully populated
        let score = &scores[0];
        assert!(score.prediction.is_some());

        let pred = score.prediction.as_ref().unwrap();
        assert_eq!(pred.file, "adequate.rs");
        assert_eq!(pred.current_churn, 40.0);
        assert!(pred.predicted_churn_7days > pred.current_churn);
        assert!(pred.predicted_churn_14days >= pred.predicted_churn_7days);
        assert!(pred.prediction_confidence >= 0.5);
    }

    #[test]
    fn test_existing_risk_fields_unchanged_with_prediction() {
        // Test backward compatibility: all existing risk fields remain unchanged
        let mut metrics = HashMap::new();

        let m1 = FileMetrics {
            file: "backward_compat.rs".to_string(),
            loc_history: vec![
                LOCMetric {
                    file: "backward_compat.rs".to_string(),
                    timestamp: Utc::now(),
                    lines: 250,
                },
            ],
            churn_history: vec![
                ChurnMetric {
                    file: "backward_compat.rs".to_string(),
                    timestamp: Utc::now() - chrono::Duration::days(1),
                    churn_percentage: 50.0,
                },
                ChurnMetric {
                    file: "backward_compat.rs".to_string(),
                    timestamp: Utc::now(),
                    churn_percentage: 65.0,
                },
            ],
            authors: vec![
                AuthorFrequency {
                    file: "backward_compat.rs".to_string(),
                    author: "alice".to_string(),
                    commits: 10,
                    lines_changed: 100,
                },
                AuthorFrequency {
                    file: "backward_compat.rs".to_string(),
                    author: "bob".to_string(),
                    commits: 8,
                    lines_changed: 80,
                },
            ],
            complexity_history: vec![
                ComplexityMetric {
                    file: "backward_compat.rs".to_string(),
                    timestamp: Utc::now(),
                    estimated_complexity: 7.5,
                },
            ],
        };

        metrics.insert("backward_compat.rs".to_string(), m1);

        let scores = calculate_risk_scores(&metrics, 20).unwrap();
        assert_eq!(scores.len(), 1);

        let score = &scores[0];

        // Verify all existing fields are still present and correct
        assert_eq!(score.file, "backward_compat.rs");
        assert!(score.risk_value > 0.0);
        assert!(!format!("{:?}", score.risk_level).is_empty());
        assert_eq!(score.churn_percentage, 65.0);
        assert_eq!(score.loc, 250);
        assert_eq!(score.author_count, 2);
        assert_eq!(score.recent_commits, 18); // sum of commits from all authors
        assert_eq!(score.complexity, 7.5);
        assert!(!format!("{:?}", score.trend).is_empty());
        assert!(!score.recommendation.is_empty());
        assert!(score.last_modified_days_ago <= 1); // should be 0 or 1

        // And verify prediction is also available
        assert!(score.prediction.is_some());
    }

    #[test]
    fn test_multiple_files_all_have_predictions() {
        // Integration test: multiple files all have predictions calculated
        let mut metrics = HashMap::new();

        for i in 1..=3 {
            let filename = format!("file{}.rs", i);
            let churn_values = vec![10.0 + i as f64, 20.0 + i as f64, 30.0 + i as f64];

            let churn_history: Vec<ChurnMetric> = churn_values
                .iter()
                .enumerate()
                .map(|(idx, &val)| ChurnMetric {
                    file: filename.clone(),
                    timestamp: Utc::now() - chrono::Duration::days(2 - idx as i64),
                    churn_percentage: val,
                })
                .collect();

            let m = FileMetrics {
                file: filename.clone(),
                loc_history: vec![],
                churn_history,
                authors: vec![],
                complexity_history: vec![],
            };

            metrics.insert(filename, m);
        }

        let scores = calculate_risk_scores(&metrics, 10).unwrap();
        assert_eq!(scores.len(), 3);

        // Verify all files have predictions
        for score in &scores {
            assert!(
                score.prediction.is_some(),
                "file {} should have prediction",
                score.file
            );
            let pred = score.prediction.as_ref().unwrap();
            assert_eq!(pred.file, score.file);
            assert!(pred.current_churn > 0.0);
        }
    }

    #[test]
    fn test_prediction_warning_levels_integrated() {
        // Test that prediction warning levels are properly set in integrated risk calculation
        let mut metrics = HashMap::new();

        // Create a file with degrading churn (should have warning)
        let degrading_churn: Vec<ChurnMetric> = vec![
            5.0, 15.0, 25.0, 35.0, 45.0, 55.0, 65.0,
        ]
        .into_iter()
        .enumerate()
        .map(|(idx, val)| ChurnMetric {
            file: "degrading.rs".to_string(),
            timestamp: Utc::now() - chrono::Duration::days(6 - idx as i64),
            churn_percentage: val,
        })
        .collect();

        let m1 = FileMetrics {
            file: "degrading.rs".to_string(),
            loc_history: vec![],
            churn_history: degrading_churn,
            authors: vec![],
            complexity_history: vec![],
        };

        metrics.insert("degrading.rs".to_string(), m1);

        let scores = calculate_risk_scores(&metrics, 10).unwrap();
        assert_eq!(scores.len(), 1);

        let score = &scores[0];
        assert!(score.prediction.is_some());

        let pred = score.prediction.as_ref().unwrap();
        // With 65% current churn and increasing trend, should have a warning
        assert_ne!(
            pred.warning_level,
            PredictionWarning::None,
            "degrading file should have warning"
        );
    }

    #[test]
    fn test_refactoring_attenuation_in_risk_calculation() {
        // Test that detected refactoring applies the 0.6 attenuation factor
        let mut metrics = HashMap::new();

        // Create metrics with 56% LOC reduction (like 321→142)
        let m1 = FileMetrics {
            file: "deal-migration.service.ts".to_string(),
            loc_history: vec![
                LOCMetric {
                    file: "deal-migration.service.ts".to_string(),
                    timestamp: Utc::now() - chrono::Duration::days(30),
                    lines: 321,
                },
                LOCMetric {
                    file: "deal-migration.service.ts".to_string(),
                    timestamp: Utc::now(),
                    lines: 142,
                },
            ],
            churn_history: vec![
                ChurnMetric {
                    file: "deal-migration.service.ts".to_string(),
                    timestamp: Utc::now() - chrono::Duration::days(1),
                    churn_percentage: 200.0,
                },
                ChurnMetric {
                    file: "deal-migration.service.ts".to_string(),
                    timestamp: Utc::now(),
                    churn_percentage: 254.0, // High churn
                },
            ],
            authors: vec![],
            complexity_history: vec![],
        };

        metrics.insert("deal-migration.service.ts".to_string(), m1);

        let scores = calculate_risk_scores(&metrics, 10).unwrap();
        assert_eq!(scores.len(), 1);

        let score = &scores[0];

        // Verify refactoring was detected
        assert!(score.refactor_detected.is_some());
        let reduction = score.refactor_detected.unwrap();
        assert!(reduction > 55.0 && reduction < 57.0, "Expected ~56% reduction, got {}", reduction);

        // Verify recommendation mentions refactoring
        assert!(
            score.recommendation.contains("Refactoring detected"),
            "Recommendation should mention refactoring: {}",
            score.recommendation
        );

        // Verify that risk score was attenuated
        // Without refactoring attenuation, with 254% churn and 142 LOC, risk would be much higher
        // With 0.6 attenuation, it should be lower
        // The risk should be in Monitor/Alert range, not Critical
        assert!(
            score.risk_level != RiskLevel::Critical,
            "Refactored file should not be Critical - attenuation not applied?"
        );
    }

    #[test]
    fn test_risk_score_prediction_consistency() {
        // Test that prediction data is consistent with risk_value and trend
        let mut metrics = HashMap::new();

        let m1 = FileMetrics {
            file: "consistency.rs".to_string(),
            loc_history: vec![],
            churn_history: vec![
                ChurnMetric {
                    file: "consistency.rs".to_string(),
                    timestamp: Utc::now() - chrono::Duration::days(1),
                    churn_percentage: 20.0,
                },
                ChurnMetric {
                    file: "consistency.rs".to_string(),
                    timestamp: Utc::now(),
                    churn_percentage: 50.0,
                },
            ],
            authors: vec![],
            complexity_history: vec![],
        };

        metrics.insert("consistency.rs".to_string(), m1);

        let scores = calculate_risk_scores(&metrics, 10).unwrap();
        let score = &scores[0];

        // With high churn increase, trend should be degrading
        assert_eq!(score.trend, ChurnTrend::Degrading);

        // Prediction should also reflect degradation
        assert!(score.prediction.is_some());
        let pred = score.prediction.as_ref().unwrap();
        assert!(
            pred.predicted_churn_14days >= pred.current_churn,
            "degrading trend should predict equal or higher churn"
        );
    }
}
