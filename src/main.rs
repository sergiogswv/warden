use clap::{CommandFactory, Parser, Subcommand};
use std::path::PathBuf;
use warden::{git_parser, ui, cache, models, agent_config, agent_server, agent_reporter};

#[derive(Parser)]
#[command(name = "Warden")]
#[command(about = "Historical code quality analysis and predictive architecture insights")]
#[command(version = "0.1.0")]
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
    /// Iniciar Warden como agente HTTP para recibir comandos del Cerebro
    Serve,
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let args = Args::parse();
    let config = agent_config::AgentConfig::from_env();
    let repo_path = args.path.unwrap_or_else(|| PathBuf::from("."));

    match args.command {
        Some(Commands::Version) => {
            println!("Warden v0.1.0");
            return Ok(());
        }
        Some(Commands::ClearCache) => {
            cache::clear_cache(&repo_path)?;
            println!("✅ Cache cleared");
            return Ok(());
        }
        // ── Nuevo: modo agente HTTP ───────────────────────────────────────────
        Some(Commands::Serve) => {
            println!("╔════════════════════════════════════╗");
            println!("║   Warden v0.1.0 — Modo Agente     ║");
            println!("║   Conectado al Cerebro             ║");
            println!("╚════════════════════════════════════╝");
            println!();
            println!("   Cerebro URL : {}", config.cerebro_url);
            println!("   Puerto      : {}", config.port);
            println!();
            agent_server::start_server(config).await?;
            return Ok(());
        }
        None => {}
    }

    println!("╔════════════════════════════════════╗");
    println!("║   Warden v0.1.0                    ║");
    println!("║   Code Quality Historical Analysis ║");
    println!("╚════════════════════════════════════╝");
    println!();

    println!("📊 Analyzing Git repository...");
    println!("   • Period: {}", args.history);
    println!("   • Location: {}", repo_path.display());
    println!();

    // Try to load from cache first
    if let Ok(Some(cached_analysis)) = cache::load_cache(&repo_path) {
        println!("✅ Using cached results (use --clear-cache to refresh)");
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

    use chrono::Utc;
    use std::collections::HashMap;

    let analysis = models::AnalysisResult {
        repository_path: repo_path.to_string_lossy().to_string(),
        analysis_period: args.history,
        files_analyzed: commits.len(),
        total_commits: commits.len(),
        authors_count: 0,
        file_metrics: HashMap::new(),
        predictions: vec![],
        overall_trend: models::Trend::Stable,
        timestamp: Utc::now(),
    };

    // Cache results
    let _ = cache::save_cache(&repo_path, &analysis);

    // ── Reportar resultado al Cerebro ─────────────────────────────────────────
    {
        let mut payload = HashMap::new();
        payload.insert("total_commits".to_string(), serde_json::json!(analysis.total_commits));
        payload.insert("files_analyzed".to_string(), serde_json::json!(analysis.files_analyzed));
        payload.insert("overall_trend".to_string(), serde_json::json!(format!("{:?}", analysis.overall_trend)));
        payload.insert("repository".to_string(), serde_json::json!(analysis.repository_path));

        let _ = agent_reporter::report_event(
            &config,
            "analysis_complete",
            "info",
            payload,
        ).await;
    }

    // Render based on format
    match args.format.as_str() {
        "json" => {
            ui::export_json(&analysis, "warden-report.json")?;
        }
        _ => {
            ui::show_main_menu(&analysis)?;
        }
    }

    Ok(())
}
