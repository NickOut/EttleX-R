//! Slice 03 conformance tests (SC-S03-12 through SC-S03-37).
//!
//! Source-inspection tests that assert EP-retirement structural invariants.
//! These tests read source files and check for presence/absence of patterns.

#![allow(clippy::unwrap_used)]

// ---------------------------------------------------------------------------
// SC-S03-12: ep module is NOT declared in ettlex-mcp/src/tools/mod.rs
// ---------------------------------------------------------------------------

#[test]
fn test_s03_ep_module_not_in_tools_mod_rs() {
    let src = std::fs::read_to_string(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../ettlex-mcp/src/tools/mod.rs"
    ))
    .unwrap();

    assert!(
        !src.contains("pub mod ep;"),
        "tools/mod.rs must NOT declare `pub mod ep;` — ep module retired by Slice 03"
    );
}

// ---------------------------------------------------------------------------
// SC-S03-13: ept module is NOT declared in ettlex-mcp/src/tools/mod.rs
// ---------------------------------------------------------------------------

#[test]
fn test_s03_ept_module_not_in_tools_mod_rs() {
    let src = std::fs::read_to_string(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../ettlex-mcp/src/tools/mod.rs"
    ))
    .unwrap();

    assert!(
        !src.contains("pub mod ept;"),
        "tools/mod.rs must NOT declare `pub mod ept;` — ept module retired by Slice 03"
    );
}

// ---------------------------------------------------------------------------
// SC-S03-14: crates/ettlex-mcp/src/tools/ep.rs does NOT exist
// ---------------------------------------------------------------------------

#[test]
fn test_s03_ep_rs_file_does_not_exist() {
    let path = concat!(env!("CARGO_MANIFEST_DIR"), "/../ettlex-mcp/src/tools/ep.rs");
    assert!(
        !std::path::Path::new(path).exists(),
        "ettlex-mcp/src/tools/ep.rs must not exist after Slice 03 EP retirement"
    );
}

// ---------------------------------------------------------------------------
// SC-S03-15: crates/ettlex-mcp/src/tools/ept.rs does NOT exist
// ---------------------------------------------------------------------------

#[test]
fn test_s03_ept_rs_file_does_not_exist() {
    let path = concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../ettlex-mcp/src/tools/ept.rs"
    );
    assert!(
        !std::path::Path::new(path).exists(),
        "ettlex-mcp/src/tools/ept.rs must not exist after Slice 03 EP retirement"
    );
}

// ---------------------------------------------------------------------------
// SC-S03-16: sqlite_repo.rs has NO parent_id column references
// ---------------------------------------------------------------------------

#[test]
fn test_s03_sqlite_repo_no_parent_id_reference() {
    let src = std::fs::read_to_string(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../ettlex-store/src/repo/sqlite_repo.rs"
    ))
    .unwrap();

    assert!(
        !src.contains("parent_id"),
        "sqlite_repo.rs must NOT reference the dead column `parent_id` after Slice 03 migration"
    );
}

// ---------------------------------------------------------------------------
// SC-S03-17: sqlite_repo.rs has NO `deleted` column references
// ---------------------------------------------------------------------------

#[test]
fn test_s03_sqlite_repo_no_deleted_column_reference() {
    let src = std::fs::read_to_string(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../ettlex-store/src/repo/sqlite_repo.rs"
    ))
    .unwrap();

    // Must not reference the dead `deleted` column in SQL
    assert!(
        !src.contains("ettle.deleted"),
        "sqlite_repo.rs must NOT reference `ettle.deleted` — column removed by migration 015"
    );
    assert!(
        !src.contains("deleted = excluded.deleted"),
        "sqlite_repo.rs must NOT reference `deleted = excluded.deleted` — column removed"
    );
}

// ---------------------------------------------------------------------------
// SC-S03-18: sqlite_repo.rs has NO parent_ep_id column references
// ---------------------------------------------------------------------------

#[test]
fn test_s03_sqlite_repo_no_parent_ep_id_reference() {
    let src = std::fs::read_to_string(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../ettlex-store/src/repo/sqlite_repo.rs"
    ))
    .unwrap();

    assert!(
        !src.contains("parent_ep_id"),
        "sqlite_repo.rs must NOT reference the dead column `parent_ep_id` after Slice 03"
    );
}

// ---------------------------------------------------------------------------
// SC-S03-19: CLAUDE.md contains the phrase "EP construct is prohibited"
// ---------------------------------------------------------------------------

