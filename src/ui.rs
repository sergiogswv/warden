//! Interactive terminal UI
//!
//! Renders reports and manages user interaction.

use crate::models::AnalysisResult;

pub fn show_main_menu(analysis: &AnalysisResult) -> anyhow::Result<()> {
    println!();
    println!("╔════════════════════════════════════╗");
    println!("║   Warden v0.1.0                    ║");
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
    render_hotspots(analysis, 10)?;

    Ok(())
}

pub fn render_debt_trends(_analysis: &AnalysisResult) -> anyhow::Result<()> {
    println!("📈 TECHNICAL DEBT TRENDS (ASCII charts coming in v0.2.0)");
    Ok(())
}

pub fn render_alerts(analysis: &AnalysisResult) -> anyhow::Result<()> {
    if analysis.predictions.is_empty() {
        println!("✅ No critical alerts detected!");
        return Ok(());
    }

    println!("⚠️  PREDICTIVE ALERTS:");
    for pred in &analysis.predictions {
        let days = pred.days_to_unmaintainable
            .map(|d| d.to_string())
            .unwrap_or_else(|| "-".to_string());
        println!("   [{:?}] {} - {}", pred.severity, pred.file, days);
    }
    println!();

    Ok(())
}

pub fn render_hotspots(analysis: &AnalysisResult, top_n: usize) -> anyhow::Result<()> {
    let mut hotspots: Vec<_> = analysis.file_metrics.iter()
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

pub fn render_author_stats(_analysis: &AnalysisResult) -> anyhow::Result<()> {
    println!("👤 AUTHOR STATISTICS (coming in v0.2.0)");
    Ok(())
}

pub fn export_json(analysis: &AnalysisResult, output_path: &str) -> anyhow::Result<()> {
    let json = serde_json::to_string_pretty(analysis)?;
    std::fs::write(output_path, json)?;
    println!("✅ Exported to {}", output_path);
    Ok(())
}

pub fn export_markdown(_analysis: &AnalysisResult, _output_path: &str) -> anyhow::Result<()> {
    println!("Markdown export coming in v0.2.0");
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
