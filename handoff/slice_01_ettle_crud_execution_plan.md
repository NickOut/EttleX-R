# Slice 01 — Ettle CRUD Binding Execution Plan

**Ettle ID:** ettle:019cf0f7-ae6b-7ea3-8b72-ba0b4052b9c3
**Protocol:** code_generator_prompt_vertical_slice_v1.1.md
**Date written:** 2026-03-15
**Status:** COMPLETE — see `handoff/completed/ettle-crud_completion_report.md`

---

## STEP 1 — Binding Execution Plan

### 1. Slice Identifier
`ettle-crud`

### 2. Change Classification
**C + B** — Behavioural modification (EttleCreate shape, EttleGet/List response) + Behavioural extension (EttleUpdate, EttleTombstone as new operations).

### 3. Slice Boundary Declaration

**In scope (modified):**
| Crate | Scope |
|-------|-------|
| `ettlex-store` | `src/repo/sqlite_repo.rs`, `migrations/012_ettle_v2_schema.sql` (new), `src/model/ettle_record.rs` (new), `src/model/mod.rs` (new) |
| `ettlex-engine` | `src/commands/mcp_command.rs`, `src/commands/ettle.rs` (new), `src/commands/mod.rs`, `tests/ettle_crud_tests.rs` (new), `tests/ettle_architectural_conformance_tests.rs` (new) |
| `ettlex-mcp` | `src/tools/ettle.rs`, `src/main.rs` |
| `ettlex-store` | `tests/ettle_migration_tests.rs` (new) |
| `makefile` | Step 3 exception |
| `handoff/slice_registry.toml` | Step 6 exception |

**Infrastructure exception (McpCommand::EttleCreate field update only):**
`crates/ettlex-engine/tests/identity_contract_tests.rs` updated mechanically to add new fields to all `EttleCreate` constructions. No other logic changed.

**Read-only (outside boundary):**
- `ettlex-core` — `Ettle` struct in `ettlex-core/src/model/ettle.rs` NOT modified
- `ettlex-errors` — all required `ExErrorKind` variants already present (Slice 00)
- `ettlex-logging`, `ettlex-cli`, `ettlex-tauri` — untouched

### 4. Replacement Targets

| Target | File | Post-Slice Structural Invariant |
|--------|------|----------------------------------|
| `persist_ettle` | `sqlite_repo.rs` | Removed; replaced by `insert_ettle`. |
| `persist_ettle_tx` | `sqlite_repo.rs` | Removed; no replacement. |
| `get_ettle` (old) | `sqlite_repo.rs` | Replaced by new `get_ettle` returning `Result<Option<EttleRecord>>` using v2 columns. |
| `list_ettles_paginated` | `sqlite_repo.rs` | Replaced by `list_ettles(conn, opts)` with cursor pagination and `include_tombstoned` support. |
| `McpCommand::EttleCreate` dispatch arm | `mcp_command.rs` | Delegates to `handle_ettle_create`. |
| `handle_ettle_get` | `ettlex-mcp/src/tools/ettle.rs` | Returns full v2 field set. |
| `handle_ettle_list` | `ettlex-mcp/src/tools/ettle.rs` | Accepts `include_tombstoned`; returns `{items, cursor?}`. |

### 5. Layer Coverage Declaration

| Layer | Covered | Test Evidence |
|-------|---------|---------------|
| Store | Yes | SC-50, SC-51 (migration tests) |
| Engine/Action | Yes | SC-01 through SC-49 in `ettle_crud_tests.rs` |
| MCP | Yes | SC-52 through SC-58 (architectural conformance) |

### 6. Pre-Authorised Failure Registry (PAFR)

**Category B — Runtime failures: snapshot commit hydration**
Migration 012 does NOT drop the old columns (`deleted`, `parent_id`, `parent_ep_id`, `metadata`) so the snapshot commit hydration code continues to compile and run. However, all snapshot commit tests were expected to be PAF due to the schema change. In practice, because DROP COLUMN was omitted from the migration, these tests continue to pass.

**Category C — Runtime failures: old EttleGet/EttleList response shape**
Tests in `action_read_tools_integration_tests.rs` that assert on old ettle query fields remain as PAF. The `EngineQuery::EttleGet` / `EttleList` dispatch arms in `apply_engine_query` are stale (DI-02).

### 7. Deferred Items

| # | Issue | Owner / Trigger |
|---|-------|-----------------|
| DI-01 | Store hydration layer for ettles | EPT / Snapshot pipeline slice |
| DI-02 | `EngineQuery::EttleGet` / `EttleList` dispatch in `apply_engine_query` stale | EngineQuery cleanup slice |
| DI-03 | `ettlex-core::Ettle` struct retirement | Model consolidation slice |
| DI-04 | `DROP COLUMN` for `deleted`, `parent_id`, `parent_ep_id`, `metadata` | After DI-01/DI-03 complete |

### 8. Scenario Inventory

**Test file 1: `crates/ettlex-engine/tests/ettle_crud_tests.rs` (49 tests)**

