# Slice 03 — EP Retirement and Schema Cleanup: Binding Execution Plan

**Ettle:** `ettle:019d170e-828b-79e1-9a7b-b85b214e6ec4`
**Plan written:** 2026-03-22

---

## 1. Slice Identifier

`slice-03-ep-retirement`

---

## 2. Change Classification

- **C** — Behavioural modification: snapshot pipeline behaviour changes from a full commit
  pipeline to a `NotImplemented` stub.
- **D** — Refactor / removal: EP types, EP columns, EP store functions, EP engine query
  variants, EP MCP handlers, EP command variants, ep_ops module, and the Ettle struct's
  EP fields are removed with no intended behavioural change for the remaining retained code.

---

## 3. Slice Boundary Declaration

### Crates in scope

| Crate | Modules |
|---|---|
| `ettlex-store` | `migrations/015_ep_retirement.sql` (new); `repo/sqlite_repo.rs` (EP fn removal); `repo/hydration.rs` (EP reference removal) |
| `ettlex-core` | `model/ep.rs` (delete file); `model/constraint.rs` (`EpConstraintRef` removal); `model/ettle.rs` (EP field removal); `model/mod.rs` (export cleanup); `ops/ep_ops.rs` (delete file); `ops/mod.rs` (module decl removal) |
| `ettlex-engine` | `commands/snapshot.rs` (delete — stub moves to `snapshot/mod.rs`); `snapshot/mod.rs` (populate with stub); `commands/engine_command.rs` (update imports from `crate::snapshot`); `commands/engine_query.rs` (EP variant removal); `commands/command.rs` (`EpCreate`/`EpUpdate` variant removal) |
| `ettlex-mcp` | `tools/ep.rs` (delete file); `tools/ept.rs` (delete file); `tools/mod.rs` (module decl removal) |
| `ettlex-cli` | `commands/ep.rs` (delete file — references `Ep` type being removed); `commands/mod.rs` (ep module decl removal) |
| `CLAUDE.md` | Architecture section update per W7 |
| `handoff/schema_cleanup_notes.md` | Mark resolved items per W8 |

### Read-only crates (boundary)

- `ettlex-errors` — unchanged
- `ettlex-logging` — unchanged
- `ettlex-core-types` — unchanged
- `ettlex-memory` — unchanged
- `ettlex-agent-api` — unchanged
- `ettlex-projection` — unchanged
- `ettlex-tauri` — unchanged

### Infrastructure exceptions

1. **`ettlex-cli/src/commands/ep.rs` and `commands/mod.rs`** — CLI ep command references `Ep`
   type being removed by W11. Deletion is a cascade compilation requirement, not a primary
   behaviour change. The CLI `ep` subcommand is EP-model only; no replacement is warranted
   at this stage.
2. **`ettlex-engine/src/commands/command.rs`** — `EpCreate`/`EpUpdate` Command variants
   reference `Ep`/`SqliteRepo::persist_ep`/`SqliteRepo::get_ep` being removed. Their removal
   cascades from W11 and W5. The MCP ep.rs handler (removed by W4) was the sole caller.
3. **`handoff/schema_cleanup_notes.md`** and **`CLAUDE.md`** — documentation updates required
   by W7 and W8; outside the code boundary but explicitly required by the spec.

---

## 4. Replacement Targets

