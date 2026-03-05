use clap::{CommandFactory, Parser, Subcommand};
use std::path::PathBuf;

mod git_parser;
mod metrics;
mod analytics;
mod prediction;
mod ui;
mod cache;
mod models;

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
}

fn main() -> anyhow::Result<()> {
    let args = Args::parse();

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
        Some(Commands::Help) => {
            println!("{}", Args::command().render_help());
            return Ok(());
        }
        None => {}
    }

    // Main analysis flow
    println!("╔════════════════════════════════════╗");
    println!("║   Warden v0.1.0                    ║");
    println!("║   Code Quality Historical Analysis ║");
    println!("╚════════════════════════════════════╝");
    println!();

    // TODO: Implement analysis pipeline
    println!("📊 Analyzing Git repository...");
    println!("⏳ Coming soon in v0.1.0");

    Ok(())
}