| ID | Test Name | Layer |
|----|-----------|-------|
| SC-01 | `test_create_minimal_ettle_succeeds` | Engine+Store |
| SC-02 | `test_create_returns_ettle_id` | Engine+Store |
| SC-03 | `test_create_with_all_fields_succeeds` | Engine+Store |
| SC-04 | `test_create_with_reasoning_link_succeeds` | Engine+Store |
| SC-05 | `test_create_empty_title_fails` | Engine |
| SC-06 | `test_create_rejects_caller_supplied_id` | Engine |
| SC-07 | `test_create_link_without_type_fails` | Engine |
| SC-08 | `test_create_type_without_link_fails` | Engine |
| SC-09 | `test_create_link_to_nonexistent_ettle_fails` | Engine |
| SC-10 | `test_create_link_to_tombstoned_ettle_fails` | Engine |
| SC-11 | `test_create_whitespace_only_title_fails` | Engine |
| SC-12 | `test_get_returns_all_fields` | Engine+Store |
| SC-13 | `test_get_nonexistent_returns_not_found` | Engine+Store |
| SC-14 | `test_list_empty_returns_empty_page` | Engine+Store |
| SC-15 | `test_list_single_ettle` | Engine+Store |
| SC-16 | `test_list_pagination_cursor` | Engine+Store |
| SC-17 | `test_list_limit_zero_fails` | Engine |
| SC-18 | `test_list_limit_over_500_fails` | Engine |
| SC-19 | `test_list_invalid_cursor_fails` | Engine |
| SC-20 | `test_list_excludes_tombstoned_by_default` | Engine+Store |
| SC-21 | `test_list_include_tombstoned_flag` | Engine+Store |
| SC-22 | `test_update_title_succeeds` | Engine+Store |
| SC-23 | `test_update_why_succeeds` | Engine+Store |
| SC-24 | `test_update_what_succeeds` | Engine+Store |
| SC-25 | `test_update_how_succeeds` | Engine+Store |
| SC-26 | `test_update_sets_reasoning_link` | Engine+Store |
| SC-27 | `test_update_changes_reasoning_link` | Engine+Store |
| SC-28 | `test_update_clears_reasoning_link` | Engine+Store |
| SC-29 | `test_update_preserves_unspecified_fields` | Engine+Store |
| SC-30 | `test_update_rejects_self_referential_link` | Engine |
| SC-31 | `test_update_nonexistent_ettle_fails` | Engine |
| SC-32 | `test_update_tombstoned_ettle_fails` | Engine |
| SC-33 | `test_update_empty_update_fails` | Engine |
| SC-34 | `test_update_link_to_nonexistent_fails` | Engine |
| SC-35 | `test_update_link_without_type_fails` | Engine |
| SC-36 | `test_tombstone_active_ettle_succeeds` | Engine+Store |
| SC-37 | `test_tombstone_nonexistent_ettle_fails` | Engine |
| SC-38 | `test_tombstone_already_tombstoned_fails` | Engine |
| SC-39 | `test_tombstone_with_active_dependants_fails` | Engine |
| SC-40 | `test_tombstone_allows_tombstoned_dependant` | Engine+Store |
| SC-41 | `test_hard_delete_not_exposed` | Engine |
| SC-42 | `test_occ_correct_version_succeeds` | Engine |
| SC-43 | `test_occ_wrong_version_fails` | Engine |
| SC-44 | `test_each_mutation_appends_one_provenance_event` | Engine+Store |
| SC-45 | `test_failed_command_no_provenance_event` | Engine+Store |
| SC-46 | `test_ettle_get_byte_identical` | Engine+Store |
| SC-47 | `test_ettle_list_byte_identical` | Engine+Store |
| SC-48 | `test_create_large_fields_succeeds` | Engine+Store |
| SC-49 | `test_list_max_limit_succeeds` | Engine+Store |

**Test file 2: `crates/ettlex-store/tests/ettle_migration_tests.rs` (2 tests)**

| ID | Test Name | Layer |
|----|-----------|-------|
| SC-50 | `test_migration_012_applies_cleanly` | Store |
| SC-51 | `test_existing_ettle_rows_survive_with_defaults` | Store |

**Test file 3: `crates/ettlex-engine/tests/ettle_architectural_conformance_tests.rs` (7 tests)**

| ID | Test Name | INV |
|----|-----------|-----|
| SC-52 | `test_dispatch_no_ettle_business_logic` | INV-1 |
| SC-53 | `test_dedicated_handler_functions_exist` | INV-2 |
| SC-54 | `test_store_functions_no_domain_validation` | INV-3 |
| SC-55 | `test_state_version_owned_by_apply_mcp_command` | INV-6 |
| SC-56 | `test_provenance_owned_by_engine_action` | INV-7 |
| SC-57 | `test_no_ettle_delete_variant` | INV-8 |
| SC-58 | `test_ettle_handler_no_raw_sql` | INV-4 |

**Total: 58 tests across 3 files.**

### 9. Key Design Decisions

1. **`EttleRecord` in `ettlex-store`**: Avoids touching `ettlex-core::Ettle` struct. The old `Ettle` struct survives as legacy for in-memory store tests.

2. **DROP COLUMN omitted from migration 012**: The plan called for dropping `deleted`, `parent_id`, `parent_ep_id`, `metadata`. These are retained in the schema to avoid breaking the snapshot commit hydration path (DI-01). Deferred to DI-04.

3. **Double-Option for nullable EttleUpdate fields**: `reasoning_link_id: Option<Option<String>>` with custom `deserialize_double_option` fn. Absent → `None` (preserve), null → `Some(None)` (clear), string → `Some(Some(v))` (set).

4. **Cursor encoding**: `base64_url_safe_no_pad("{created_at},{id}")` for opaque pagination token.

5. **Provenance events**: Appended in `apply_mcp_command` after successful dispatch, keyed by result variant (`ettle_created`, `ettle_updated`, `ettle_tombstoned`). Handler functions in `ettle.rs` do NOT emit provenance directly.

6. **`handle_ettle_get`/`handle_ettle_list` are `pub fn`**: Called directly from MCP tool handlers, bypassing the stale `apply_engine_query` path.