| File | Function / Module | Disposition | Post-slice structural invariant |
|---|---|---|---|
| `crates/ettlex-engine/src/commands/snapshot.rs` | Entire file | Superseded — deleted | Does not exist after slice |
| `crates/ettlex-engine/src/snapshot/mod.rs` | Empty placeholder | Superseded — replaced with stub | Contains only: deferred comment block, `SnapshotOptions`, `SnapshotCommitResult`, `SnapshotCommitOutcome`, `RoutedForApprovalResult`, stub `snapshot_commit_by_leaf` returning `NotImplemented` |
| `crates/ettlex-core/src/model/ep.rs` | Entire file | Superseded — deleted | Does not exist after slice |
| `crates/ettlex-core/src/ops/ep_ops.rs` | Entire file | Superseded — deleted | Does not exist after slice |
| `crates/ettlex-mcp/src/tools/ep.rs` | Entire file | Superseded — deleted | Does not exist after slice |
| `crates/ettlex-mcp/src/tools/ept.rs` | Entire file | Superseded — deleted | Does not exist after slice |
| `crates/ettlex-cli/src/commands/ep.rs` | Entire file | Cascade delete — Ep type gone | Does not exist after slice |
| `crates/ettlex-core/src/model/constraint.rs` | `EpConstraintRef` struct | Superseded — struct deleted | `Constraint` remains; `EpConstraintRef` is absent |
| `crates/ettlex-core/src/model/ettle.rs` | `parent_id`, `parent_ep_id`, `ep_ids`, `deleted` fields; `is_root`, `has_parent`, `has_eps`, `is_deleted`, `add_ep_id`, `remove_ep_id` methods | Superseded — removed | `Ettle` has only `id`, `title`, `created_at`, `updated_at` |
| `crates/ettlex-engine/src/commands/engine_query.rs` | `EpGet`, `EpListChildren`, `EpListParents`, `EpListConstraints`, `EpListDecisions`, `EptCompute`, `EptComputeDecisionContext` variants | Superseded — removed | None of these variants exist in `EngineQuery` or `EngineQueryResult` |
| `crates/ettlex-engine/src/commands/command.rs` | `Command::EpCreate`, `Command::EpUpdate`, `CommandResult::EpCreate`, `CommandResult::EpUpdate` | Cascade removal | None of these variants exist |
| `crates/ettlex-store/src/repo/sqlite_repo.rs` | `persist_ettle`, `persist_ettle_tx`, `persist_ep`, `persist_ep_tx`, `get_ep`, `list_eps_for_ettle`, `persist_ep_constraint_ref`, `list_ep_constraint_refs`, `list_all_ep_constraint_refs` | Superseded — deleted | None of these functions exist; no remaining function references `parent_id`, `deleted`, `parent_ep_id`, or `ep_constraint_refs` |

---

## 5. Layer Coverage Declaration

| Layer | In scope? | Test coverage |
|---|---|---|
| Store | ✅ | Migration tests in `ettlex-store/tests/slice_03_migration_tests.rs` |
| Engine | ✅ | Functional + conformance tests in `ettlex-engine/tests/slice_03_conformance_tests.rs` |
| Action (Command dispatch) | ✅ | Covered by engine conformance tests (EP command variant removal) |
| MCP | ✅ | Covered by engine conformance tests (source file checks) |
| CLI | ✅ | Covered by engine conformance tests (source file checks) |

All declared layers will be represented in the test suite.

---

## 6. Pre-Authorised Failure Registry (PAFR)

Tests are marked `#[ignore]` where they still compile but behavior changes. Files are deleted
where they reference removed types and cannot compile.

### Test files deleted (compilation failure — reference removed types/variants)

| Test path | Reason |
|---|---|
| `ettlex-engine/tests/ep_update_engine_tests.rs` | References `Command::EpCreate`, `Command::EpUpdate`, `CommandResult::EpCreate` — variants removed |
| `ettlex-engine/tests/mcp_ep_update_tests.rs` | References EP MCP command/result types that no longer exist |
| `ettlex-engine/tests/action_read_tools_integration_tests.rs` | Contains calls to `EngineQuery::EpGet`, `EpListChildren`, `EpListParents`, `EptCompute`, `EptComputeDecisionContext` — variants removed |

### Tests marked `#[ignore]` (compile but behavior changed)

| Test path | Reason | Logic modified? |
|---|---|---|
| `ettlex-engine/tests/snapshot_commit_by_leaf_tests.rs` (all active) | `snapshot_commit_by_leaf` now returns `NotImplemented`; real commit no longer occurs | NO |
| `ettlex-engine/tests/snapshot_commit_tests.rs` (all active) | Same reason | NO |
| `ettlex-engine/tests/snapshot_commit_policy_tests.rs` (remaining active) | Same reason | NO |
| `ettlex-engine/tests/snapshot_commit_determinism_tests.rs` (all) | Same reason | NO |
| `ettlex-engine/tests/snapshot_commit_idempotency_tests.rs` (all) | Same reason | NO |
| `ettlex-engine/tests/snapshot_commit_legacy_resolution_tests.rs` (all) | Same reason | NO |
| `ettlex-engine/tests/snapshot_diff_integration_tests.rs` (all) | Depends on snapshot commits that now return `NotImplemented` | NO |
| `ettlex-store/tests/constraint_persistence_tests.rs` (all, already `#[ignore]`) | Still `#[ignore]` — unchanged | NO |
| `ettlex-store/src/seed/importer.rs` inline tests (already `#[ignore]`) | Still `#[ignore]` — unchanged | NO |

### Coverage pre-authorised note

