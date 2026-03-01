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
    ]
}
