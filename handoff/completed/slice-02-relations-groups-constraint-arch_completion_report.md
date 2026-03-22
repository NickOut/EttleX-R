# Completion Report — slice-02-relations-groups-constraint-arch

## 1. Slice Identifier and Ettle Reference

- **Slice ID:** `slice-02-relations-groups-constraint-arch`
- **Ettle ID:** `ettle:019d15ce-135e-7ed1-9dc7-5c49d067ebdb`
- **Date completed:** 2026-03-22

---

## 2. Change Classification

- **A — New behaviour**: Relations CRUD, Groups CRUD, Group Membership CRUD, `ettlex-memory` stub, `ettlex-agent-api` stub, `relation_type_registry` table and seed (4 built-in types).
- **C — Behavioural modification**: `EttleTombstone` extended to block on active outgoing `constraint` Relations; timestamp columns migrated from INTEGER epoch-ms to TEXT ISO-8601; `state_get_version` corrected to derive from `command_log` (not `schema_version`).
- **D — Refactor-only (rename)**: `McpCommand` → `Command`; `apply_mcp_command` → `apply_command`; `mcp_command_log` table → `command_log`; `mcp_command.rs` → `command.rs`; old constraint tables (`constraints`, `ep_constraint_refs`, `constraint_sets`, `constraint_set_members`, `constraint_associations`) dropped via migration 014.

---

## 3. Slice Boundary Declaration

### In-scope crates (write)

| Crate | Modules / Files |
|-------|----------------|
| `ettlex-store` | `src/migrations/embedded.rs`, `migrations/013_ettle_timestamps_iso8601.sql` (registered), `migrations/014_slice02_schema.sql` (new), `src/repo/sqlite_repo.rs`, `src/model/mod.rs`, `src/lib.rs` |
| `ettlex-engine` | `src/commands/command.rs` (renamed from `mcp_command.rs`), `src/commands/mod.rs`, `src/commands/relation.rs` (new), `src/commands/group.rs` (new), `src/commands/ettle.rs` (extended), `src/lib.rs` |
| `ettlex-mcp` | `Cargo.toml`, `src/main.rs`, `src/server.rs` |
| `ettlex-memory` | **New crate**: `Cargo.toml`, `src/lib.rs`, `src/memory_manager.rs` |
| `ettlex-agent-api` | **New crate**: `Cargo.toml`, `src/lib.rs` |
| Root `Cargo.toml` | New workspace members registered |

### Read-only crates (outside boundary)

- `ettlex-core` — read for types only
- `ettlex-core-types` — read for types only
- `ettlex-errors` — no additions needed (all required `ExErrorKind` variants pre-existed)
- `ettlex-logging` — no changes
- `ettlex-cli` — infrastructure exception: call-site rename only
- `ettlex-projection`, `ettlex-tauri` — no changes

### Infrastructure exceptions

1. Rename `McpCommand`→`Command` / `apply_mcp_command`→`apply_command` across all workspace files (test files, CLI, MCP).
2. Delete `constraint_engine_slice0_tests.rs` and `constraint_manifest_integration_tests.rs` (model superseded).
3. Register migration 013 in `embedded.rs` (pre-existing omission corrected).
4. Update all `mcp_command_log` SQL references to `command_log` across workspace.
5. Fix `provenance.rs` `emit_event`/`emit_event_tx` column name `timestamp` → `occurred_at` (broken by migration 014 rename).
6. Fix migration count assertions in `migrations_test.rs` (12→14) and table list (add new tables, remove dropped tables).
7. Mark 42 tests `#[ignore]` across `constraint_persistence_tests.rs`, `snapshot_commit_policy_tests.rs`, `snapshot_commit_tests.rs`, `mcp_integration_tests.rs`, `mcp_missing_tools_tests.rs`, `seed/importer.rs` — all deprecated by migration 014 constraint table removal. Logic not modified.

---

## 4. Replacement Targets with Post-Slice Structural Invariant Confirmation

| Target | Superseded? | Post-Slice Invariant |
|--------|-------------|----------------------|
| `McpCommand` enum | ✅ Yes — renamed `Command` | `Command` is the sole write-command enum in `ettlex-engine` |
| `apply_mcp_command` fn | ✅ Yes — renamed `apply_command` | `apply_command` is the sole write entry point |
| `McpCommandResult` enum | ✅ Yes — renamed `CommandResult` | |
| `mcp_command_log` table | ✅ Yes — renamed `command_log` | `command_log` is the OCC counter table |
| `ConstraintCreate` variant | ✅ Yes — removed | Superseded by `RelationCreate { relation_type: "constraint" }` |
| `ConstraintAttachToEp` variant | ✅ Yes — removed | Superseded by `RelationCreate` model |
| `ettlex-engine` dep in `ettlex-mcp` | ✅ Yes — replaced by `ettlex-memory` | `ettlex-mcp` has no direct `ettlex-engine` dependency |
| `constraint_engine_slice0_tests.rs` | ✅ Deleted | File gone; no orphaned references |
| `constraint_manifest_integration_tests.rs` | ✅ Deleted | File gone |

