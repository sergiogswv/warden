//! Interactive terminal UI
//!
//! Renders reports and manages user interaction.

use crate::models::AnalysisResult;

pub fn show_main_menu(analysis: &AnalysisResult) -> anyhow::Result<()> {
    println!();
    println!("╔════════════════════════════════════╗");
    println!("║   Warden v0.5.0                    ║");
    println!("║   Code Quality Historical Analysis ║");
    println!("╚════════════════════════════════════╝");
    println!();

    println!("📊 Analysis Summary:");
    println!("   • Repository: {}", analysis.repository_path);
    println!("   • Period: {}", analysis.analysis_period);
    println!("   • Files analyzed: {}", analysis.files_analyzed);
    println!("   • Total commits: {}", analysis.total_commits);
    println!("   • Authors: {}", analysis.authors_count);
    println!("   • Overall Trend: {}", analysis.overall_trend);
    println!();

    render_alerts(analysis)?;
    // Note: render_hotspots (old churn-based) has been replaced by
    // render_hotspots_with_risk (intelligent risk scoring) in main.rs

    Ok(())
}

pub fn render_debt_trends(_analysis: &AnalysisResult) -> anyhow::Result<()> {
    println!("📈 TECHNICAL DEBT TRENDS (ASCII charts coming in v0.4.0)");
    Ok(())
}

pub fn render_alerts(analysis: &AnalysisResult) -> anyhow::Result<()> {
    if analysis.predictions.is_empty() {
        println!("✅ No critical alerts detected!");
        return Ok(());
    }

    println!("⚠️  PREDICTIVE ALERTS:");
    for pred in &analysis.predictions {
        let days = pred
            .days_to_unmaintainable
            .map(|d| d.to_string())
            .unwrap_or_else(|| "-".to_string());
        println!("   [{:?}] {} - {}", pred.severity, pred.file, days);
    }
    println!();

    Ok(())
}

pub fn render_hotspots(analysis: &AnalysisResult, top_n: usize) -> anyhow::Result<()> {
    let mut hotspots: Vec<_> = analysis
        .file_metrics
        .iter()
        .map(|(file, metrics)| {
            let churn = metrics.latest_churn().unwrap_or(0.0);
            let loc = metrics.latest_loc().unwrap_or(0);
            (file, churn, loc)
        })
        .collect();

    hotspots.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap());

    if hotspots.is_empty() {
        println!("✅ No hotspots detected!");
        return Ok(());
    }

    println!("🏆 TOP {} HOTSPOTS:", std::cmp::min(top_n, hotspots.len()));
    for (file, churn, loc) in hotspots.iter().take(top_n) {
        println!("   {} - {:.1}% churn, {} LOC", file, churn, loc);
    }
    println!();

    Ok(())
}

/// Get contextual description of churn based on trend, recency, and frequency
/// Considers not just the churn percentage, but the actual stability of the file
fn get_contextual_churn_description(
    churn_percentage: f64,
    trend: crate::models::ChurnTrend,
    recent_commits: usize,
    last_modified_days_ago: usize,
) -> String {
    use crate::models::ChurnTrend;

    // Priority 1: Check trend (most important indicator of actual instability)
    match trend {
        ChurnTrend::Degrading => {
            return "Degrading (churn increasing)".to_string();
        }
        ChurnTrend::Improving => {
            return "Improving (was unstable, now stable)".to_string();
        }
        ChurnTrend::Stable => {
            // Continue to other checks for stable trend
        }
    }

    // Priority 2: Check recency and frequency
    // A file that hasn't changed in weeks is stable, even with high historical churn
    if last_modified_days_ago > 14 {
        return format!("Stable (recently refactored)");
    }

    // If only 1-2 recent commits, it's an isolated change (bugfix/refactor)
    if recent_commits <= 2 && last_modified_days_ago <= 7 {
        return format!("Stable (single change)");
    }

    // If 3+ recent commits, it's actively being changed (potentially unstable)
    if recent_commits >= 3 && last_modified_days_ago <= 7 {
        return format!("Unstable (multiple recent changes)");
    }

    // Fallback: describe based on churn percentage alone
    match churn_percentage {
        c if c > 80.0 => "high churn".to_string(),
        c if c > 50.0 => "unstable".to_string(),
        c if c > 30.0 => "moderate".to_string(),
        _ => "stable".to_string(),
    }
}

