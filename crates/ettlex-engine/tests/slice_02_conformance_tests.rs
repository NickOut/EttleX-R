//! Slice 02 architectural conformance tests (SC-S02-C1 through SC-S02-C14).
//!
//! These are source-inspection tests that assert structural invariants without
//! executing runtime behaviour.  They read source files via `CARGO_MANIFEST_DIR`
//! relative paths and assert/deny the presence of specific patterns.

#![allow(clippy::unwrap_used)]

// ---------------------------------------------------------------------------
// SC-S02-C1: command.rs contains `apply_command`
// ---------------------------------------------------------------------------

#[test]
fn test_slice02_command_rs_contains_apply_command() {
    let src = std::fs::read_to_string(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/src/commands/command.rs"
    ))
    .unwrap();

    assert!(
        src.contains("pub fn apply_command"),
        "command.rs must export `apply_command`"
    );
}

// ---------------------------------------------------------------------------
// SC-S02-C2: command.rs does NOT contain the old `McpCommand` enum name
// ---------------------------------------------------------------------------

#[test]
fn test_slice02_command_rs_no_mcp_command_enum() {
    let src = std::fs::read_to_string(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/src/commands/command.rs"
    ))
    .unwrap();

    assert!(
        !src.contains("enum McpCommand"),
        "command.rs must NOT define `enum McpCommand`; it was renamed to `Command`"
    );
    assert!(
        src.contains("pub enum Command"),
        "command.rs must define `pub enum Command`"
    );
}

// ---------------------------------------------------------------------------
// SC-S02-C3: ettlex-mcp/Cargo.toml does NOT have a direct ettlex-engine dep
// ---------------------------------------------------------------------------

#[test]
fn test_slice02_mcp_no_ettlex_engine_dep() {
    let cargo_toml = std::fs::read_to_string(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../ettlex-mcp/Cargo.toml"
    ))
    .unwrap();

    assert!(
        !cargo_toml.contains("ettlex-engine"),
        "ettlex-mcp/Cargo.toml must NOT have a direct dependency on ettlex-engine; \
         it must depend only on ettlex-memory"
    );
    assert!(
        cargo_toml.contains("ettlex-memory"),
        "ettlex-mcp/Cargo.toml must depend on ettlex-memory"
    );
}

// ---------------------------------------------------------------------------
// SC-S02-C4: ettlex-agent-api/Cargo.toml does NOT have a direct engine dep
// ---------------------------------------------------------------------------

#[test]
fn test_slice02_agent_api_no_engine_dep() {
    let cargo_toml = std::fs::read_to_string(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../ettlex-agent-api/Cargo.toml"
    ))
    .unwrap();

    assert!(
        !cargo_toml.contains("ettlex-engine"),
        "ettlex-agent-api/Cargo.toml must NOT depend directly on ettlex-engine"
    );
    assert!(
        !cargo_toml.contains("ettlex-store"),
        "ettlex-agent-api/Cargo.toml must NOT depend directly on ettlex-store"
    );
    assert!(
        !cargo_toml.contains("ettlex-core"),
        "ettlex-agent-api/Cargo.toml must NOT depend directly on ettlex-core (only via ettlex-memory)"
    );
}

// ---------------------------------------------------------------------------
// SC-S02-C5: RelationCreate dispatch arm in command.rs delegates; no business logic
// ---------------------------------------------------------------------------

#[test]
fn test_slice02_relation_dispatch_no_business_logic() {
    let src = std::fs::read_to_string(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/src/commands/command.rs"
    ))
    .unwrap();

    // The dispatch arm must delegate to handle_relation_create, not inline logic
    assert!(
        src.contains("handle_relation_create"),
        "command.rs must delegate to handle_relation_create"
    );
    // Should not contain direct SqliteRepo calls for relation insertion
    assert!(
        !src.contains("SqliteRepo::insert_relation"),
        "command.rs must NOT call SqliteRepo::insert_relation directly"
    );
}

// ---------------------------------------------------------------------------
// SC-S02-C6: GroupCreate dispatch arm in command.rs delegates; no business logic
// ---------------------------------------------------------------------------

#[test]
fn test_slice02_group_dispatch_no_business_logic() {
    let src = std::fs::read_to_string(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/src/commands/command.rs"
    ))
    .unwrap();

    assert!(
        src.contains("handle_group_create"),
        "command.rs must delegate to handle_group_create"
    );
    assert!(
        !src.contains("SqliteRepo::insert_group"),
        "command.rs must NOT call SqliteRepo::insert_group directly"
    );
}

// ---------------------------------------------------------------------------
// SC-S02-C7: Dedicated relation handler functions exist (compile proof)
// ---------------------------------------------------------------------------

#[test]
fn test_slice02_dedicated_relation_handlers_exist() {
    // Compile-time proof: if these imports compile, the handler functions exist
    let _create_fn: fn(
        &mut rusqlite::Connection,
        String,
        String,
        String,
        Option<serde_json::Value>,
        Option<String>,
    ) -> _ = ettlex_engine::commands::relation::handle_relation_create;
    let _tombstone_fn: fn(&mut rusqlite::Connection, String) -> _ =
        ettlex_engine::commands::relation::handle_relation_tombstone;
}

// ---------------------------------------------------------------------------
// SC-S02-C8: Dedicated group handler functions exist (compile proof)
// ---------------------------------------------------------------------------