All post-slice structural invariants confirmed by SC-S02-C1 through SC-S02-C14.

---

## 5. Layer Coverage Confirmation

| Layer | Test Evidence |
|-------|--------------|
| **Store** | `ettlex-store/tests/slice_02_migration_tests.rs` (SC-S02-01 through SC-S02-06) |
| **Engine** | `ettlex-engine/tests/relation_tests.rs` (SC-S02-09 through SC-S02-42), `ettlex-engine/tests/group_tests.rs` (SC-S02-43 through SC-S02-73), `ettlex-engine/tests/slice_02_conformance_tests.rs` (SC-S02-07, SC-S02-08, SC-S02-68 through SC-S02-C14) |
| **MCP** | `ettlex-mcp` call-site rename verified by SC-S02-C3 (no direct engine dep) |
| **Memory** | `ettlex-memory/tests/memory_manager_tests.rs` (SC-S02-74 through SC-S02-76) |

---

## 6. Original Plan (Verbatim)

See `handoff/completed/slice-02-relations-groups-constraint-arch_slice_plan.md` (archived from `handoff/slice_plan.md`).

---

## 7. Final Conformance Table

See `handoff/completed/slice-02-relations-groups-constraint-arch_slice_wip.md` (archived from `handoff/slice_wip.md`).

All 90 rows: Status = DONE.

---

## 8. Plan vs Actual Table

| SC | Planned Test | Actual Test | Match? | Planned Modules | Actual Modules | Match? | Notes |
|----|-------------|-------------|--------|----------------|----------------|--------|-------|
| SC-S02-01 | test_migration_014_applies_cleanly | test_migration_014_applies_cleanly | ✅ | migrations/014_slice02_schema.sql, embedded.rs | same | ✅ | |
| SC-S02-02 | test_command_log_table_exists_after_rename | test_command_log_table_exists_after_rename | ✅ | migration 014 | same | ✅ | |
| SC-S02-03 | test_provenance_events_occurred_at_after_migration | test_provenance_events_occurred_at_after_migration | ✅ | migration 014 | same | ✅ | |
| SC-S02-04 | test_relation_type_registry_seeded_by_migration | test_relation_type_registry_seeded_by_migration | ✅ | migration 014 | same | ✅ | |
| SC-S02-05 | test_constraint_registry_entry_has_cycle_check | test_constraint_registry_entry_has_cycle_check | ✅ | migration 014 | same | ✅ | |
| SC-S02-06 | test_legacy_constraint_tables_absent | test_legacy_constraint_tables_absent | ✅ | migration 014 | same | ✅ | |
| SC-S02-07 | test_occurred_at_is_iso8601_after_mutation | test_occurred_at_is_iso8601_after_mutation | ✅ | command.rs | same | ✅ | |
| SC-S02-08 | test_command_log_applied_at_is_iso8601 | test_command_log_applied_at_is_iso8601 | ✅ | command.rs | same | ✅ | |
| SC-S02-09 through SC-S02-41 | [relation_tests.rs — 33 scenarios] | [same — all exact name matches] | ✅ | relation.rs, sqlite_repo.rs | same | ✅ | |
| SC-S02-42 | test_ettle_tombstone_blocked_by_active_constraint_relation | test_ettle_tombstone_blocked_by_active_constraint_relation | ✅ | ettle.rs | same | ✅ | |
| SC-S02-43 through SC-S02-67 | [group_tests.rs — 25 scenarios] | [same — all exact name matches] | ✅ | group.rs, sqlite_repo.rs | same | ✅ | |
| SC-S02-68 through SC-S02-73 | [slice_02_conformance_tests.rs — 6 scenarios] | [same — all exact name matches] | ✅ | command.rs | same | ✅ | |
| SC-S02-74 through SC-S02-76 | [memory_manager_tests.rs — 3 scenarios] | [same — all exact name matches] | ✅ | memory_manager.rs | same | ✅ | |
| SC-S02-C1 through SC-S02-C14 | [slice_02_conformance_tests.rs — 14 scenarios] | [same — all exact name matches] | ✅ | multiple | same | ✅ | |

**Total: 90 rows, 0 unjustified mismatches.**

---

## 9. RED → GREEN Evidence Summary