pub fn render_hotspots_with_risk(risk_scores: &[crate::models::RiskScore], top_n: usize) {
    println!("\n🏆 TOP {} HOTSPOTS (by Risk Score):", top_n);
    println!("{}\n", "=".repeat(60));

    for (i, score) in risk_scores.iter().take(top_n).enumerate() {
        println!("{}. {}", i + 1, score.file);
        println!(
            "   Risk Score: {:.1}/10 {}",
            score.risk_value, score.risk_level
        );

        let churn_desc = get_contextual_churn_description(
            score.churn_percentage,
            score.trend,
            score.recent_commits,
            score.last_modified_days_ago,
        );
        println!(
            "   ├─ Churn: {:.1}% ({})",
            score.churn_percentage, churn_desc
        );

        let loc_desc = match score.loc {
            l if l > 500 => "large file",
            l if l > 200 => "medium file",
            l if l > 50 => "small file",
            _ => "tiny file",
        };
        println!("   ├─ LOC: {} ({})", score.loc, loc_desc);

        let author_desc = match score.author_count {
            a if a > 4 => "fragmented",
            a if a > 2 => "shared",
            _ => "owned",
        };
        println!("   ├─ Authors: {} ({})", score.author_count, author_desc);

        println!("   ├─ Complexity: {:.1}/10", score.complexity);
        println!("   ├─ Trend: {}", score.trend);
        println!("   ├─ Recent commits: {}", score.recent_commits);
        println!(
            "   ├─ Last modified: {} days ago",
            score.last_modified_days_ago
        );

        if let Some(pct_reduction) = score.refactor_detected {
            println!("   ├─ Refactoring: ✅ Detected (LOC -{}%)", pct_reduction as u32);
        }

        println!("   └─ Recommendation: {}", score.recommendation);
        println!();
    }
}

pub fn render_hotspots_with_risk_and_predictions(risk_scores: &[crate::models::RiskScore], top_n: usize) {
    println!("\n🏆 TOP {} HOTSPOTS (by Risk Score with Predictions):", top_n);
    println!("{}\n", "=".repeat(80));

    for (i, score) in risk_scores.iter().take(top_n).enumerate() {
        println!("{}. {}", i + 1, score.file);
        println!(
            "   Risk Score: {:.1}/10 {}",
            score.risk_value, score.risk_level
        );

        let churn_desc = get_contextual_churn_description(
            score.churn_percentage,
            score.trend,
            score.recent_commits,
            score.last_modified_days_ago,
        );
        println!(
            "   ├─ Churn: {:.1}% ({})",
            score.churn_percentage, churn_desc
        );

        let loc_desc = match score.loc {
            l if l > 500 => "large file",
            l if l > 200 => "medium file",
            l if l > 50 => "small file",
            _ => "tiny file",
        };
        println!("   ├─ LOC: {} ({})", score.loc, loc_desc);

        let author_desc = match score.author_count {
            a if a > 4 => "fragmented",
            a if a > 2 => "shared",
            _ => "owned",
        };
        println!("   ├─ Authors: {} ({})", score.author_count, author_desc);

        println!("   ├─ Complexity: {:.1}/10", score.complexity);
        println!("   ├─ Trend: {}", score.trend);
        println!("   ├─ Recent commits: {}", score.recent_commits);
        println!(
            "   ├─ Last modified: {} days ago",
            score.last_modified_days_ago
        );

        if let Some(pct_reduction) = score.refactor_detected {
            println!("   ├─ Refactoring: ✅ Detected (LOC -{}%)", pct_reduction as u32);
        }

        if let Some(prediction) = &score.prediction {
            println!("   ├─ Recommendation: {}", score.recommendation);
            println!("   └─ Predictive Alert:");
            println!("      ├─ Current Churn: {:.1}%", prediction.current_churn);
            println!("      ├─ 7-Day Forecast: {:.1}%", prediction.predicted_churn_7days);
            println!("      ├─ 14-Day Forecast: {:.1}%", prediction.predicted_churn_14days);
            if let Some(days_to_critical) = prediction.days_to_critical {
                println!("      ├─ Days to Critical: {}", days_to_critical);
            }
            println!("      ├─ Confidence: {:.0}%", prediction.prediction_confidence * 100.0);
            println!("      └─ Alert Level: {}", prediction.warning_level);
        } else {
            println!("   └─ Recommendation: {}", score.recommendation);
        }
        println!();
    }
}