`make coverage-check` may remain below 80% after this slice. The snapshot pipeline stub
and the removal of EP code together change the coverage numerator and denominator
substantially. Coverage shortfall is pre-authorised as a consequence of EP retirement.
Re-specification of the snapshot commit pipeline (future slice) will restore coverage.

---

## 7. Scenario Inventory

All tests go in two new test files:
- `crates/ettlex-store/tests/slice_03_migration_tests.rs` — migration scenarios (SC-S03-01..11)
- `crates/ettlex-engine/tests/slice_03_conformance_tests.rs` — all other scenarios (SC-S03-12..37)

### Feature: Migration 015

| ID | Title | Layer | Error kind | Predicted RED failure | Production module |
|---|---|---|---|---|---|
| SC-S03-01 | Migration 015 applies cleanly | store | — | Compile: migration file 015 does not exist | `migrations/015_ep_retirement.sql` |
| SC-S03-02 | ettles table has exact expected columns | store | — | Runtime: `parent_id` / `deleted` / `parent_ep_id` columns still present | `migrations/015_ep_retirement.sql` |
| SC-S03-03 | eps table does not exist after migration | store | — | Runtime: `eps` table still exists | `migrations/015_ep_retirement.sql` |
| SC-S03-04 | facet_snapshots table does not exist | store | — | Runtime: table still exists | `migrations/015_ep_retirement.sql` |
| SC-S03-05 | cas_blobs table does not exist | store | — | Runtime: table still exists | `migrations/015_ep_retirement.sql` |
| SC-S03-06 | Ettle rows survive migration intact | store | — | Runtime: content fields lost or altered | `migrations/015_ep_retirement.sql` |
| SC-S03-07 | Migration 015 is idempotent | store | — | Runtime: second run raises SQL error | migration runner |
| SC-S03-08 | Migration applies with empty eps table | store | — | Runtime: migration fails with empty eps | `migrations/015_ep_retirement.sql` |
| SC-S03-09 | Migration applies with populated eps table | store | — | Runtime: migration fails with eps rows | `migrations/015_ep_retirement.sql` |
| SC-S03-10 | parent_id column absent after migration | store | — | Runtime: INSERT with parent_id succeeds (should fail) | `migrations/015_ep_retirement.sql` |
| SC-S03-11 | deleted column absent after migration | store | — | Runtime: INSERT with deleted column succeeds | `migrations/015_ep_retirement.sql` |

### Feature: EP tool handlers removed from MCP

| ID | Title | Layer | Error kind | Predicted RED failure | Production module |
|---|---|---|---|---|---|
| SC-S03-12 | ep module not declared in tools/mod.rs | mcp | — | Source file still contains "pub mod ep" | `tools/mod.rs` (edit) |
| SC-S03-13 | ept module not declared in tools/mod.rs | mcp | — | Source file still contains "pub mod ept" | `tools/mod.rs` (edit) |
| SC-S03-14 | ep.rs file does not exist | mcp | — | File still exists at tools/ep.rs | Delete `tools/ep.rs` |
| SC-S03-15 | ept.rs file does not exist | mcp | — | File still exists at tools/ept.rs | Delete `tools/ept.rs` |

### Feature: Legacy store conformance

| ID | Title | Layer | Error kind | Predicted RED failure | Production module |
|---|---|---|---|---|---|
| SC-S03-16 | sqlite_repo.rs has no parent_id reference | store | — | Source still contains "parent_id" | `sqlite_repo.rs` (edit) |
| SC-S03-17 | sqlite_repo.rs has no "deleted" column reference | store | — | Source still contains `"deleted"` | `sqlite_repo.rs` (edit) |
| SC-S03-18 | sqlite_repo.rs has no parent_ep_id reference | store | — | Source still contains "parent_ep_id" | `sqlite_repo.rs` (edit) |
| SC-S03-19 | CLAUDE.md declares EP construct prohibited | docs | — | CLAUDE.md does not contain "EP construct is prohibited" | `CLAUDE.md` (edit) |
| SC-S03-20 | CLAUDE.md contains "ettlex-memory" as layer | docs | — | CLAUDE.md does not mention ettlex-memory in stack | `CLAUDE.md` (edit) |
| SC-S03-21 | CLAUDE.md references apply_command not apply_mcp_command | docs | — | CLAUDE.md still has "apply_mcp_command" | `CLAUDE.md` (edit) |

### Feature: ep_ops removed from ettlex-core

| ID | Title | Layer | Error kind | Predicted RED failure | Production module |
|---|---|---|---|---|---|
| SC-S03-22 | ep_ops.rs does not exist in ettlex-core ops | core | — | File still exists at ops/ep_ops.rs | Delete `ops/ep_ops.rs` |
| SC-S03-23 | ops/mod.rs does not declare ep_ops module | core | — | Source still contains "mod ep_ops" | `ops/mod.rs` (edit) |

