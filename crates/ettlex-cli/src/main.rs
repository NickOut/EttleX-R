//! EttleX CLI
//!
//! Command-line interface for EttleX

use clap::{Parser, Subcommand};

use ettlex_cli::commands;

#[derive(Debug, Parser)]
#[command(name = "ettlex")]
#[command(about = "EttleX - Semantic architecture management", long_about = None)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Debug, Subcommand)]
enum Commands {
    /// EP operations (update)
    Ep(commands::ep::EpArgs),
    /// Seed import operations
    Seed(commands::seed::SeedArgs),
    /// Render operations (ettle or bundle to Markdown)
    Render(commands::render::RenderArgs),
    /// Snapshot operations
    Snapshot(commands::snapshot::SnapshotArgs),
}

fn main() {
    let cli = Cli::parse();

    let result = match cli.command {
        Commands::Ep(args) => commands::ep::execute(args),
        Commands::Seed(args) => commands::seed::execute(args),
        Commands::Render(args) => commands::render::execute(args),
        Commands::Snapshot(args) => commands::snapshot::execute(args),
    };

    if let Err(e) = result {
        eprintln!("Error: {}", e);
        std::process::exit(1);
    }
}
