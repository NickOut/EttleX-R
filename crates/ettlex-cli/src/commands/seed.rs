//! Seed import command
//!
//! Usage: ettlex seed import <PATH> [--commit]

use clap::{Args, Subcommand};
use std::path::PathBuf;

#[derive(Debug, Args)]
pub struct SeedArgs {
    #[command(subcommand)]
    pub command: SeedCommand,
}

#[derive(Debug, Subcommand)]
pub enum SeedCommand {
    /// Import a seed file into the database
    Import(ImportArgs),
}

#[derive(Debug, Args)]
pub struct ImportArgs {
    /// Path to seed YAML file or directory
    pub path: PathBuf,

    /// Create a snapshot commit after import (not yet implemented)
    #[arg(long)]
    pub commit: bool,
}

/// Execute seed command
pub fn execute(args: SeedArgs) -> Result<(), Box<dyn std::error::Error>> {
    match args.command {
        SeedCommand::Import(import_args) => execute_import(import_args),
    }
}

/// Execute seed import
fn execute_import(args: ImportArgs) -> Result<(), Box<dyn std::error::Error>> {
    // Check for --commit flag (not yet implemented)
    if args.commit {
        return Err("--commit flag is not yet implemented (Phase 2 feature)".into());
    }

    // Open database (TODO: make path configurable)
    let db_path = ".ettlex/store.db";
    std::fs::create_dir_all(".ettlex")?;

    let mut conn = rusqlite::Connection::open(db_path)?;

    // Apply migrations
    ettlex_store::migrations::apply_migrations(&mut conn)?;

    // Import seed(s)
    if args.path.is_dir() {
        // Import directory of seeds (sorted for determinism)
        let mut seed_files: Vec<PathBuf> = std::fs::read_dir(&args.path)?
            .filter_map(|e| e.ok())
            .map(|e| e.path())
            .filter(|p| {
                p.extension()
                    .map(|ext| ext == "yaml" || ext == "yml")
                    .unwrap_or(false)
            })
            .collect();

        seed_files.sort();

        for seed_file in seed_files {
            println!("Importing {}...", seed_file.display());
            let digest = ettlex_store::seed::import_seed(&seed_file, &mut conn)?;
            println!("✓ Imported (digest: {})", digest);
        }
    } else {
        // Import single seed
        println!("Importing {}...", args.path.display());
        let digest = ettlex_store::seed::import_seed(&args.path, &mut conn)?;
        println!("✓ Imported (digest: {})", digest);
    }

    Ok(())
}