### Feature: Snapshot pipeline stub

| ID | Title | Layer | Error kind | Predicted RED failure | Production module |
|---|---|---|---|---|---|
| SC-S03-24 | snapshot/ directory contains only mod.rs | engine | — | commands/snapshot.rs still exists in snapshot dir (N/A — already 1 file; test still needed) | Delete `commands/snapshot.rs` + populate `snapshot/mod.rs` |
| SC-S03-25 | snapshot/mod.rs does not reference Ep type | engine | — | Source contains "::Ep" (empty file means this passes trivially until stub is written) | `snapshot/mod.rs` (populate with stub) |
| SC-S03-26 | snapshot/mod.rs does not reference EpConstraintRef | engine | — | Source contains "EpConstraintRef" | `snapshot/mod.rs` |
| SC-S03-27 | snapshot/mod.rs does not reference in-memory Store | engine | — | Source contains "ops::Store" | `snapshot/mod.rs` |
| SC-S03-28 | snapshot_commit_by_leaf returns NotImplemented | engine | `NotImplemented` | `apply_engine_command(SnapshotCommit{...})` succeeds instead of returning `NotImplemented` | `snapshot/mod.rs` stub + `engine_command.rs` |
| SC-S03-29 | EngineCommand::SnapshotCommit retained in engine_command.rs | engine | — | Variant absent from source | `engine_command.rs` (retained, no change needed) |

### Feature: Ep type removed from ettlex-core

| ID | Title | Layer | Error kind | Predicted RED failure | Production module |
|---|---|---|---|---|---|
| SC-S03-30 | ep.rs does not exist in ettlex-core model | core | — | File still exists | Delete `model/ep.rs` |
| SC-S03-31 | model/mod.rs does not export Ep | core | — | Source still contains "pub use ep::Ep" | `model/mod.rs` (edit) |
| SC-S03-32 | model/mod.rs does not export EpConstraintRef | core | — | Source still contains "EpConstraintRef" | `model/mod.rs` + `constraint.rs` |
| SC-S03-33 | No workspace file references ettlex_core::model::Ep | workspace | — | grep finds matches | All EP-reference removals above |

### Feature: EP fields removed from ettlex-core Ettle struct

| ID | Title | Layer | Error kind | Predicted RED failure | Production module |
|---|---|---|---|---|---|
| SC-S03-34 | Ettle struct has no parent_id field | core | — | Source still contains "parent_id" | `model/ettle.rs` (edit) |
| SC-S03-35 | Ettle struct has no parent_ep_id field | core | — | Source still contains "parent_ep_id" | `model/ettle.rs` (edit) |
| SC-S03-36 | Ettle struct has no ep_ids field | core | — | Source still contains "ep_ids" | `model/ettle.rs` (edit) |
| SC-S03-37 | Ettle struct has no deleted field | core | — | Source still contains "pub deleted" | `model/ettle.rs` (edit) |

**Total scenarios: 37**

---

## 8. Makefile Update Plan

The following 37 test names are appended to `SLICE_TEST_FILTER` in `makefile`:

```
test_migration_015_applies_cleanly
test_ettles_columns_exact_after_migration
test_eps_table_absent_after_migration
test_facet_snapshots_table_absent_after_migration
test_cas_blobs_table_absent_after_migration
test_ettle_rows_survive_migration_intact
test_migration_015_idempotent
test_migration_applies_with_empty_eps_table
test_migration_applies_with_eps_rows
test_parent_id_column_absent_after_migration
test_deleted_column_absent_after_migration
test_s03_ep_module_not_in_tools_mod_rs
test_s03_ept_module_not_in_tools_mod_rs
test_s03_ep_rs_file_does_not_exist
test_s03_ept_rs_file_does_not_exist
test_s03_sqlite_repo_no_parent_id_reference
test_s03_sqlite_repo_no_deleted_column_reference
test_s03_sqlite_repo_no_parent_ep_id_reference
test_s03_claude_md_ep_construct_prohibited
test_s03_claude_md_ettlex_memory_present
test_s03_claude_md_apply_command_not_mcp
test_s03_ep_ops_file_does_not_exist
test_s03_ops_mod_no_ep_ops_declaration
test_s03_snapshot_only_mod_rs_in_directory
test_s03_snapshot_mod_rs_no_ep_reference
test_s03_snapshot_mod_rs_no_epconstraintref
test_s03_snapshot_mod_rs_no_in_memory_store
test_s03_snapshot_commit_returns_not_implemented
test_s03_engine_command_retains_snapshot_commit
test_s03_ep_rs_not_in_core_model
test_s03_core_model_no_ep_export
test_s03_core_model_no_epconstraintref_export
test_s03_no_workspace_ep_type_references
test_s03_ettle_no_parent_id_field
test_s03_ettle_no_parent_ep_id_field
test_s03_ettle_no_ep_ids_field
test_s03_ettle_no_deleted_field
```

