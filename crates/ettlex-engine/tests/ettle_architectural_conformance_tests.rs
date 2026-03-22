//! Architectural conformance tests — Ettle CRUD Slice 01.
//!
//! SC-52  test_dispatch_no_ettle_business_logic          (INV-1)
//! SC-53  test_dedicated_handler_functions_exist          (INV-2)
//! SC-54  test_store_functions_no_domain_validation       (INV-3)
//! SC-55  test_state_version_owned_by_apply_mcp_command   (INV-6) — checks command.rs / command_log
//! SC-56  test_provenance_owned_by_engine_action          (INV-7)
//! SC-57  test_no_ettle_delete_variant                    (INV-8)
//! SC-58  test_ettle_handler_no_raw_sql                   (INV-9)

#![allow(clippy::unwrap_used)]

// ---------------------------------------------------------------------------
// SC-52: INV-1 — dispatch arm delegates; no business logic in command.rs EttleCreate arm
// ---------------------------------------------------------------------------

#[test]
fn test_dispatch_no_ettle_business_logic() {
    let src = std::fs::read_to_string(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/src/commands/command.rs"
    ))
    .unwrap();

    // The EttleCreate arm must delegate to handle_ettle_create — not directly call SqliteRepo
    assert!(
        !src.contains("SqliteRepo::persist_ettle"),
        "command.rs EttleCreate arm must NOT call SqliteRepo::persist_ettle directly"
    );
    assert!(
        !src.contains("Ettle::new("),
        "command.rs must NOT call Ettle::new directly in EttleCreate arm"
    );
}

// ---------------------------------------------------------------------------
// SC-53: INV-2 — dedicated handler functions exist (compile proof)
// ---------------------------------------------------------------------------

#[test]
fn test_dedicated_handler_functions_exist() {
    // This test is a compile-time proof: if the imports below compile,
    // the handler functions exist with the expected signatures.
    let _: fn(&rusqlite::Connection, &str) -> _ = ettlex_engine::commands::ettle::handle_ettle_get;
    let _: fn(&rusqlite::Connection, ettlex_store::model::EttleListOpts) -> _ =
        ettlex_engine::commands::ettle::handle_ettle_list;
}

// ---------------------------------------------------------------------------
// SC-54: INV-3 — store functions contain no domain validation
// ---------------------------------------------------------------------------

#[test]
fn test_store_functions_no_domain_validation() {
    let src = std::fs::read_to_string(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../ettlex-store/src/repo/sqlite_repo.rs"
    ))
    .unwrap();

    // Find the insert_ettle function body and check it has no domain validation
    // (The simplest approach: assert the specific patterns are absent from the whole file
    //  as they should only live in the handler layer)
    assert!(
        !src.contains("trim().is_empty()"),
        "sqlite_repo.rs insert_ettle must not validate title emptiness"
    );
    assert!(
        !src.contains("InvalidTitle"),
        "sqlite_repo.rs must not reference InvalidTitle"
    );
}

// ---------------------------------------------------------------------------
// SC-55: INV-6 — state_version increment owned by apply_command (command.rs), not ettle.rs
// ---------------------------------------------------------------------------

#[test]
fn test_state_version_owned_by_apply_mcp_command() {
    let cmd_src = std::fs::read_to_string(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/src/commands/command.rs"
    ))
    .unwrap();

    let ettle_src = std::fs::read_to_string(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/src/commands/ettle.rs"
    ))
    .unwrap();

    assert!(
        cmd_src.contains("command_log"),
        "command.rs must manage command_log (state_version)"
    );
    assert!(
        !ettle_src.contains("command_log"),
        "ettle.rs (handler) must NOT touch command_log"
    );
}

// ---------------------------------------------------------------------------
// SC-56: INV-7 — provenance events owned by engine action layer, not handler
// ---------------------------------------------------------------------------

#[test]
fn test_provenance_owned_by_engine_action() {
    let ettle_src = std::fs::read_to_string(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/src/commands/ettle.rs"
    ))
    .unwrap();

    assert!(
        !ettle_src.contains("provenance_events"),
        "ettle.rs (handler) must NOT insert into provenance_events directly"
    );
}

// ---------------------------------------------------------------------------
// SC-57: INV-8 — no EttleDelete variant
// ---------------------------------------------------------------------------

#[test]
fn test_no_ettle_delete_variant() {
    let src = std::fs::read_to_string(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/src/commands/command.rs"
    ))
    .unwrap();

    assert!(
        !src.contains("EttleDelete"),
        "EttleDelete must not exist — only soft tombstone is supported"
    );
}

// ---------------------------------------------------------------------------
// SC-58: INV-9 — ettle handler has no raw SQL
// ---------------------------------------------------------------------------

#[test]
fn test_ettle_handler_no_raw_sql() {
    let ettle_src = std::fs::read_to_string(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/src/commands/ettle.rs"
    ))
    .unwrap();

    assert!(
        !ettle_src.contains("conn.execute(\""),
        "ettle.rs handler must not contain raw SQL — delegate to SqliteRepo"
    );
    assert!(
        !ettle_src.contains("conn.query_row(\""),
        "ettle.rs handler must not contain raw SQL — delegate to SqliteRepo"
    );
    assert!(
        !ettle_src.contains("conn.prepare(\""),
        "ettle.rs handler must not contain raw SQL — delegate to SqliteRepo"
    );
}
