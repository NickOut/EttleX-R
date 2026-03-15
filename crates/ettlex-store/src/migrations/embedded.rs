//! Embedded SQL migrations
//!
//! Migrations are embedded at compile time using include_str!

/// Migration metadata
pub struct Migration {
    pub id: &'static str,
    pub sql: &'static str,
}

/// Get all embedded migrations in order
pub fn get_migrations() -> Vec<Migration> {
    vec![
        Migration {
            id: "001_initial_schema",
            sql: include_str!("../../migrations/001_initial_schema.sql"),
        },
        Migration {
            id: "002_snapshot_ledger",
            sql: include_str!("../../migrations/002_snapshot_ledger.sql"),
        },
        Migration {
            id: "003_constraints_schema",
            sql: include_str!("../../migrations/003_constraints_schema.sql"),
        },
        Migration {
            id: "004_decisions_schema",
            sql: include_str!("../../migrations/004_decisions_schema.sql"),
        },
        Migration {
            id: "005_profiles_schema",
            sql: include_str!("../../migrations/005_profiles_schema.sql"),
        },
        Migration {
            id: "006_approval_requests_schema",
            sql: include_str!("../../migrations/006_approval_requests_schema.sql"),
        },
        Migration {
            id: "007_approval_cas_schema",
            sql: include_str!("../../migrations/007_approval_cas_schema.sql"),
        },
        Migration {
            id: "008_mcp_command_log",
            sql: include_str!("../../migrations/008_mcp_command_log.sql"),
        },
        Migration {
            id: "009_parent_ep_id",
            sql: include_str!("../../migrations/009_parent_ep_id.sql"),
        },
        Migration {
            id: "010_backfill_parent_ep_id",
            sql: include_str!("../../migrations/010_backfill_parent_ep_id.sql"),
        },
        Migration {
            id: "011_eps_title",
            sql: include_str!("../../migrations/011_eps_title.sql"),
        },
        Migration {
            id: "012_ettle_v2_schema",
            sql: include_str!("../../migrations/012_ettle_v2_schema.sql"),
        },
    ]
}