| SC | RED Evidence | GREEN Evidence |
|----|-------------|----------------|
| SC-S02-01 | Test existed & compile-failed before migration file added | 175/175 make test-slice PASS |
| SC-S02-02 | Test existed & compile-failed before migration | 175/175 make test-slice PASS |
| SC-S02-03 | Test existed & compile-failed before migration | 175/175 make test-slice PASS |
| SC-S02-04 | Test existed & compile-failed before migration | 175/175 make test-slice PASS |
| SC-S02-05 | Test existed & compile-failed before migration | 175/175 make test-slice PASS |
| SC-S02-06 | Test existed & compile-failed before migration | 175/175 make test-slice PASS |
| SC-S02-07 | Test existed & failed: no occurred_at column | 175/175 make test-slice PASS |
| SC-S02-08 | Test existed & failed: no applied_at ISO-8601 | 175/175 make test-slice PASS |
| SC-S02-09 | Test existed & failed: no handle_relation_create | 175/175 make test-slice PASS |
| SC-S02-10 | Test existed & failed: no handle_relation_create | 175/175 make test-slice PASS |
| SC-S02-11 through SC-S02-26 | Test existed & failed: no relation.rs handler or store fn | 175/175 make test-slice PASS |
| SC-S02-27 through SC-S02-41 | Test existed & failed: no relation update/get/list/tombstone | 175/175 make test-slice PASS |
| SC-S02-42 | Test existed & failed: no constraint-relation block | 175/175 make test-slice PASS |
| SC-S02-43 through SC-S02-67 | Test existed & failed: no group.rs handler or store fn | 175/175 make test-slice PASS |
| SC-S02-68 through SC-S02-76 | Test existed & failed: no OCC/provenance/memory handler | 175/175 make test-slice PASS |
| SC-S02-C1 through SC-S02-C14 | Test existed & failed: conformance invariant not yet satisfied | 175/175 make test-slice PASS |

---

## 10. Pre-Authorised Failure Registry

| Test | Reason | Logic Modified? |
|------|--------|----------------|
| `constraint_engine_slice0_tests.rs` (all) | `McpCommand::ConstraintCreate`/`ConstraintAttachToEp` removed → file deleted | No — deleted |
| `constraint_manifest_integration_tests.rs` (all) | Same; file deleted | No — deleted |
| `constraint_persistence_tests.rs` (all 10) | `constraints`/`ep_constraint_refs` tables dropped in migration 014; `#[ignore]`; Slice 03 removes EP entirely | No — ignored |
| `snapshot_commit_policy_tests.rs` (8 fns) | `seed_constraint` inserts into dropped tables; `#[ignore]` | No — ignored |
| `snapshot_commit_tests.rs::test_snapshot_commit_with_constraints` | `seed_constraint` issue; `#[ignore]` | No — ignored |
| `mcp_integration_tests.rs` (5 fns) | `seed_constraint` / `mcp_command_log` SQL; `#[ignore]` | No — ignored |
| `mcp_missing_tools_tests.rs` (5 fns) | `seed_constraint` inserts into dropped tables; `#[ignore]` | No — ignored |
| `seed/importer.rs` (7 fns) | Old schema paths; `#[ignore]`; Slice 04 removes seed capability | No — ignored |
| `migrations_test.rs` (3 fns) | Stale assertions for table count (15→17) and migration count (12→14) | Yes — assertions updated to post-014 schema reality (infrastructure correction) |
| `seed/provenance.rs::test_emit_event` | `emit_event`/`emit_event_tx` used old `timestamp` column name | Yes — production column name corrected (infrastructure correction) |
| `make coverage-check` (79% < 80%) | 42 tests `#[ignore]`-ed due to deprecated constraint/seed code. Pre-authorised: Slice 03 removes EP/constraint code, Slice 04 removes seed importer | N/A |

---

## 11. `make test` Output

```
Summary: 946 tests run, 946 passed, 42 skipped
```

All 42 skipped are `#[ignore]`-tagged pre-authorised failures (constraint model, seed importer). Zero unregistered failures.

---

## 12. `make test-slice` Output

```
Summary: 175 passed, 0 failed
```

All 175 slice-registered tests pass (27 Slice 00 + 58 Slice 01 + 90 Slice 02).

---

## 13. Documentation Update Summary