Existing `test` and `test-full` (alias: `test-full`) targets are unchanged.

**IMPORTANT:** The SLICE_TEST_FILTER regex is appended — no existing test names are removed.

---

## 9. Slice Registry Update Plan

The following TOML entry is appended to `handoff/slice_registry.toml` on completion:

```toml
[[slice]]
id = "slice-03-ep-retirement"
ettle_id = "ettle:019d170e-828b-79e1-9a7b-b85b214e6ec4"
description = "EP Retirement and Schema Cleanup — migration 015 (drop eps/facet_snapshots/cas_blobs, rebuild ettles), remove ep.rs/ept.rs MCP handlers, remove EpCreate/EpUpdate Command variants, remove Ep type and EpConstraintRef from ettlex-core, remove EP fields from Ettle struct, stub snapshot pipeline (NotImplemented), remove ep_ops.rs, update CLAUDE.md per W7"
layers = ["store", "engine", "mcp", "cli", "core"]
status = "complete"

[[slice.tests]]
crate = "ettlex-store"
file = "tests/slice_03_migration_tests.rs"
test = "test_migration_015_applies_cleanly"
scenario = "SC-S03-01"

[[slice.tests]]
crate = "ettlex-store"
file = "tests/slice_03_migration_tests.rs"
test = "test_ettles_columns_exact_after_migration"
scenario = "SC-S03-02"

# ... (all 37 tests)
```

Full TOML with all 37 `[[slice.tests]]` entries will be written by `vs-close`.

---

## 10. Acceptance Strategy

```
make lint              # Must pass: no banned patterns, clippy clean, fmt clean
make test-slice        # Must pass: all 37 new tests + all prior registered tests
make test              # Must pass: full suite excluding pre-authorised #[ignore] tests
make coverage-check    # Pre-authorised shortfall — see PAFR coverage note
```

Coverage note: `make coverage-check` may be below 80% after this slice. This is pre-authorised
because the snapshot pipeline stub (returning NotImplemented) is untestable at depth, and the
removed EP code was previously exercised by tests now #[ignore]d. The threshold will be restored
when the snapshot pipeline is re-specified against the Ettle/Relation model (future slice).

---

## 11. Plan Integrity Declaration

> No production code will be written before RED evidence exists.
> No code outside the declared slice boundary will be modified except the Makefile and handoff/slice_registry.toml (and any declared infrastructure exceptions).
> All replacement targets have been identified and their post-slice structural invariants declared.

---

## Implementation Order

The following order minimises intermediate compilation failures:

1. **Migration SQL** — write `015_ep_retirement.sql`; write migration tests (RED); confirm RED;
   run migrations runner (GREEN)
2. **ettlex-core model cleanup** — remove Ep type (ep.rs delete), EpConstraintRef from
   constraint.rs, EP fields from ettle.rs, ep_ops.rs delete, update mod.rs and ops/mod.rs
3. **ettlex-store sqlite_repo cleanup** — remove EP functions (now Ep type is gone, they
   won't compile anyway); remove hydration.rs EP references
4. **ettlex-engine commands cleanup** — remove EpCreate/EpUpdate from command.rs, remove EP
   variants from engine_query.rs
5. **Snapshot stub** — delete commands/snapshot.rs, populate snapshot/mod.rs with stub,
   update engine_command.rs imports; write snapshot conformance tests (RED); verify RED;
   confirm GREEN
6. **MCP handler removal** — delete ep.rs, ept.rs, update tools/mod.rs
7. **CLI ep command removal** — delete commands/ep.rs, update commands/mod.rs
8. **CLAUDE.md + schema_cleanup_notes.md** updates
9. **Write all conformance tests** (RED gate: tests fail before code changes; implement changes
   to reach GREEN)

**Note on RED gate for structural/deletion tests:** Many conformance tests assert absence of
a string in a file. RED is confirmed when the assertion fails because the string IS still
present. Removal of the code makes the test pass (GREEN).
