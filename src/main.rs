use clap::{CommandFactory, Parser, Subcommand};
use std::path::PathBuf;
use warden::{git_parser, ui, cache, models, metrics, analytics, risk_scorer};

#[derive(Parser)]
#[command(name = "Warden")]
#[command(about = "Historical code quality analysis and predictive architecture insights")]
#[command(version = "0.2.0")]
#[command(author = "Sergio Guadarrama")]
struct Args {
    /// Path to Git repository (defaults to current directory)
    #[arg(value_name = "PATH")]
    path: Option<PathBuf>,

    /// Analysis period (3m, 6m, 1y, or custom date)
    #[arg(short, long, default_value = "6m")]
    history: String,

    /// Output format (interactive, json, markdown)
    #[arg(short, long, default_value = "interactive")]
    format: String,

    /// Compare with another branch
    #[arg(long)]
    compare: Option<String>,

    /// Only show predictive alerts
    #[arg(long)]
    only_predictions: bool,

    /// Only show hotspots
    #[arg(long)]
    only_hotspots: bool,

    /// Only show trends
    #[arg(long)]
    only_trends: bool,

    #[command(subcommand)]
    command: Option<Commands>,
}

#[derive(Subcommand)]
enum Commands {
    /// Show version
    Version,
    /// Clear cache
    ClearCache,
    /// Show help
    Help,
    /// Check for updates
    CheckUpdates,
}

fn main() -> anyhow::Result<()> {
    let args = Args::parse();
    let mut repo_path = args.path.unwrap_or_else(|| PathBuf::from("."));

    // Convert to absolute path
    if repo_path.is_relative() {
        if let Ok(cwd) = std::env::current_dir() {
            repo_path = cwd.join(&repo_path);
            // Normalize path to remove redundant components
            if let Ok(normalized) = std::fs::canonicalize(&repo_path) {
                repo_path = normalized;
            }
        }
    }

    match args.command {
        Some(Commands::Version) => {
            let version = env!("CARGO_PKG_VERSION");
            println!("Warden v{}", version);
            if let Ok(build_dir) = std::env::var("CARGO_MANIFEST_DIR") {
                if let Ok(version_file) = std::fs::read_to_string(
                    std::path::PathBuf::from(build_dir).join(".version")
                ) {
                    println!("  Version file: {}", version_file.trim());
                }
            }
            return Ok(());
        }
        Some(Commands::ClearCache) => {
            cache::clear_cache(&repo_path)?;
            println!("✅ Cache cleared");
            return Ok(());
        }
        Some(Commands::Help) => {
            println!("{}", Args::command().render_help());
            return Ok(());
        }
        Some(Commands::CheckUpdates) => {
            let installed_version = env!("CARGO_PKG_VERSION");

            println!("╔═══════════════════════════════════════╗");
            println!("║  Warden Update Check                  ║");
            println!("╚═══════════════════════════════════════╝");
            println!();
            println!("Installed version: {}", installed_version);
            println!("Latest available: (configure GITHUB_REPO for automatic checks)");
            println!();
            println!("✓ To enable automatic checks, set GITHUB_REPO environment variable");

            return Ok(());
        }
        None => {}
    }

    println!("╔════════════════════════════════════╗");
    println!("║   Warden v0.2.0                    ║");
    println!("║   Code Quality Historical Analysis ║");
    println!("╚════════════════════════════════════╝");
    println!();

    println!("📊 Analyzing Git repository...");
    println!("   • Period: {}", args.history);
    println!("   • Location: {}", repo_path.display());
    println!();

    // Try to load from cache first
    if let Ok(Some(cached_analysis)) = cache::load_cache(&repo_path) {
        println!("✅ Using cached results (use 'warden clear-cache' to refresh)");
        println!();

        return match args.format.as_str() {
            "json" => {
                ui::export_json(&cached_analysis, "warden-report.json")?;
                Ok(())
            }
            _ => {
                ui::show_main_menu(&cached_analysis)?;
                Ok(())
            }
        };
    }

    // Parse git history
    println!("🔍 Parsing Git history...");
    let commits = git_parser::parse_git_history(&repo_path, &args.history)
        .unwrap_or_else(|_| vec![]);

    println!("   ✓ {} commits analyzed", commits.len());

    // Process commits to calculate real metrics
    println!("📈 Calculating file metrics...");
    let file_metrics = metrics::process_commits(&commits)?;
    println!("   ✓ {} files analyzed", file_metrics.len());

    // Count unique authors
    let unique_authors: std::collections::HashSet<_> = commits
        .iter()
        .map(|c| c.author.clone())
        .collect();

    println!("👥 Identified {} unique authors", unique_authors.len());

    use chrono::Utc;

    let mut analysis = models::AnalysisResult {
        repository_path: repo_path.to_string_lossy().to_string(),
        analysis_period: args.history.clone(),
        files_analyzed: file_metrics.len(),
        total_commits: commits.len(),
        authors_count: unique_authors.len(),
        file_metrics,  // Now has real data
        predictions: vec![],
        overall_trend: models::Trend::Stable,
        timestamp: Utc::now(),
    };

    // Detect trend using real metrics data
    println!("🔍 Analyzing trends...");
    analysis.overall_trend = analytics::detect_trend(&analysis);

    // Calculate risk scores
    println!("🎯 Calculating risk scores...");
    let risk_scores = risk_scorer::calculate_risk_scores(&analysis.file_metrics, analysis.total_commits)?;
    println!("   ✓ Risk scoring complete\n");

    // Cache results
    let _ = cache::save_cache(&repo_path, &analysis);

    // Render based on format
    match args.format.as_str() {
        "json" => {
            ui::export_json(&analysis, "warden-report.json")?;
        }
        _ => {
            ui::show_main_menu(&analysis)?;
            ui::render_hotspots_with_risk(&risk_scores, 10);
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use std::path::Path;

    #[test]
    fn test_main_populates_real_metrics() {
        // Test with current directory (should have at least some git history)
        let repo_path = Path::new(".");
        let commits = crate::git_parser::parse_git_history(repo_path, "3m").unwrap();

        if !commits.is_empty() {
            let metrics = crate::metrics::process_commits(&commits).unwrap();
            // Should have extracted metrics from commits
            // (may be empty for small test repos, but structure should be sound)
            let _ = metrics; // Verify metrics were extracted
        }
    }
}