#[test]
fn test_s03_claude_md_ep_construct_prohibited() {
    let src =
        std::fs::read_to_string(concat!(env!("CARGO_MANIFEST_DIR"), "/../../CLAUDE.md")).unwrap();

    assert!(
        src.contains("EP construct is prohibited"),
        "CLAUDE.md must contain the phrase 'EP construct is prohibited' after Slice 03"
    );
}

// ---------------------------------------------------------------------------
// SC-S03-20: CLAUDE.md mentions ettlex-memory in the architecture stack
// ---------------------------------------------------------------------------

#[test]
fn test_s03_claude_md_ettlex_memory_present() {
    let src =
        std::fs::read_to_string(concat!(env!("CARGO_MANIFEST_DIR"), "/../../CLAUDE.md")).unwrap();

    assert!(
        src.contains("ettlex-memory"),
        "CLAUDE.md must reference ettlex-memory in the architecture stack (added by Slice 02)"
    );
}

// ---------------------------------------------------------------------------
// SC-S03-21: CLAUDE.md contains `apply_command` and NOT `apply_mcp_command`
// ---------------------------------------------------------------------------

#[test]
fn test_s03_claude_md_apply_command_not_mcp() {
    let src =
        std::fs::read_to_string(concat!(env!("CARGO_MANIFEST_DIR"), "/../../CLAUDE.md")).unwrap();

    assert!(
        src.contains("apply_command"),
        "CLAUDE.md must reference `apply_command` (the renamed function from Slice 02)"
    );
    assert!(
        !src.contains("apply_mcp_command"),
        "CLAUDE.md must NOT reference `apply_mcp_command` — function was renamed to `apply_command`"
    );
}

// ---------------------------------------------------------------------------
// SC-S03-22: crates/ettlex-core/src/ops/ep_ops.rs does NOT exist
// ---------------------------------------------------------------------------

#[test]
fn test_s03_ep_ops_file_does_not_exist() {
    let path = concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../ettlex-core/src/ops/ep_ops.rs"
    );
    assert!(
        !std::path::Path::new(path).exists(),
        "ettlex-core/src/ops/ep_ops.rs must not exist after Slice 03 EP retirement"
    );
}

// ---------------------------------------------------------------------------
// SC-S03-23: ettlex-core/src/ops/mod.rs does NOT declare pub mod ep_ops
// ---------------------------------------------------------------------------

#[test]
fn test_s03_ops_mod_no_ep_ops_declaration() {
    let src = std::fs::read_to_string(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../ettlex-core/src/ops/mod.rs"
    ))
    .unwrap();

    assert!(
        !src.contains("ep_ops"),
        "ettlex-core/src/ops/mod.rs must NOT declare ep_ops — retired by Slice 03"
    );
}

// ---------------------------------------------------------------------------
// SC-S03-24: commands/snapshot.rs does NOT exist; snapshot logic is in snapshot/mod.rs only
// ---------------------------------------------------------------------------

#[test]
fn test_s03_snapshot_only_mod_rs_in_directory() {
    let commands_snapshot_path = concat!(env!("CARGO_MANIFEST_DIR"), "/src/commands/snapshot.rs");
    assert!(
        !std::path::Path::new(commands_snapshot_path).exists(),
        "crates/ettlex-engine/src/commands/snapshot.rs must NOT exist after Slice 03; \
         snapshot logic must live only in crates/ettlex-engine/src/snapshot/mod.rs"
    );

    // Confirm snapshot/mod.rs exists (the sole snapshot entry point)
    let snapshot_mod_path = concat!(env!("CARGO_MANIFEST_DIR"), "/src/snapshot/mod.rs");
    assert!(
        std::path::Path::new(snapshot_mod_path).exists(),
        "crates/ettlex-engine/src/snapshot/mod.rs must exist as the sole snapshot module"
    );
}

// ---------------------------------------------------------------------------
// SC-S03-25: snapshot/mod.rs contains snapshot_commit_by_leaf and no Ep reference
// ---------------------------------------------------------------------------

#[test]
fn test_s03_snapshot_mod_rs_no_ep_reference() {
    let src = std::fs::read_to_string(concat!(env!("CARGO_MANIFEST_DIR"), "/src/snapshot/mod.rs"))
        .unwrap();

    // Must contain the stub function (not empty)
    assert!(
        src.contains("snapshot_commit_by_leaf"),
        "snapshot/mod.rs must contain `snapshot_commit_by_leaf` stub function"
    );

    // Must not reference the retired Ep type
    assert!(
        !src.contains("model::Ep") && !src.contains("use.*::Ep") && !src.contains("crate::Ep"),
        "snapshot/mod.rs must NOT reference the Ep type"
    );
    assert!(
        !src.contains("::Ep{") && !src.contains("Ep::"),
        "snapshot/mod.rs must NOT construct or call methods on Ep"
    );
}

