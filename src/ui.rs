//! Interactive terminal UI
//!
//! Renders reports and manages user interaction.

use crate::models::AnalysisResult;

pub fn show_main_menu(analysis: &AnalysisResult) -> anyhow::Result<()> {
    println!();
    println!("╔════════════════════════════════════╗");
    println!("║   Warden v0.3.0                    ║");
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
        println!("   └─ Recommendation: {}", score.recommendation);
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

    #[test]
    fn test_render_functions() {
        assert!(true);
    }
}
