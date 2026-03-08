//! EP commands — update an existing EP's content fields.

use clap::{Args, Subcommand};
use ettlex_core::approval_router::NoopApprovalRouter;
use ettlex_core::policy_provider::NoopPolicyProvider;
use ettlex_engine::commands::mcp_command::{apply_mcp_command, McpCommand};
use ettlex_store::cas::FsStore;

#[derive(Debug, Args)]
pub struct EpArgs {
    #[command(subcommand)]
    pub command: EpCommand,
}

#[derive(Debug, Subcommand)]
pub enum EpCommand {
    /// Update an EP's content fields (why / what / how / title)
    Update(UpdateArgs),
}

#[derive(Debug, Args)]
pub struct UpdateArgs {
    /// ID of the EP to update
    #[arg(long)]
    pub ep_id: String,

    /// New WHY text (preserved if absent)
    #[arg(long)]
    pub why: Option<String>,

    /// New WHAT text (preserved if absent)
    #[arg(long)]
    pub what: Option<String>,

    /// New HOW text (preserved if absent)
    #[arg(long)]
    pub how: Option<String>,

    /// New display title (preserved if absent)
    #[arg(long)]
    pub title: Option<String>,

    /// Path to the SQLite database
    #[arg(long, default_value = ".ettlex/store.db")]
    pub db: String,

    /// Path to the CAS blob store
    #[arg(long, default_value = ".ettlex/cas")]
    pub cas: String,
}

pub fn execute(args: EpArgs) -> Result<(), Box<dyn std::error::Error>> {
    match args.command {
        EpCommand::Update(update_args) => execute_update(update_args),
    }
}

pub fn execute_update(args: UpdateArgs) -> Result<(), Box<dyn std::error::Error>> {
    let mut conn = rusqlite::Connection::open(&args.db)?;
    ettlex_store::migrations::apply_migrations(&mut conn)?;
    let cas = FsStore::new(&args.cas);

    let cmd = McpCommand::EpUpdate {
        ep_id: args.ep_id.clone(),
        why: args.why,
        what: args.what,
        how: args.how,
        title: args.title,
    };

    apply_mcp_command(
        cmd,
        None,
        &mut conn,
        &cas,
        &NoopPolicyProvider,
        &NoopApprovalRouter,
    )
    .map_err(|e| Box::new(e) as Box<dyn std::error::Error>)?;

    println!("EP {} updated successfully.", args.ep_id);
    Ok(())
}
