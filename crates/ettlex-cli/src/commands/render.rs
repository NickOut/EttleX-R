//! Render command
//!
//! Usage: ettlex render <ETTLE_ID> [--output <FILE>]

use clap::{Args, Subcommand};
use std::path::PathBuf;

#[derive(Debug, Args)]
pub struct RenderArgs {
    #[command(subcommand)]
    pub command: RenderCommand,
}

#[derive(Debug, Subcommand)]
pub enum RenderCommand {
    /// Render a single ettle to Markdown
    Ettle(RenderEttleArgs),
    /// Render a leaf bundle (full EPT path) to Markdown
    Bundle(RenderBundleArgs),
}

#[derive(Debug, Args)]
pub struct RenderEttleArgs {
    /// Ettle ID to render
    pub ettle_id: String,

    /// Output file path (default: stdout)
    #[arg(short, long)]
    pub output: Option<PathBuf>,
}

#[derive(Debug, Args)]
pub struct RenderBundleArgs {
    /// Leaf ettle ID to render
    pub leaf_id: String,

    /// Optional EP ordinal for leaf
    #[arg(short, long)]
    pub ep_ordinal: Option<u32>,

    /// Output file path (default: stdout)
    #[arg(short, long)]
    pub output: Option<PathBuf>,
}

/// Execute render command
pub fn execute(args: RenderArgs) -> Result<(), Box<dyn std::error::Error>> {
    match args.command {
        RenderCommand::Ettle(ettle_args) => execute_render_ettle(ettle_args),
        RenderCommand::Bundle(bundle_args) => execute_render_bundle(bundle_args),
    }
}

/// Execute render ettle command
fn execute_render_ettle(args: RenderEttleArgs) -> Result<(), Box<dyn std::error::Error>> {
    // Open database
    let db_path = ".ettlex/store.db";
    let conn = rusqlite::Connection::open(db_path)?;

    // Load tree
    let store = ettlex_store::repo::hydration::load_tree(&conn)?;

    // Render ettle
    let markdown = ettlex_core::render::render_ettle(&store, &args.ettle_id)?;

    // Output
    if let Some(output_path) = args.output {
        std::fs::write(&output_path, markdown)?;
        println!("✓ Rendered to {}", output_path.display());
    } else {
        print!("{}", markdown);
    }

    Ok(())
}

/// Execute render bundle command
fn execute_render_bundle(args: RenderBundleArgs) -> Result<(), Box<dyn std::error::Error>> {
    // Open database
    let db_path = ".ettlex/store.db";
    let conn = rusqlite::Connection::open(db_path)?;

    // Load tree
    let store = ettlex_store::repo::hydration::load_tree(&conn)?;

    // Render bundle
    let markdown = ettlex_core::render::render_leaf_bundle(&store, &args.leaf_id, args.ep_ordinal)?;

    // Output
    if let Some(output_path) = args.output {
        std::fs::write(&output_path, markdown)?;
        println!("✓ Rendered to {}", output_path.display());
    } else {
        print!("{}", markdown);
    }

    Ok(())
}