// ---------------------------------------------------------------------------
// SC-S03-26: snapshot/mod.rs does NOT reference EpConstraintRef
// ---------------------------------------------------------------------------

#[test]
fn test_s03_snapshot_mod_rs_no_epconstraintref() {
    let src = std::fs::read_to_string(concat!(env!("CARGO_MANIFEST_DIR"), "/src/snapshot/mod.rs"))
        .unwrap();

    // Must contain the stub function (not empty)
    assert!(
        src.contains("snapshot_commit_by_leaf"),
        "snapshot/mod.rs must contain `snapshot_commit_by_leaf` stub function"
    );

    assert!(
        !src.contains("EpConstraintRef"),
        "snapshot/mod.rs must NOT reference EpConstraintRef — retired by Slice 03"
    );
}

// ---------------------------------------------------------------------------
// SC-S03-27: snapshot/mod.rs does NOT reference InMemoryStore
// ---------------------------------------------------------------------------

#[test]
fn test_s03_snapshot_mod_rs_no_in_memory_store() {
    let src = std::fs::read_to_string(concat!(env!("CARGO_MANIFEST_DIR"), "/src/snapshot/mod.rs"))
        .unwrap();

    // Must contain the stub function (not empty)
    assert!(
        src.contains("snapshot_commit_by_leaf"),
        "snapshot/mod.rs must contain `snapshot_commit_by_leaf` stub function"
    );

    assert!(
        !src.contains("InMemoryStore"),
        "snapshot/mod.rs must NOT reference InMemoryStore — EP-era type retired by Slice 03"
    );
}

// ---------------------------------------------------------------------------
// SC-S03-28: snapshot_commit_by_leaf in snapshot/mod.rs returns NotImplemented
// ---------------------------------------------------------------------------

#[test]
fn test_s03_snapshot_commit_returns_not_implemented() {
    let src = std::fs::read_to_string(concat!(env!("CARGO_MANIFEST_DIR"), "/src/snapshot/mod.rs"))
        .unwrap();

    assert!(
        src.contains("snapshot_commit_by_leaf"),
        "snapshot/mod.rs must contain `snapshot_commit_by_leaf`"
    );
    assert!(
        src.contains("NotImplemented"),
        "snapshot_commit_by_leaf in snapshot/mod.rs must return NotImplemented error kind"
    );
}

// ---------------------------------------------------------------------------
// SC-S03-29: EngineCommand enum retains SnapshotCommit; imports from snapshot not commands::snapshot
// ---------------------------------------------------------------------------

#[test]
fn test_s03_engine_command_retains_snapshot_commit() {
    let src = std::fs::read_to_string(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/src/commands/engine_command.rs"
    ))
    .unwrap();

    assert!(
        src.contains("SnapshotCommit"),
        "engine_command.rs must retain the SnapshotCommit variant in EngineCommand"
    );

    // After snapshot.rs is deleted and stub moved, engine_command.rs must import from
    // crate::snapshot, NOT from crate::commands::snapshot
    assert!(
        !src.contains("crate::commands::snapshot"),
        "engine_command.rs must NOT import from `crate::commands::snapshot`; \
         use `crate::snapshot` instead after Slice 03 restructuring"
    );
}

// ---------------------------------------------------------------------------
// SC-S03-30: crates/ettlex-core/src/model/ep.rs does NOT exist
// ---------------------------------------------------------------------------

#[test]
fn test_s03_ep_rs_not_in_core_model() {
    let path = concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../ettlex-core/src/model/ep.rs"
    );
    assert!(
        !std::path::Path::new(path).exists(),
        "ettlex-core/src/model/ep.rs must not exist after Slice 03 EP retirement"
    );
}

// ---------------------------------------------------------------------------
// SC-S03-31: ettlex-core/src/model/mod.rs does NOT export Ep
// ---------------------------------------------------------------------------

#[test]
fn test_s03_core_model_no_ep_export() {
    let src = std::fs::read_to_string(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../ettlex-core/src/model/mod.rs"
    ))
    .unwrap();

    assert!(
        !src.contains("pub mod ep;"),
        "model/mod.rs must NOT declare `pub mod ep;` — ep module retired"
    );
    assert!(
        !src.contains("pub use ep::Ep"),
        "model/mod.rs must NOT re-export `Ep` — EP type retired by Slice 03"
    );
}

