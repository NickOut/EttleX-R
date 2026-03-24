# Slice WIP — slice-03-ep-retirement

**Ettle ID:** `ettle:019d170e-828b-79e1-9a7b-b85b214e6ec4`
**Status:** IN PROGRESS

## Conformance Table

| SC | Layer(s) | Planned Test | RED Evidence | GREEN Evidence | Code Files | Doc Files | Doc Evidence | Status |
|----|----------|-------------|--------------|----------------|------------|-----------|--------------|--------|
| SC-S03-01 | store | `test_migration_015_applies_cleanly` | 3 tests failed (eps/cas_blobs/deleted): migration 015 not registered | 212 passed, 0 failed | `crates/ettlex-store/migrations/015_ep_retirement.sql`, `crates/ettlex-store/src/migrations/embedded.rs`, `crates/ettlex-store/src/migrations/runner.rs` | `crates/ettlex-store/README.md`, `crates/ettlex-store/src/migrations/embedded.rs` rustdoc | make doc clean | DONE |
| SC-S03-02 | store | `test_ettles_columns_exact_after_migration` | same RED run | same GREEN run | same | same | same | DONE |
| SC-S03-03 | store | `test_eps_table_absent_after_migration` | assertion failed: eps table must not exist (count was 1) | same GREEN run | same | same | same | DONE |
| SC-S03-04 | store | `test_facet_snapshots_table_absent_after_migration` | same RED run | same GREEN run | same | same | same | DONE |
| SC-S03-05 | store | `test_cas_blobs_table_absent_after_migration` | assertion failed: cas_blobs table must not exist | same GREEN run | same | same | same | DONE |
| SC-S03-06 | store | `test_ettle_rows_survive_migration_intact` | same RED run | same GREEN run | same | same | same | DONE |
| SC-S03-07 | store | `test_migration_015_idempotent` | same RED run | same GREEN run | same | same | same | DONE |
| SC-S03-08 | store | `test_migration_applies_with_empty_eps_table` | same RED run | same GREEN run | same | same | same | DONE |
| SC-S03-09 | store | `test_migration_applies_with_eps_rows` | same RED run | same GREEN run | same | same | same | DONE |
| SC-S03-10 | store | `test_parent_id_column_absent_after_migration` | same RED run | same GREEN run | same | same | same | DONE |
| SC-S03-11 | store | `test_deleted_column_absent_after_migration` | assertion failed: INSERT with deleted should fail | same GREEN run | same | same | same | DONE |
| SC-S03-12 | mcp | `test_s03_ep_module_not_in_tools_mod_rs` | compile error: ep module referenced before removal | 212 passed, 0 failed | `crates/ettlex-mcp/src/tools/mod.rs` | `crates/ettlex-mcp/src/main.rs` description updated | EpCreate/EpUpdate removed from apply description | DONE |
| SC-S03-13 | mcp | `test_s03_ept_module_not_in_tools_mod_rs` | compile error: ept module referenced before removal | same GREEN run | `crates/ettlex-mcp/src/tools/mod.rs` | same | same | DONE |
| SC-S03-14 | mcp | `test_s03_ep_rs_file_does_not_exist` | file existed: crates/ettlex-mcp/src/tools/ep.rs | same GREEN run | deleted `crates/ettlex-mcp/src/tools/ep.rs` | — | — | DONE |
| SC-S03-15 | mcp | `test_s03_ept_rs_file_does_not_exist` | file existed: crates/ettlex-mcp/src/tools/ept.rs | same GREEN run | deleted `crates/ettlex-mcp/src/tools/ept.rs` | — | — | DONE |
| SC-S03-16 | store | `test_s03_sqlite_repo_no_parent_id_reference` | test failed: parent_id references found in sqlite_repo.rs | same GREEN run | `crates/ettlex-store/src/repo/sqlite_repo.rs` | `handoff/schema_cleanup_notes.md` | parent_id resolved | DONE |
| SC-S03-17 | store | `test_s03_sqlite_repo_no_deleted_column_reference` | test failed: deleted column references found | same GREEN run | `crates/ettlex-store/src/repo/sqlite_repo.rs` | `handoff/schema_cleanup_notes.md` | deleted column resolved | DONE |
| SC-S03-18 | store | `test_s03_sqlite_repo_no_parent_ep_id_reference` | test failed: parent_ep_id references found | same GREEN run | `crates/ettlex-store/src/repo/sqlite_repo.rs` | `handoff/schema_cleanup_notes.md` | parent_ep_id resolved | DONE |
| SC-S03-19 | docs | `test_s03_claude_md_ep_construct_prohibited` | test failed: CLAUDE.md lacked phrase "EP construct is prohibited" | 212 passed, 0 failed | `CLAUDE.md` | `CLAUDE.md` | Contains "EP construct is prohibited" | DONE |
| SC-S03-20 | docs | `test_s03_claude_md_ettlex_memory_present` | test failed: CLAUDE.md lacked ettlex-memory in architecture stack | same GREEN run | `CLAUDE.md` | `CLAUDE.md` | Contains `ettlex-memory` in architecture stack | DONE |
| SC-S03-21 | docs | `test_s03_claude_md_apply_command_not_mcp` | test failed: CLAUDE.md lacked `apply_command` reference | same GREEN run | `CLAUDE.md` | `CLAUDE.md` | Contains `apply_command`, no `apply_mcp_command` | DONE |
| SC-S03-22 | core | `test_s03_ep_ops_file_does_not_exist` | file existed: crates/ettlex-core/src/ops/ep_ops.rs | same GREEN run | deleted `crates/ettlex-core/src/ops/ep_ops.rs` | — | — | DONE |
| SC-S03-23 | core | `test_s03_ops_mod_no_ep_ops_declaration` | test failed: mod.rs declared ep_ops | same GREEN run | `crates/ettlex-core/src/ops/mod.rs` | — | — | DONE |
| SC-S03-24 | engine | `test_s03_snapshot_only_mod_rs_in_directory` | compile error: old snapshot.rs in commands/, new mod.rs absent | same GREEN run | deleted `crates/ettlex-engine/src/commands/snapshot.rs`; created `crates/ettlex-engine/src/snapshot/mod.rs` | `crates/ettlex-engine/src/snapshot/mod.rs` rustdoc (`//!` module doc + `///` item docs) | make doc clean | DONE |
| SC-S03-25 | engine | `test_s03_snapshot_mod_rs_no_ep_reference` | test failed: old snapshot.rs contained ep references | same GREEN run | `crates/ettlex-engine/src/snapshot/mod.rs` | same | same | DONE |
| SC-S03-26 | engine | `test_s03_snapshot_mod_rs_no_epconstraintref` | test failed: old snapshot.rs contained EpConstraintRef | same GREEN run | `crates/ettlex-engine/src/snapshot/mod.rs` | same | same | DONE |
| SC-S03-27 | engine | `test_s03_snapshot_mod_rs_no_in_memory_store` | test failed: old snapshot.rs used InMemoryStore | same GREEN run | `crates/ettlex-engine/src/snapshot/mod.rs` | same | same | DONE |
| SC-S03-28 | engine | `test_s03_snapshot_commit_returns_not_implemented` | test failed: snapshot_commit_by_leaf did not return NotImplemented | same GREEN run | `crates/ettlex-engine/src/snapshot/mod.rs`, `crates/ettlex-engine/src/commands/engine_command.rs` | same | same | DONE |
| SC-S03-29 | engine | `test_s03_engine_command_retains_snapshot_commit` | test failed: EngineCommand::SnapshotCommit variant absent | same GREEN run | `crates/ettlex-engine/src/commands/engine_command.rs` | same | same | DONE |
| SC-S03-30 | core | `test_s03_ep_rs_not_in_core_model` | file existed: crates/ettlex-core/src/model/ep.rs | same GREEN run | deleted `crates/ettlex-core/src/model/ep.rs` | — | — | DONE |
| SC-S03-31 | core | `test_s03_core_model_no_ep_export` | test failed: model/mod.rs exported Ep | same GREEN run | `crates/ettlex-core/src/model/mod.rs` | — | — | DONE |
| SC-S03-32 | core | `test_s03_core_model_no_epconstraintref_export` | test failed: model/mod.rs exported EpConstraintRef | same GREEN run | `crates/ettlex-core/src/model/mod.rs`, `crates/ettlex-core/src/model/constraint.rs` | — | — | DONE |
| SC-S03-33 | workspace | `test_s03_no_workspace_ep_type_references` | compile error: Ep/EpConstraintRef referenced in many files | same GREEN run | all EP-reference removals across workspace | — | — | DONE |
| SC-S03-34 | core | `test_s03_ettle_no_parent_id_field` | test failed: Ettle struct had parent_id field | same GREEN run | `crates/ettlex-core/src/model/ettle.rs` | — | — | DONE |
| SC-S03-35 | core | `test_s03_ettle_no_parent_ep_id_field` | test failed: Ettle struct had parent_ep_id field | same GREEN run | `crates/ettlex-core/src/model/ettle.rs` | — | — | DONE |
| SC-S03-36 | core | `test_s03_ettle_no_ep_ids_field` | test failed: Ettle struct had ep_ids field | same GREEN run | `crates/ettlex-core/src/model/ettle.rs` | — | — | DONE |
| SC-S03-37 | core | `test_s03_ettle_no_deleted_field` | test failed: Ettle struct had deleted field | same GREEN run | `crates/ettlex-core/src/model/ettle.rs` | — | — | DONE |
