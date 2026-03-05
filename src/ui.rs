//! Interactive terminal UI
//!
//! Renders reports and manages user interaction.

use crate::models::AnalysisResult;

/// Show main menu and handle user interaction
pub fn show_main_menu(analysis: &AnalysisResult) -> anyhow::Result<()> {
    // TODO: Implement interactive menu using dialoguer
    // - Options:
    //   1. 📈 Technical Debt Trends
    //   2. ⚠️  Predictive Alerts
    //   3. 🏆 Top 10 Problem Modules
    //   4. 👤 Author Statistics
    //   5. 🔀 Compare branches
    //   6. ⚙️  Settings
    //   x. Exit

    Ok(())
}

/// Render technical debt trends as ASCII chart
pub fn render_debt_trends(analysis: &AnalysisResult) -> anyhow::Result<()> {
    // TODO: Implement ASCII chart rendering
    // - Use indicatif or custom rendering
    // - Show: Churn Rate, LOC Trend, Complexity Evolution
    // - Include: current, 4-week prediction

    Ok(())
}

/// Render predictive alerts
pub fn render_alerts(analysis: &AnalysisResult) -> anyhow::Result<()> {
    // TODO: Render formatted alerts
    // - Color code by severity (red=critical, yellow=warning)
    // - Show file, metric, prediction, recommended action

    Ok(())
}

/// Render top hotspot modules
pub fn render_hotspots(analysis: &AnalysisResult, top_n: usize) -> anyhow::Result<()> {
    // TODO: Render table of top problematic modules
    // - Columns: File, Churn%, LOC, Complexity, Last Change
    // - Sorted by risk score

    Ok(())
}

/// Render author statistics
pub fn render_author_stats(analysis: &AnalysisResult) -> anyhow::Result<()> {
    // TODO: Render author analysis
    // - Who commits most?
    // - Who touches risky code?
    // - Productivity patterns

    Ok(())
}

/// Export to JSON
pub fn export_json(analysis: &AnalysisResult, output_path: &str) -> anyhow::Result<()> {
    // TODO: Serialize analysis to JSON
    // - Pretty-print for readability

    Ok(())
}

/// Export to Markdown
pub fn export_markdown(analysis: &AnalysisResult, output_path: &str) -> anyhow::Result<()> {
    // TODO: Generate Markdown report
    // - Formatted for sharing/documentation

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_render_functions() {
        // TODO: Add tests
    }
}
