//! Snapshot commit command

use clap::{Args, Subcommand};
use ettlex_core::approval_router::NoopApprovalRouter;
use ettlex_core::policy::NoopCommitPolicyHook;
use ettlex_engine::commands::engine_command::{
    apply_engine_command, EngineCommand, EngineCommandResult,
};
use ettlex_engine::commands::snapshot::{
    resolve_root_to_leaf_ep, SnapshotCommitOutcome, SnapshotOptions,
};
use ettlex_store::cas::FsStore;

#[derive(Debug, Args)]
pub struct SnapshotArgs {
    #[command(subcommand)]
    pub command: SnapshotCommand,
}

#[derive(Debug, Subcommand)]
pub enum SnapshotCommand {
    Commit(CommitArgs),
}

#[derive(Debug, Args)]
pub struct CommitArgs {
    #[arg(long, conflicts_with = "root")]
    pub leaf: Option<String>,

    #[arg(long, conflicts_with = "leaf")]
    pub root: Option<String>,

    #[arg(long, default_value = "policy/default@0")]
    pub policy: String,

    /// Profile reference (optional; defaults to profile/default@0 if not specified)
    #[arg(long)]
    pub profile: Option<String>,

    #[arg(long)]
    pub dry_run: bool,

    #[arg(long, default_value = ".ettlex/store.db")]
    pub db: String,

    #[arg(long, default_value = ".ettlex/cas")]
    pub cas: String,
}

pub fn execute(args: SnapshotArgs) -> Result<(), Box<dyn std::error::Error>> {
    match args.command {
        SnapshotCommand::Commit(commit_args) => execute_commit(commit_args),
    }
}

fn execute_commit(args: CommitArgs) -> Result<(), Box<dyn std::error::Error>> {
    if args.leaf.is_none() && args.root.is_none() {
        return Err("Must specify either --leaf or --root".into());
    }

    std::fs::create_dir_all(
        std::path::Path::new(&args.cas)
            .parent()
            .unwrap_or(std::path::Path::new(".ettlex")),
    )?;
    let mut conn = rusqlite::Connection::open(&args.db)?;
    let cas = FsStore::new(&args.cas);

    let options = SnapshotOptions {
        expected_head: None,
        dry_run: args.dry_run,
        allow_dedup: false,
    };

    let outcome = if let Some(leaf_ep_id) = args.leaf {
        let cmd = EngineCommand::SnapshotCommit {
            leaf_ep_id,
            policy_ref: args.policy,
            profile_ref: args.profile,
            options,
        };

        match apply_engine_command(
            cmd,
            &mut conn,
            &cas,
            &NoopCommitPolicyHook,
            &NoopApprovalRouter,
        )? {
            EngineCommandResult::SnapshotCommit(r) => SnapshotCommitOutcome::Committed(r),
            EngineCommandResult::SnapshotCommitRouted(r) => {
                SnapshotCommitOutcome::RoutedForApproval(r)
            }
        }
    } else if let Some(root_ettle_id) = args.root {
        // Resolve root → leaf, then delegate to apply_engine_command (canonical path)
        let leaf_ep_id = resolve_root_to_leaf_ep(&mut conn, &root_ettle_id)?;
        let cmd = EngineCommand::SnapshotCommit {
            leaf_ep_id,
            policy_ref: args.policy,
            profile_ref: args.profile,
            options,
        };
        match apply_engine_command(
            cmd,
            &mut conn,
            &cas,
            &NoopCommitPolicyHook,
            &NoopApprovalRouter,
        )? {
            EngineCommandResult::SnapshotCommit(r) => SnapshotCommitOutcome::Committed(r),
            EngineCommandResult::SnapshotCommitRouted(r) => {
                SnapshotCommitOutcome::RoutedForApproval(r)
            }
        }
    } else {
        unreachable!()
    };

    match outcome {
        SnapshotCommitOutcome::Committed(r) => {
            if args.dry_run {
                println!("Dry run (no commit):");
                println!("  semantic_manifest_digest: {}", r.semantic_manifest_digest);
            } else {
                println!("Snapshot committed:");
                println!("  snapshot_id: {}", r.snapshot_id);
                println!("  manifest_digest: {}", r.manifest_digest);
                println!("  head_after: {}", r.head_after);
                println!("  semantic_manifest_digest: {}", r.semantic_manifest_digest);
                if r.was_duplicate {
                    println!("  (duplicate - idempotent return)");
                }
            }
        }
        SnapshotCommitOutcome::RoutedForApproval(r) => {
            println!("Routed for approval:");
            println!("  approval_token: {}", r.approval_token);
            println!("  reason_code: {}", r.reason_code);
        }
    }

    Ok(())
}