// ---------------------------------------------------------------------------
// SC-S03-32: ettlex-core/src/model/mod.rs does NOT export EpConstraintRef
// ---------------------------------------------------------------------------

#[test]
fn test_s03_core_model_no_epconstraintref_export() {
    let src = std::fs::read_to_string(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../ettlex-core/src/model/mod.rs"
    ))
    .unwrap();

    assert!(
        !src.contains("EpConstraintRef"),
        "model/mod.rs must NOT re-export `EpConstraintRef` — retired by Slice 03"
    );
}

// ---------------------------------------------------------------------------
// SC-S03-33: No workspace source files (outside ep.rs/ept.rs) reference Ep types
// ---------------------------------------------------------------------------

#[test]
fn test_s03_no_workspace_ep_type_references() {
    // Check command.rs does not import Ep
    let command_src = std::fs::read_to_string(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/src/commands/command.rs"
    ))
    .unwrap();
    assert!(
        !command_src.contains("use ettlex_core::model::Ep"),
        "engine/commands/command.rs must NOT import the Ep type"
    );

    // Check sqlite_repo.rs does not import Ep or EpConstraintRef
    let repo_src = std::fs::read_to_string(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../ettlex-store/src/repo/sqlite_repo.rs"
    ))
    .unwrap();
    assert!(
        !repo_src.contains(", Ep,") && !repo_src.contains(", Ep}") && !repo_src.contains("Ep, "),
        "sqlite_repo.rs must NOT import the Ep type from ettlex_core::model"
    );
    assert!(
        !repo_src.contains("EpConstraintRef"),
        "sqlite_repo.rs must NOT reference EpConstraintRef — retired by Slice 03"
    );

    // Check ettlex-core/src/ops/store.rs does not reference Ep types
    let store_src = std::fs::read_to_string(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../ettlex-core/src/ops/store.rs"
    ))
    .unwrap();
    assert!(
        !store_src.contains("EpConstraintRef"),
        "ettlex-core/src/ops/store.rs must NOT reference EpConstraintRef — retired"
    );
    assert!(
        !store_src.contains(", Ep,") && !store_src.contains(", Ep}") && !store_src.contains("Ep, "),
        "ettlex-core/src/ops/store.rs must NOT import the Ep type"
    );
}

// ---------------------------------------------------------------------------
// SC-S03-34: Ettle struct in ettle.rs has NO parent_id field
// ---------------------------------------------------------------------------

#[test]
fn test_s03_ettle_no_parent_id_field() {
    let src = std::fs::read_to_string(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../ettlex-core/src/model/ettle.rs"
    ))
    .unwrap();

    assert!(
        !src.contains("parent_id"),
        "ettlex-core/src/model/ettle.rs must NOT define `parent_id` field — dead column removed"
    );
}

// ---------------------------------------------------------------------------
// SC-S03-35: Ettle struct in ettle.rs has NO parent_ep_id field
// ---------------------------------------------------------------------------

#[test]
fn test_s03_ettle_no_parent_ep_id_field() {
    let src = std::fs::read_to_string(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../ettlex-core/src/model/ettle.rs"
    ))
    .unwrap();

    assert!(
        !src.contains("parent_ep_id"),
        "ettlex-core/src/model/ettle.rs must NOT define `parent_ep_id` field — dead column removed"
    );
}

// ---------------------------------------------------------------------------
// SC-S03-36: Ettle struct in ettle.rs has NO ep_ids field
// ---------------------------------------------------------------------------

#[test]
fn test_s03_ettle_no_ep_ids_field() {
    let src = std::fs::read_to_string(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../ettlex-core/src/model/ettle.rs"
    ))
    .unwrap();

    assert!(
        !src.contains("ep_ids"),
        "ettlex-core/src/model/ettle.rs must NOT define `ep_ids` field — EP construct retired"
    );
}

// ---------------------------------------------------------------------------
// SC-S03-37: Ettle struct in ettle.rs has NO deleted field
// ---------------------------------------------------------------------------

#[test]
fn test_s03_ettle_no_deleted_field() {
    let src = std::fs::read_to_string(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../ettlex-core/src/model/ettle.rs"
    ))
    .unwrap();

    assert!(
        !src.contains("pub deleted:"),
        "ettlex-core/src/model/ettle.rs must NOT define `deleted` field — dead column removed"
    );
    assert!(
        !src.contains("deleted: bool"),
        "ettlex-core/src/model/ettle.rs must NOT define `deleted: bool` — dead column removed"
    );
}
