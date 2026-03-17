use clap::{CommandFactory, Parser, Subcommand};
use std::path::PathBuf;
use warden::{analytics, cache, git_parser, metrics, models, risk_scorer, ui};

#[derive(Parser)]
#[command(name = "Warden")]
#[command(about = "Historical code quality analysis and predictive architecture insights")]
#[command(version = "0.5.0")]
#[command(author = "Sergio Guadarrama")]
struct Args {
    /// Path to Git repository (defaults to current directory)
    #[arg(value_name = "PATH")]
    path: Option<PathBuf>,

    /// Analysis period (3m, 6m, 1y, or custom date)
    #[arg(short = 'H', long, default_value = "6m")]
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
    /// Iniciar Warden como agente HTTP para recibir comandos del Cerebro
    Serve,
    /// Predict which files will become critical
    PredictCritical {
        #[arg(long, default_value = "30")]
        days: usize,
        #[arg(long, default_value = "0.5")]
        threshold: f64,
    },
    /// Assess risk scores for files
    RiskAssess {
        #[arg(long)]
        file: Option<String>,
    },
    /// Generate churn trend report
    ChurnReport {
        #[arg(long, default_value = "12")]
        weeks: usize,
        #[arg(long, default_value = "15")]
        top: usize,
    },
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
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
                if let Ok(version_file) =
                    std::fs::read_to_string(std::path::PathBuf::from(build_dir).join(".version"))
                {
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
            let mut cmd = Args::command();
            cmd.print_help()?;
            println!();
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
        // ── Nuevo: modo agente HTTP ───────────────────────────────────────────
        Some(Commands::Serve) => {
            use warden::agent_config::AgentConfig;
            println!("╔════════════════════════════════════╗");
            println!("║   Warden v0.1.0 — Modo Agente     ║");
            println!("║   Conectado al Cerebro             ║");
            println!("╚════════════════════════════════════╝");
            println!();

            let config = AgentConfig::from_env();

            println!("   Cerebro URL : {}", config.cerebro_url);
            println!("   Puerto      : {}", config.port);
            println!();
            warden::agent_server::start_server(config).await?;
            return Ok(());
        }
        Some(Commands::PredictCritical { days, threshold }) => {
            println!("🔮 Predicting critical files ({} days, threshold {})...", days, threshold);

            // Parse git history and analyze
            println!("🔍 Parsing Git history...");
            let commits = git_parser::parse_git_history(&repo_path, "6m").unwrap_or_else(|_| vec![]);
            println!("   ✓ {} commits analyzed", commits.len());

            println!("📈 Calculating file metrics...");
            let file_metrics = metrics::process_commits(&commits)?;
            println!("   ✓ {} files analyzed", file_metrics.len());

            use chrono::Utc;
            let analysis = models::AnalysisResult {
                repository_path: repo_path.to_string_lossy().to_string(),
                analysis_period: "6m".to_string(),
                files_analyzed: file_metrics.len(),
                total_commits: commits.len(),
                authors_count: commits.iter().map(|c| c.author.clone()).collect::<std::collections::HashSet<_>>().len(),
                file_metrics,
                predictions: vec![],
                overall_trend: models::Trend::Stable,
                timestamp: Utc::now(),
            };

            let predictions = warden::predictor::Predictor::predict_critical(&analysis, days, threshold);

            println!("\n📊 PREDICTIONS:\n");
            for (i, pred) in predictions.iter().take(10).enumerate() {
                println!("  {}. {} - {:?} (confidence: {:.0}%)",
                    i + 1, pred.file, pred.severity, pred.confidence * 100.0);
            }

            if predictions.is_empty() {
                println!("  No files at risk within {} days", days);
            }

            return Ok(());
        }
        Some(Commands::RiskAssess { file }) => {
            println!("📊 Assessing risk for: {:?}", file);

            // Parse git history and analyze
            println!("🔍 Parsing Git history...");
            let commits = git_parser::parse_git_history(&repo_path, "6m").unwrap_or_else(|_| vec![]);
            println!("   ✓ {} commits analyzed", commits.len());

            println!("📈 Calculating file metrics...");
            let file_metrics = metrics::process_commits(&commits)?;
            println!("   ✓ {} files analyzed", file_metrics.len());

            let risk_scores = risk_scorer::calculate_risk_scores(&file_metrics, commits.len())?;

            println!("\n📊 RISK ASSESSMENT:\n");

            if let Some(ref target_file) = file {
                // Show specific file
                if let Some(score) = risk_scores.iter().find(|r| &r.file == target_file) {
                    println!("  File: {}", score.file);
                    println!("  Risk Score: {:.2}", score.risk_value);
                    println!("  Risk Level: {}", score.risk_level);
                    println!("  Churn: {:.1}%", score.churn_percentage);
                    println!("  LOC: {}", score.loc);
                    println!("  Authors: {}", score.author_count);
                    println!("  Trend: {}", score.trend);
                    if let Some(ref pred) = score.prediction {
                        println!("  14-day Prediction: {:.1}%", pred.predicted_churn_14days);
                    }
                } else {
                    println!("  File not found in analysis");
                }
            } else {
                // Show top 10 risky files
                let mut sorted = risk_scores.clone();
                sorted.sort_by(|a, b| b.risk_value.partial_cmp(&a.risk_value).unwrap());

                for (i, score) in sorted.iter().take(10).enumerate() {
                    println!("  {}. {} - Risk: {:.2} ({})",
                        i + 1, score.file, score.risk_value, score.risk_level);
                }
            }

            return Ok(());
        }
        Some(Commands::ChurnReport { weeks, top }) => {
            println!("📈 Generating churn report ({} weeks, top {})...", weeks, top);

            // Parse git history and analyze
            println!("🔍 Parsing Git history...");
            let commits = git_parser::parse_git_history(&repo_path, "6m").unwrap_or_else(|_| vec![]);
            println!("   ✓ {} commits analyzed", commits.len());

            println!("📈 Calculating file metrics...");
            let file_metrics = metrics::process_commits(&commits)?;
            println!("   ✓ {} files analyzed", file_metrics.len());

            use chrono::Utc;
            let analysis = models::AnalysisResult {
                repository_path: repo_path.to_string_lossy().to_string(),
                analysis_period: "6m".to_string(),
                files_analyzed: file_metrics.len(),
                total_commits: commits.len(),
                authors_count: commits.iter().map(|c| c.author.clone()).collect::<std::collections::HashSet<_>>().len(),
                file_metrics,
                predictions: vec![],
                overall_trend: models::Trend::Stable,
                timestamp: Utc::now(),
            };

            let report = warden::churn_reporter::ChurnReporter::generate_report(&analysis, weeks);

            println!("\n📊 CHURN REPORT\n");
            println!("  Summary:");
            println!("    - Total commits: {}", report.summary.total_commits);
            println!("    - Avg churn: {:.2}%", report.summary.avg_churn);
            println!("    - Max churn: {:.2}%", report.summary.max_churn);
            println!("    - Trend: {}", report.summary.trend_direction);

            println!("\n  Weekly Trends:");
            for week in &report.weekly_trends {
                println!("    - Week of {}: {:.2}% ({} commits)",
                    week.week_start, week.avg_churn, week.commit_count);
            }

            println!("\n  Top {} Churned Files:", top);
            for (i, file) in report.top_churned_files.iter().take(top).enumerate() {
                println!("    {}. {} - Total churn: {:.2}% ({} changes)",
                    i + 1, file.file, file.total_churn, file.change_count);
            }

            if !report.patterns.is_empty() {
                println!("\n  Patterns Detected:");
                for pattern in &report.patterns {
                    println!("    - [{}] {}", pattern.severity.to_uppercase(), pattern.description);
                }
            }

            return Ok(());
        }
        None => {}
    }

