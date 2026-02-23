//! Snapshot commit command
//!
//! Usage:
//!   ettlex snapshot commit --leaf <LEAF_EP_ID>
//!   ettlex snapshot commit --root <ROOT_ETTLE_ID>  (legacy)

use clap::{Args, Subcommand};
use ettlex_engine::commands::engine_command::{
    apply_engine_command, EngineCommand, EngineCommandResult,
};
use ettlex_engine::commands::snapshot::{snapshot_commit_by_root_legacy, SnapshotOptions};
use ettlex_store::cas::FsStore;

#[derive(Debug, Args)]
pub struct SnapshotArgs {
    #[command(subcommand)]
    pub command: SnapshotCommand,
}

#[derive(Debug, Subcommand)]
pub enum SnapshotCommand {
    /// Commit a snapshot of the current tree state
    Commit(CommitArgs),
}

#[derive(Debug, Args)]
pub struct CommitArgs {
    /// Leaf EP identifier (canonical, mutually exclusive with --root)
    #[arg(long, conflicts_with = "root")]
    pub leaf: Option<String>,

    /// Root Ettle identifier (legacy, mutually exclusive with --leaf)
    #[arg(long, conflicts_with = "leaf")]
    pub root: Option<String>,

    /// Policy reference (defaults to "policy/default@0")
    #[arg(long, default_value = "policy/default@0")]
    pub policy: String,

    /// Profile reference (defaults to "profile/default@0")
    #[arg(long, default_value = "profile/default@0")]
    pub profile: String,

    /// Dry run (compute manifest but don't persist)
    #[arg(long)]
    pub dry_run: bool,

    /// Database path (defaults to .ettlex/store.db)
    #[arg(long, default_value = ".ettlex/store.db")]
    pub db: String,

    /// CAS directory path (defaults to .ettlex/cas)
    #[arg(long, default_value = ".ettlex/cas")]
    pub cas: String,
}

/// Execute snapshot command
pub fn execute(args: SnapshotArgs) -> Result<(), Box<dyn std::error::Error>> {
    match args.command {
        SnapshotCommand::Commit(commit_args) => execute_commit(commit_args),
    }
}

/// Execute snapshot commit
fn execute_commit(args: CommitArgs) -> Result<(), Box<dyn std::error::Error>> {
    // Validate: Must specify either --leaf or --root
    if args.leaf.is_none() && args.root.is_none() {
        return Err("Must specify either --leaf or --root".into());
    }

    // Open database and CAS
    std::fs::create_dir_all(
        std::path::Path::new(&args.cas)
            .parent()
            .unwrap_or(std::path::Path::new(".ettlex")),
    )?;
    let mut conn = rusqlite::Connection::open(&args.db)?;
    let cas = FsStore::new(&args.cas);

    // Prepare options
    let options = SnapshotOptions {
        expected_head: None,
        dry_run: args.dry_run,
    };

    // Execute: Leaf-scoped (canonical) or root-scoped (legacy)
    let result = if let Some(leaf_ep_id) = args.leaf {
        // Canonical path: leaf-scoped via EngineCommand
        let cmd = EngineCommand::SnapshotCommit {
            leaf_ep_id,
            policy_ref: args.policy,
            profile_ref: args.profile,
            options,
        };

        match apply_engine_command(cmd, &mut conn, &cas)? {
            EngineCommandResult::SnapshotCommit(r) => r,
        }
    } else if let Some(root_ettle_id) = args.root {
        // Legacy path: root-scoped with deterministic resolution
        snapshot_commit_by_root_legacy(
            &root_ettle_id,
            &args.policy,
            &args.profile,
            options,
            &mut conn,
            &cas,
        )?
    } else {
        unreachable!("Either --leaf or --root must be specified");
    };

    // Output result
    if args.dry_run {
        println!("Dry run (no commit):");
        println!(
            "  semantic_manifest_digest: {}",
            result.semantic_manifest_digest
        );
    } else {
        println!("Snapshot committed:");
        println!("  snapshot_id: {}", result.snapshot_id);
        println!("  manifest_digest: {}", result.manifest_digest);
        println!(
            "  semantic_manifest_digest: {}",
            result.semantic_manifest_digest
        );
        if result.was_duplicate {
            println!("  (duplicate - idempotent return)");
        }
    }

    Ok(())
}