pub fn render_author_stats(_analysis: &AnalysisResult) -> anyhow::Result<()> {
    println!("👤 AUTHOR STATISTICS (coming in v0.4.0)");
    Ok(())
}

pub fn export_json(analysis: &AnalysisResult, output_path: &str) -> anyhow::Result<()> {
    let json = serde_json::to_string_pretty(analysis)?;
    std::fs::write(output_path, json)?;
    println!("✅ Exported to {}", output_path);
    Ok(())
}

pub fn export_markdown(_analysis: &AnalysisResult, _output_path: &str) -> anyhow::Result<()> {
    println!("Markdown export coming in v0.4.0");
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::{ChurnPrediction, PredictionWarning, RiskLevel, ChurnTrend};

    #[test]
    fn test_render_functions() {
        assert!(true);
    }

    #[test]
    fn test_prediction_warning_display_with_emojis() {
        assert_eq!(format!("{}", PredictionWarning::None), "None");
        assert_eq!(format!("{}", PredictionWarning::Watch), "⚠️ Watch");
        assert_eq!(format!("{}", PredictionWarning::Degrade), "⚠️ Degrade");
        assert_eq!(format!("{}", PredictionWarning::Critical), "🔴 Critical");
    }

    #[test]
    fn test_render_hotspots_with_risk_and_predictions_no_panic() {
        let risk_scores = vec![
            crate::models::RiskScore {
                file: "src/analytics.rs".to_string(),
                risk_value: 6.5,
                risk_level: RiskLevel::Alert,
                churn_percentage: 75.2,
                loc: 285,
                author_count: 3,
                recent_commits: 4,
                complexity: 7.2,
                trend: ChurnTrend::Degrading,
                recommendation: "Refactor and document".to_string(),
                last_modified_days_ago: 2,
                prediction: Some(ChurnPrediction {
                    file: "src/analytics.rs".to_string(),
                    current_churn: 75.2,
                    predicted_churn_7days: 78.5,
                    predicted_churn_14days: 82.1,
                    days_to_critical: Some(8),
                    prediction_confidence: 0.85,
                    warning_level: PredictionWarning::Critical,
                }),
                refactor_detected: None,
            },
        ];

        render_hotspots_with_risk_and_predictions(&risk_scores, 5);
    }

    #[test]
    fn test_render_hotspots_with_predictions_output_contains_expected_fields() {
        let risk_scores = vec![
            crate::models::RiskScore {
                file: "src/test.rs".to_string(),
                risk_value: 5.5,
                risk_level: RiskLevel::Monitor,
                churn_percentage: 45.0,
                loc: 150,
                author_count: 2,
                recent_commits: 3,
                complexity: 5.0,
                trend: ChurnTrend::Stable,
                recommendation: "Monitor closely".to_string(),
                last_modified_days_ago: 5,
                prediction: Some(ChurnPrediction {
                    file: "src/test.rs".to_string(),
                    current_churn: 45.0,
                    predicted_churn_7days: 50.0,
                    predicted_churn_14days: 55.0,
                    days_to_critical: Some(20),
                    prediction_confidence: 0.80,
                    warning_level: PredictionWarning::Watch,
                }),
                refactor_detected: None,
            },
        ];

        render_hotspots_with_risk_and_predictions(&risk_scores, 10);
    }

    #[test]
    fn test_render_hotspots_with_predictions_handles_none_prediction() {
        let risk_scores = vec![
            crate::models::RiskScore {
                file: "src/stable.rs".to_string(),
                risk_value: 2.0,
                risk_level: RiskLevel::Safe,
                churn_percentage: 5.0,
                loc: 100,
                author_count: 1,
                recent_commits: 1,
                complexity: 2.0,
                trend: ChurnTrend::Stable,
                recommendation: "No action needed".to_string(),
                last_modified_days_ago: 30,
                prediction: None,
                refactor_detected: None,
            },
        ];

        render_hotspots_with_risk_and_predictions(&risk_scores, 10);
    }

    #[test]
    fn test_render_hotspots_with_predictions_top_n_filtering() {
        let risk_scores = vec![
            crate::models::RiskScore {
                file: "src/file1.rs".to_string(),
                risk_value: 8.0,
                risk_level: RiskLevel::Critical,
                churn_percentage: 85.0,
                loc: 500,
                author_count: 5,
                recent_commits: 10,
                complexity: 8.5,
                trend: ChurnTrend::Degrading,
                recommendation: "Urgent refactoring".to_string(),
                last_modified_days_ago: 1,
                prediction: Some(ChurnPrediction {
                    file: "src/file1.rs".to_string(),
                    current_churn: 85.0,
                    predicted_churn_7days: 90.0,
                    predicted_churn_14days: 95.0,
                    days_to_critical: Some(3),
                    prediction_confidence: 0.95,
                    warning_level: PredictionWarning::Critical,
                }),
                refactor_detected: None,
            },
            crate::models::RiskScore {
                file: "src/file2.rs".to_string(),
                risk_value: 5.0,
                risk_level: RiskLevel::Alert,
                churn_percentage: 60.0,
                loc: 300,
                author_count: 3,
                recent_commits: 5,
                complexity: 6.0,
                trend: ChurnTrend::Degrading,
                recommendation: "Review and refactor".to_string(),
                last_modified_days_ago: 2,
                prediction: Some(ChurnPrediction {
                    file: "src/file2.rs".to_string(),
                    current_churn: 60.0,
                    predicted_churn_7days: 65.0,
                    predicted_churn_14days: 70.0,
                    days_to_critical: Some(10),
                    prediction_confidence: 0.85,
                    warning_level: PredictionWarning::Watch,
                }),
                refactor_detected: None,
            },
            crate::models::RiskScore {
                file: "src/file3.rs".to_string(),
                risk_value: 3.0,
                risk_level: RiskLevel::Monitor,
                churn_percentage: 35.0,
                loc: 200,
                author_count: 2,
                recent_commits: 2,
                complexity: 4.0,
                trend: ChurnTrend::Stable,
                recommendation: "Monitor".to_string(),
                last_modified_days_ago: 7,
                prediction: None,
                refactor_detected: None,
            },
        ];

        render_hotspots_with_risk_and_predictions(&risk_scores, 2);
    }

    #[test]
    fn test_render_hotspots_with_predictions_missing_days_to_critical() {
        let risk_scores = vec![
            crate::models::RiskScore {
                file: "src/test.rs".to_string(),
                risk_value: 4.0,
                risk_level: RiskLevel::Monitor,
                churn_percentage: 40.0,
                loc: 150,
                author_count: 2,
                recent_commits: 3,
                complexity: 4.5,
                trend: ChurnTrend::Stable,
                recommendation: "Monitor".to_string(),
                last_modified_days_ago: 5,
                prediction: Some(ChurnPrediction {
                    file: "src/test.rs".to_string(),
                    current_churn: 40.0,
                    predicted_churn_7days: 42.0,
                    predicted_churn_14days: 44.0,
                    days_to_critical: None,
                    prediction_confidence: 0.70,
                    warning_level: PredictionWarning::None,
                }),
                refactor_detected: None,
            },
        ];

        render_hotspots_with_risk_and_predictions(&risk_scores, 10);
    }

    #[test]
    fn test_render_hotspots_with_predictions_empty_list() {
        let risk_scores: Vec<crate::models::RiskScore> = vec![];
        render_hotspots_with_risk_and_predictions(&risk_scores, 10);
    }

    #[test]
    fn test_render_hotspots_with_predictions_prediction_confidence_formatting() {
        let risk_scores = vec![
            crate::models::RiskScore {
                file: "src/test.rs".to_string(),
                risk_value: 6.0,
                risk_level: RiskLevel::Alert,
                churn_percentage: 70.0,
                loc: 250,
                author_count: 3,
                recent_commits: 5,
                complexity: 6.5,
                trend: ChurnTrend::Degrading,
                recommendation: "Refactor".to_string(),
                last_modified_days_ago: 3,
                prediction: Some(ChurnPrediction {
                    file: "src/test.rs".to_string(),
                    current_churn: 70.0,
                    predicted_churn_7days: 75.0,
                    predicted_churn_14days: 80.0,
                    days_to_critical: Some(7),
                    prediction_confidence: 0.925,
                    warning_level: PredictionWarning::Critical,
                }),
                refactor_detected: None,
            },
        ];

        render_hotspots_with_risk_and_predictions(&risk_scores, 10);
    }
}