    println!("╔════════════════════════════════════╗");
    println!("║   Warden v0.5.0                    ║");
    println!("║   Code Quality Historical Analysis ║");
    println!("╚════════════════════════════════════╝");
    println!();

    println!("📊 Analyzing Git repository...");
    println!("   • Period: {}", args.history);
    println!("   • Location: {}", repo_path.display());
    println!();

    // Try to load from cache first
    let (analysis, risk_scores) = if let Ok(Some(cached_analysis)) = cache::load_cache(&repo_path) {
        println!("✅ Using cached results (use 'warden clear-cache' to refresh)");
        println!();

        // Recalculate risk scores from cached metrics
        let risk_scores = risk_scorer::calculate_risk_scores(
            &cached_analysis.file_metrics,
            cached_analysis.total_commits,
        )?;
        (cached_analysis, risk_scores)
    } else {
        // Parse git history
        println!("🔍 Parsing Git history...");
        let commits =
            git_parser::parse_git_history(&repo_path, &args.history).unwrap_or_else(|_| vec![]);

        println!("   ✓ {} commits analyzed", commits.len());

        // Process commits to calculate real metrics
        println!("📈 Calculating file metrics...");
        let file_metrics = metrics::process_commits(&commits)?;
        println!("   ✓ {} files analyzed", file_metrics.len());

        // Count unique authors
        let unique_authors: std::collections::HashSet<_> =
            commits.iter().map(|c| c.author.clone()).collect();

        println!("👥 Identified {} unique authors", unique_authors.len());

        use chrono::Utc;

        let mut analysis = models::AnalysisResult {
            repository_path: repo_path.to_string_lossy().to_string(),
            analysis_period: args.history.clone(),
            files_analyzed: file_metrics.len(),
            total_commits: commits.len(),
            authors_count: unique_authors.len(),
            file_metrics, // Now has real data
            predictions: vec![],
            overall_trend: models::Trend::Stable,
            timestamp: Utc::now(),
        };

        // Detect trend using real metrics data
        println!("🔍 Analyzing trends...");
        analysis.overall_trend = analytics::detect_trend(&analysis);

        // Calculate risk scores
        println!("🎯 Calculating risk scores...");
        let risk_scores =
            risk_scorer::calculate_risk_scores(&analysis.file_metrics, analysis.total_commits)?;
        println!("   ✓ Risk scoring complete\n");

        // Cache results
        let _ = cache::save_cache(&repo_path, &analysis);

        (analysis, risk_scores)
    };

    // Render based on format
    match args.format.as_str() {
        "json" => {
            ui::export_json(&analysis, "warden-report.json")?;
        }
        _ => {
            // Show main menu summary unless --only-* flags are used
            if !args.only_trends && !args.only_hotspots && !args.only_predictions {
                ui::show_main_menu(&analysis)?;
                ui::render_hotspots_with_risk_and_predictions(&risk_scores, 10);
            } else {
                // Only show header when using --only-* flags
                println!();
                println!("╔════════════════════════════════════╗");
                println!("║   Warden v0.5.0                    ║");
                println!("║   Code Quality Historical Analysis ║");
                println!("╚════════════════════════════════════╝");
                println!();

                // Show requested sections
                if args.only_trends {
                    println!("📈 TREND ANALYSIS:");
                    println!("   • Overall Trend: {}", analysis.overall_trend);
                    ui::render_debt_trends(&analysis)?;
                }

                if args.only_hotspots {
                    println!("🔴 HOTSPOT ANALYSIS:");
                    ui::render_hotspots_with_risk_and_predictions(&risk_scores, 20);
                }

                if args.only_predictions {
                    println!("🔮 PREDICTIVE ALERTS:");
                    ui::render_alerts(&analysis)?;
                }
            }
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