#[test]
fn test_slice02_dedicated_group_handlers_exist() {
    // Compile-time proof: if these imports compile, the handler functions exist
    let _create_fn: fn(&mut rusqlite::Connection, String) -> _ =
        ettlex_engine::commands::group::handle_group_create;
    let _tombstone_fn: fn(&mut rusqlite::Connection, String) -> _ =
        ettlex_engine::commands::group::handle_group_tombstone;
}

// ---------------------------------------------------------------------------
// SC-S02-C9: Store's insert_relation does not perform domain validation
// ---------------------------------------------------------------------------

#[test]
fn test_slice02_store_no_domain_validation_for_relations() {
    let src = std::fs::read_to_string(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../ettlex-store/src/repo/sqlite_repo.rs"
    ))
    .unwrap();

    // Store must not check source/target ettle tombstone status during insert
    assert!(
        !src.contains("AlreadyTombstoned"),
        "sqlite_repo.rs must NOT reference AlreadyTombstoned (domain validation belongs in engine)"
    );
    // Store must not check source/target existence during insert_relation
    assert!(
        !src.contains("SelfReferentialLink"),
        "sqlite_repo.rs must NOT reference SelfReferentialLink (domain validation belongs in engine)"
    );
    // Store must not perform cycle detection
    assert!(
        !src.contains("CycleDetected"),
        "sqlite_repo.rs must NOT reference CycleDetected (domain validation belongs in engine)"
    );
}

// ---------------------------------------------------------------------------
// SC-S02-C10: Cycle detection is absent from store; it lives in the engine handler
// ---------------------------------------------------------------------------

#[test]
fn test_slice02_registry_lookup_absent_from_store() {
    // Confirm store has no cycle detection logic
    let store_src = std::fs::read_to_string(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../ettlex-store/src/repo/sqlite_repo.rs"
    ))
    .unwrap();

    assert!(
        !store_src.contains("is_cycle_check_enabled"),
        "sqlite_repo.rs must NOT call is_cycle_check_enabled"
    );
    assert!(
        !store_src.contains("would_create_cycle"),
        "sqlite_repo.rs must NOT call would_create_cycle"
    );

    // Confirm engine relation handler DOES contain cycle check
    let relation_src = std::fs::read_to_string(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/src/commands/relation.rs"
    ))
    .unwrap();

    assert!(
        relation_src.contains("is_cycle_check_enabled"),
        "relation.rs must contain cycle check logic"
    );
}

// ---------------------------------------------------------------------------
// SC-S02-C11: relation.rs does NOT write to command_log (only doc comments allowed)
// ---------------------------------------------------------------------------

#[test]
fn test_slice02_relation_handler_no_command_log_ref() {
    let src = std::fs::read_to_string(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/src/commands/relation.rs"
    ))
    .unwrap();

    // The string may appear in doc comments; it must NOT appear in actual SQL strings
    assert!(
        !src.contains("\"command_log\""),
        "relation.rs must NOT have SQL referencing command_log table"
    );
    assert!(
        !src.contains("\"mcp_command_log\""),
        "relation.rs must NOT have SQL referencing mcp_command_log table"
    );
    assert!(
        !src.contains("INSERT INTO command_log"),
        "relation.rs must NOT INSERT INTO command_log"
    );
}

// ---------------------------------------------------------------------------
// SC-S02-C12: group.rs does NOT write to command_log (only doc comments allowed)
// ---------------------------------------------------------------------------

#[test]
fn test_slice02_group_handler_no_command_log_ref() {
    let src = std::fs::read_to_string(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/src/commands/group.rs"
    ))
    .unwrap();

    assert!(
        !src.contains("\"command_log\""),
        "group.rs must NOT have SQL referencing command_log table"
    );
    assert!(
        !src.contains("\"mcp_command_log\""),
        "group.rs must NOT have SQL referencing mcp_command_log table"
    );
    assert!(
        !src.contains("INSERT INTO command_log"),
        "group.rs must NOT INSERT INTO command_log"
    );
}

// ---------------------------------------------------------------------------
// SC-S02-C13: relation.rs and group.rs do NOT write to provenance_events
// ---------------------------------------------------------------------------

#[test]
fn test_slice02_provenance_absent_from_handler_files() {
    let relation_src = std::fs::read_to_string(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/src/commands/relation.rs"
    ))
    .unwrap();

    let group_src = std::fs::read_to_string(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/src/commands/group.rs"
    ))
    .unwrap();

    // Must not have SQL INSERT statements for provenance_events
    assert!(
        !relation_src.contains("INSERT INTO provenance_events"),
        "relation.rs must NOT insert into provenance_events — owned by apply_command"
    );
    assert!(
        !group_src.contains("INSERT INTO provenance_events"),
        "group.rs must NOT insert into provenance_events — owned by apply_command"
    );
}

// ---------------------------------------------------------------------------
// SC-S02-C14: StateGetVersion in engine_query.rs uses command_log, not schema_version
// ---------------------------------------------------------------------------

#[test]
fn test_slice02_state_get_version_uses_command_log() {
    let src = std::fs::read_to_string(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/src/commands/engine_query.rs"
    ))
    .unwrap();

    assert!(
        src.contains("command_log"),
        "engine_query.rs StateGetVersion must use command_log table"
    );
    // Should not use the old schema_version table
    assert!(
        !src.contains("schema_version"),
        "engine_query.rs must NOT use schema_version for state_version (use command_log)"
    );
}