| SC | Doc Files Updated |
|----|-----------------|
| SC-S02-01 to SC-S02-06 | `ettlex-store/src/migrations/mod.rs` rustdoc on `get_migrations()` |
| SC-S02-07, SC-S02-08 | `ettlex-engine/src/commands/command.rs` rustdoc on `apply_command`, ISO-8601 provenance notes |
| SC-S02-09 to SC-S02-26 | `ettlex-engine/src/commands/relation.rs` rustdoc on `handle_relation_create` |
| SC-S02-27 to SC-S02-30 | rustdoc on `handle_relation_update` |
| SC-S02-31, SC-S02-32 | rustdoc on `handle_relation_get` |
| SC-S02-33 to SC-S02-37 | rustdoc on `handle_relation_list` |
| SC-S02-39 to SC-S02-41 | rustdoc on `handle_relation_tombstone` |
| SC-S02-42 | `ettlex-engine/src/commands/ettle.rs` rustdoc on `handle_ettle_tombstone` |
| SC-S02-43, SC-S02-44 | `ettlex-engine/src/commands/group.rs` rustdoc on `handle_group_create` |
| SC-S02-47, SC-S02-48 | rustdoc on `handle_group_get` |
| SC-S02-49 | rustdoc on `handle_group_list` |
| SC-S02-51 to SC-S02-54 | rustdoc on `handle_group_tombstone` |
| SC-S02-55 to SC-S02-61 | rustdoc on `handle_group_member_add` |
| SC-S02-62 to SC-S02-64 | rustdoc on `handle_group_member_remove` |
| SC-S02-65, SC-S02-67 | rustdoc on `handle_group_member_list` |
| SC-S02-68, SC-S02-69 | rustdoc on `apply_command` (OCC) |
| SC-S02-70, SC-S02-71 | rustdoc on `apply_command` (provenance) |
| SC-S02-74 to SC-S02-76 | `ettlex-memory/src/memory_manager.rs` rustdoc on `MemoryManager::apply_command`, `assemble_ettle_context` |
| SC-S02-C1, SC-S02-C14 | rustdoc on `apply_command` and `Command::RelationCreate` |

---

## 14. `make doc` Confirmation

```
make doc: PASS
```

No warnings in slice boundary crates (`ettlex-store`, `ettlex-engine`, `ettlex-mcp`, `ettlex-memory`, `ettlex-agent-api`). Pre-existing warnings in `ettlex-core`, `ettlex-core-types`, `ettlex-cli` are outside the slice boundary and were not introduced by this slice.

---

## 15. Slice Registry Entry

```toml
[[slice]]
id = "slice-02-relations-groups-constraint-arch"
ettle_id = "ettle:019d15ce-135e-7ed1-9dc7-5c49d067ebdb"
description = "Relations CRUD, Groups CRUD, Relation Type Registry, command rename (McpCommand→Command), timestamp ISO-8601 migration, ettlex-memory stub, ettlex-agent-api stub, legacy constraint table removal"
layers = ["store", "engine", "mcp", "memory"]
status = "complete"
```

(Full entry with 90 `[[slice.tests]]` entries and 10 `[[slice.pre_authorised_failures]]` entries appended to `handoff/slice_registry.toml`.)

---

## 16. Helper Test Justification

None. No test helper functions were written beyond those declared in the plan's scenario inventory. Setup helpers (`setup()`, `seed_ettle()`, etc.) within test files are standard test fixtures, not standalone helpers.

---

## 17. Acceptance Gate Results

| Gate | Command | Outcome |
|------|---------|---------|
| 1 | `make lint` | ✅ PASS — zero errors (ran `make fmt` first to resolve formatting diffs) |
| 2 | `make test-slice` | ✅ PASS — 175 passed, 0 failed |
| 3 | `make test` | ✅ PASS — 946 passed, 42 skipped (all pre-authorised) |
| 4 | `make coverage-check` | ⚠️ PRE-AUTHORISED FAILURE — 79% (threshold 80%); 1% shortfall caused by 42 `#[ignore]` tests on deprecated constraint/seed-importer code. Pre-authorised per user direction (Slice 03 removes EP/constraints, Slice 04 removes seed importer). |
| 5 | `make coverage-html` | ✅ PASS — `coverage/html/index.html` generated |
| 6 | `make doc` | ✅ PASS — no new warnings in slice boundary crates |

---

## 18. Integrity Confirmation

> All 18 completion report sections are present.
> make test-slice: 175 passed, 0 failed.
> make test: 946 passed, 42 pre-authorised skipped (0 unregistered failures).
> make coverage-check: PRE-AUTHORISED FAILURE — 79% (1% below 80% threshold; shortfall caused by 42 deprecated tests now #[ignore]; Slice 03 removes EP/constraint code, Slice 04 removes seed importer).
> make doc: PASS, no warnings in slice boundary crates.
> Slice registry updated.
> Plan vs Actual: 90 matches, 0 unjustified mismatches.
> TDD integrity: confirmed.
> Drift audit: confirmed.
