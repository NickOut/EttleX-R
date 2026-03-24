# Slice 02b — MCP Read Surface Patch (Relations and Groups)

## STEP 0 COMPLETE — Files read

✅ Ettle spec loaded: `ettle:019d1d8d-d483-7941-a694-cd6924df612b`
✅ `handoff/slice_registry.toml`
✅ `Makefile`
✅ `CLAUDE.md`
✅ `handoff/EttleX_Logging_Facility_Rust_FULL_Ettle_v3.md`
✅ `handoff/EttleX_Rust_Common_Error_Handling_Facility_FULL_Ettle_v3.md`

---

## 1. Slice Identifier

`slice-02b-mcp-read-surface-patch`

---

## 2. Change Classification

**A — New behaviour (additive only).**

Slice 02 delivered the engine and store implementations for Relation/Group CRUD and exposed
write commands (`RelationCreate`, `RelationUpdate`, `RelationTombstone`, `GroupCreate`,
`GroupTombstone`, `GroupMemberAdd`, `GroupMemberRemove`) via `ettlex_apply`. It did not wire
dedicated MCP read tools. As a result, reads must currently travel through the write command
path — architecturally wrong.

This slice adds 5 new MCP read tools wired directly to existing engine handler functions,
bypassing the `apply_command` / `ettlex_apply` write path entirely.

---

## 3. Slice Boundary Declaration

### Crates in scope (write)

| Crate | File(s) | Change |
|-------|---------|--------|
| `ettlex-store` | `src/repo/sqlite_repo.rs` | Add `list_group_members_by_filter(conn, group_id, ettle_id, include_tombstoned)` — required for `group_member_list(ettle_id)` filter (see spec gap note below) |
| `ettlex-mcp` | `src/tools/relation.rs` (new) | Add 2 MCP tool handlers: `handle_relation_get`, `handle_relation_list` |
| `ettlex-mcp` | `src/tools/group.rs` (new) | Add 3 MCP tool handlers: `handle_group_get`, `handle_group_list`, `handle_group_member_list` |
| `ettlex-mcp` | `src/tools/mod.rs` | Add `pub mod relation; pub mod group;` |
| `ettlex-mcp` | `src/server.rs` | Add 5 dispatch arms in `dispatch_inner` |
| `ettlex-mcp` | `src/main.rs` | Add 5 `tool_def` entries in `handle_tools_list()` |

### Crates read-only (outside boundary)

- `ettlex-engine` — existing `handle_relation_get`, `handle_relation_list`, `handle_group_get`,
  `handle_group_list`, `handle_group_member_list` are called from MCP tools via
  `ettlex_memory::commands::relation::*` / `ettlex_memory::commands::group::*`. No engine changes.
- `ettlex-core`, `ettlex-core-types`, `ettlex-errors`, `ettlex-logging` — no changes
- `ettlex-memory` — re-exports engine; no direct modification
- `ettlex-cli`, `ettlex-tauri`, `ettlex-projection`, `ettlex-agent-api` — no changes

### Infrastructure exceptions

None. All changes are confined to declared boundary crates.

### Spec gap note — store function for `group_member_list(ettle_id)`

The Ettle specifies `group_member_list` accepts either `group_id` or `ettle_id` as a filter.
The existing `SqliteRepo::list_group_members(conn, group_id, include_tombstoned)` only accepts
`group_id`. The existing engine handler `handle_group_member_list(conn, group_id, include_tombstoned)`
mirrors this. There is no store function for listing memberships by `ettle_id`.

The Ettle's `why` section states "No engine or store work is required" — this appears to have
been written before recognising the ettle_id filter gap. To fully implement the spec's
`group_member_list(ettle_id: E)` scenario without layer violations, a minimal store addition
is required. This is declared in scope above. No engine handler change is needed — the MCP
tool calls the store function directly for the ettle_id branch (consistent with how other MCP
tools access the store when no engine handler covers the case).

---

## 4. Replacement Targets

None. This slice is purely additive.

---

## 5. Layer Coverage Declaration

| Layer | This slice | Notes |
|-------|-----------|-------|
| Store | In scope (minimal) | One new query function for `group_member_list(ettle_id)` |
| Engine | Read-only | Existing handlers called directly; no new variants |
| MCP transport | In scope | 5 new tools, dispatch arms, tool definitions |
| CLI | Out of scope | CLI read tools deferred |

All declared layers will have test coverage.

---

## 6. Deletion Impact Analysis

**Not applicable — Classification A (purely additive).**

---

## 7. Scenario Sequence for Destructive Slices

**Not applicable — not a destructive slice.**

---

## 8. Pre-Authorised Failure Registry (PAFR)

No existing tests are expected to fail. All changes are additive.

---

## 9. Scenario Inventory

Scenarios are numbered `SC-S02b-NN`. Test files:
- `ettlex-mcp/tests/relation_read_tools_tests.rs` — SC-S02b-01 through SC-S02b-18
- `ettlex-mcp/tests/group_read_tools_tests.rs` — SC-S02b-19 through SC-S02b-43

All scenarios sourced directly from the Ettle Gherkin.

### Feature: relation_get

| SC | Title | Layer | Error kind | Predicted RED failure | Module |
|----|-------|-------|------------|----------------------|--------|
| SC-S02b-01 | `relation_get` returns full record for active relation | MCP | — | Tool handler absent; `relation_get` arm missing in `server.rs` | `tools/relation.rs`, `server.rs` |
| SC-S02b-02 | `relation_get` returns tombstoned record | MCP | — | Tool handler absent | `tools/relation.rs` |
| SC-S02b-03 | `relation_get` returns `NotFound` for unknown relation_id | MCP | `NotFound` | Tool handler absent | `tools/relation.rs` |
| SC-S02b-04 | `relation_get` does not use `ettlex_apply` path | MCP (conformance) | — | Tool handler absent (can't assert absence of calls on absent code) | `tools/relation.rs` |
| SC-S02b-05 | `relation_get` is byte-identical across repeated calls | MCP | — | Tool handler absent | `tools/relation.rs` |
| SC-S02b-06 | `relation_get` errors are logged with relation_id | MCP | — | Tool handler absent | `tools/relation.rs` |
| SC-S02b-07 | `relation_get` does not mutate store state (state_version unchanged) | MCP | — | Tool handler absent | `tools/relation.rs` |
| SC-S02b-08 | `relation_get` response fields match stored record byte-for-byte | MCP | — | Tool handler absent | `tools/relation.rs` |

### Feature: relation_list

| SC | Title | Layer | Error kind | Predicted RED failure | Module |
|----|-------|-------|------------|----------------------|--------|
| SC-S02b-09 | `relation_list` filtered by `source_ettle_id` returns matching, excluding tombstoned | MCP | — | `relation_list` arm missing | `tools/relation.rs`, `server.rs` |
| SC-S02b-10 | `relation_list` filtered by `target_ettle_id` returns matching | MCP | — | Tool handler absent | `tools/relation.rs` |
| SC-S02b-11 | `relation_list` filtered by both source and target returns intersection | MCP | — | Tool handler absent | `tools/relation.rs` |
| SC-S02b-12 | `relation_list` with `include_tombstoned: true` includes tombstoned | MCP | — | Tool handler absent | `tools/relation.rs` |
| SC-S02b-13 | `relation_list` with neither filter returns `InvalidInput` | MCP | `InvalidInput` | Tool handler absent | `tools/relation.rs` |
| SC-S02b-14 | `relation_list` returns empty list when no relations match | MCP | — | Tool handler absent | `tools/relation.rs` |
| SC-S02b-15 | `relation_list` pagination is complete and non-overlapping | MCP | — | Tool handler absent | `tools/relation.rs` |
| SC-S02b-16 | `relation_list` does not use `ettlex_apply` path | MCP (conformance) | — | Tool handler absent | `tools/relation.rs` |
| SC-S02b-17 | `relation_list` ordering is deterministic (`created_at ASC, relation_id ASC`) | MCP | — | Tool handler absent | `tools/relation.rs` |
| SC-S02b-18 | `relation_list` does not mutate store state | MCP | — | Tool handler absent | `tools/relation.rs` |

### Feature: group_get

| SC | Title | Layer | Error kind | Predicted RED failure | Module |
|----|-------|-------|------------|----------------------|--------|
| SC-S02b-19 | `group_get` returns full record for active group | MCP | — | `group_get` arm missing | `tools/group.rs`, `server.rs` |
| SC-S02b-20 | `group_get` returns tombstoned group | MCP | — | Tool handler absent | `tools/group.rs` |
| SC-S02b-21 | `group_get` returns `NotFound` for unknown group_id | MCP | `NotFound` | Tool handler absent | `tools/group.rs` |
| SC-S02b-22 | `group_get` does not use `ettlex_apply` path | MCP (conformance) | — | Tool handler absent | `tools/group.rs` |
| SC-S02b-23 | `group_get` does not mutate store state | MCP | — | Tool handler absent | `tools/group.rs` |
| SC-S02b-24 | `group_get` response fields match stored record byte-for-byte | MCP | — | Tool handler absent | `tools/group.rs` |

### Feature: group_list

| SC | Title | Layer | Error kind | Predicted RED failure | Module |
|----|-------|-------|------------|----------------------|--------|
| SC-S02b-25 | `group_list` returns all active groups (tombstoned excluded by default) | MCP | — | `group_list` arm missing | `tools/group.rs`, `server.rs` |
| SC-S02b-26 | `group_list` with `include_tombstoned: true` returns all groups | MCP | — | Tool handler absent | `tools/group.rs` |
| SC-S02b-27 | `group_list` pagination is complete and non-overlapping | MCP | — | Tool handler absent | `tools/group.rs` |
| SC-S02b-28 | `group_list` returns empty list when no groups exist | MCP | — | Tool handler absent | `tools/group.rs` |
| SC-S02b-29 | `group_list` ordering is deterministic (`created_at ASC, group_id ASC`) | MCP | — | Tool handler absent | `tools/group.rs` |
| SC-S02b-30 | `group_list` does not mutate store state | MCP | — | Tool handler absent | `tools/group.rs` |

### Feature: group_member_list

| SC | Title | Layer | Error kind | Predicted RED failure | Module |
|----|-------|-------|------------|----------------------|--------|
| SC-S02b-31 | `group_member_list` filtered by `group_id` returns all members | MCP | — | `group_member_list` arm missing | `tools/group.rs`, `server.rs` |
| SC-S02b-32 | `group_member_list` filtered by `ettle_id` returns all memberships for that ettle | MCP | — | Tool handler absent; store fn `list_group_members_by_filter` absent | `sqlite_repo.rs`, `tools/group.rs` |
| SC-S02b-33 | `group_member_list` filtered by both `group_id` and `ettle_id` returns intersection | MCP | — | Tool handler absent | `tools/group.rs` |
| SC-S02b-34 | `group_member_list` with `include_tombstoned: true` includes removed memberships | MCP | — | Tool handler absent | `tools/group.rs` |
| SC-S02b-35 | `group_member_list` with neither filter returns `InvalidInput` | MCP | `InvalidInput` | Tool handler absent | `tools/group.rs` |
| SC-S02b-36 | `group_member_list` returns empty list when no memberships match | MCP | — | Tool handler absent | `tools/group.rs` |
| SC-S02b-37 | `group_member_list` pagination is complete and non-overlapping | MCP | — | Tool handler absent | `tools/group.rs` |
| SC-S02b-38 | `group_member_list` does not use `ettlex_apply` path | MCP (conformance) | — | Tool handler absent | `tools/group.rs` |
| SC-S02b-39 | `group_member_list` ordering is deterministic (`created_at ASC`) | MCP | — | Tool handler absent | `tools/group.rs` |
| SC-S02b-40 | Application startup registers all five new MCP tools in tool list | MCP | — | Tool defs absent from `handle_tools_list()` | `main.rs` |
| SC-S02b-41 | `group_member_list` does not mutate store state | MCP | — | Tool handler absent | `tools/group.rs` |
| SC-S02b-42 | None of the five new tools invoke the write command path | MCP (conformance) | — | Tools absent | `server.rs` |

**Total: 42 scenarios** (all sourced from Ettle Gherkin)

---

## 10. Makefile Update Plan

`SLICE_TEST_FILTER` is updated by `vs-close` from `handoff/slice_registry.toml`. No Makefile
changes are needed prior to implementation. Existing `test` and `test-full` targets unchanged.

---

## 11. Slice Registry Update Plan

Append the following to `handoff/slice_registry.toml` on successful completion:

```toml
[[slice]]
id = "slice-02b-mcp-read-surface-patch"
ettle_id = "ettle:019d1d8d-d483-7941-a694-cd6924df612b"
description = "MCP read surface patch for Relations and Groups — 5 new read tools (relation_get, relation_list, group_get, group_list, group_member_list), bypassing write path"
layers = ["store", "mcp"]
status = "complete"

[[slice.tests]]
crate = "ettlex-mcp"
file = "tests/relation_read_tools_tests.rs"
test = "test_s02b_relation_get_returns_full_record"
scenario = "SC-S02b-01"

[[slice.tests]]
crate = "ettlex-mcp"
file = "tests/relation_read_tools_tests.rs"
test = "test_s02b_relation_get_returns_tombstoned_record"
scenario = "SC-S02b-02"

[[slice.tests]]
crate = "ettlex-mcp"
file = "tests/relation_read_tools_tests.rs"
test = "test_s02b_relation_get_not_found"
scenario = "SC-S02b-03"

[[slice.tests]]
crate = "ettlex-mcp"
file = "tests/relation_read_tools_tests.rs"
test = "test_s02b_relation_get_does_not_use_apply_path"
scenario = "SC-S02b-04"

[[slice.tests]]
crate = "ettlex-mcp"
file = "tests/relation_read_tools_tests.rs"
test = "test_s02b_relation_get_byte_identical_repeated"
scenario = "SC-S02b-05"

[[slice.tests]]
crate = "ettlex-mcp"
file = "tests/relation_read_tools_tests.rs"
test = "test_s02b_relation_get_error_logged_with_relation_id"
scenario = "SC-S02b-06"

[[slice.tests]]
crate = "ettlex-mcp"
file = "tests/relation_read_tools_tests.rs"
test = "test_s02b_relation_get_does_not_mutate_state"
scenario = "SC-S02b-07"

[[slice.tests]]
crate = "ettlex-mcp"
file = "tests/relation_read_tools_tests.rs"
test = "test_s02b_relation_get_fields_match_stored_record"
scenario = "SC-S02b-08"

[[slice.tests]]
crate = "ettlex-mcp"
file = "tests/relation_read_tools_tests.rs"
test = "test_s02b_relation_list_by_source_returns_matching"
scenario = "SC-S02b-09"

[[slice.tests]]
crate = "ettlex-mcp"
file = "tests/relation_read_tools_tests.rs"
test = "test_s02b_relation_list_by_target_returns_matching"
scenario = "SC-S02b-10"

[[slice.tests]]
crate = "ettlex-mcp"
file = "tests/relation_read_tools_tests.rs"
test = "test_s02b_relation_list_by_source_and_target_returns_intersection"
scenario = "SC-S02b-11"

[[slice.tests]]
crate = "ettlex-mcp"
file = "tests/relation_read_tools_tests.rs"
test = "test_s02b_relation_list_include_tombstoned"
scenario = "SC-S02b-12"

[[slice.tests]]
crate = "ettlex-mcp"
file = "tests/relation_read_tools_tests.rs"
test = "test_s02b_relation_list_no_filter_returns_invalid_input"
scenario = "SC-S02b-13"

[[slice.tests]]
crate = "ettlex-mcp"
file = "tests/relation_read_tools_tests.rs"
test = "test_s02b_relation_list_empty_when_no_match"
scenario = "SC-S02b-14"

[[slice.tests]]
crate = "ettlex-mcp"
file = "tests/relation_read_tools_tests.rs"
test = "test_s02b_relation_list_pagination_complete_non_overlapping"
scenario = "SC-S02b-15"

[[slice.tests]]
crate = "ettlex-mcp"
file = "tests/relation_read_tools_tests.rs"
test = "test_s02b_relation_list_does_not_use_apply_path"
scenario = "SC-S02b-16"

[[slice.tests]]
crate = "ettlex-mcp"
file = "tests/relation_read_tools_tests.rs"
test = "test_s02b_relation_list_ordering_deterministic"
scenario = "SC-S02b-17"

[[slice.tests]]
crate = "ettlex-mcp"
file = "tests/relation_read_tools_tests.rs"
test = "test_s02b_relation_list_does_not_mutate_state"
scenario = "SC-S02b-18"

[[slice.tests]]
crate = "ettlex-mcp"
file = "tests/group_read_tools_tests.rs"
test = "test_s02b_group_get_returns_full_record"
scenario = "SC-S02b-19"

[[slice.tests]]
crate = "ettlex-mcp"
file = "tests/group_read_tools_tests.rs"
test = "test_s02b_group_get_returns_tombstoned_group"
scenario = "SC-S02b-20"

[[slice.tests]]
crate = "ettlex-mcp"
file = "tests/group_read_tools_tests.rs"
test = "test_s02b_group_get_not_found"
scenario = "SC-S02b-21"

[[slice.tests]]
crate = "ettlex-mcp"
file = "tests/group_read_tools_tests.rs"
test = "test_s02b_group_get_does_not_use_apply_path"
scenario = "SC-S02b-22"

[[slice.tests]]
crate = "ettlex-mcp"
file = "tests/group_read_tools_tests.rs"
test = "test_s02b_group_get_does_not_mutate_state"
scenario = "SC-S02b-23"

[[slice.tests]]
crate = "ettlex-mcp"
file = "tests/group_read_tools_tests.rs"
test = "test_s02b_group_get_fields_match_stored_record"
scenario = "SC-S02b-24"

[[slice.tests]]
crate = "ettlex-mcp"
file = "tests/group_read_tools_tests.rs"
test = "test_s02b_group_list_returns_active_groups"
scenario = "SC-S02b-25"

[[slice.tests]]
crate = "ettlex-mcp"
file = "tests/group_read_tools_tests.rs"
test = "test_s02b_group_list_include_tombstoned"
scenario = "SC-S02b-26"

[[slice.tests]]
crate = "ettlex-mcp"
file = "tests/group_read_tools_tests.rs"
test = "test_s02b_group_list_pagination_complete_non_overlapping"
scenario = "SC-S02b-27"

[[slice.tests]]
crate = "ettlex-mcp"
file = "tests/group_read_tools_tests.rs"
test = "test_s02b_group_list_empty_when_no_groups"
scenario = "SC-S02b-28"

[[slice.tests]]
crate = "ettlex-mcp"
file = "tests/group_read_tools_tests.rs"
test = "test_s02b_group_list_ordering_deterministic"
scenario = "SC-S02b-29"

[[slice.tests]]
crate = "ettlex-mcp"
file = "tests/group_read_tools_tests.rs"
test = "test_s02b_group_list_does_not_mutate_state"
scenario = "SC-S02b-30"

[[slice.tests]]
crate = "ettlex-mcp"
file = "tests/group_read_tools_tests.rs"
test = "test_s02b_group_member_list_by_group_id"
scenario = "SC-S02b-31"

[[slice.tests]]
crate = "ettlex-mcp"
file = "tests/group_read_tools_tests.rs"
test = "test_s02b_group_member_list_by_ettle_id"
scenario = "SC-S02b-32"

[[slice.tests]]
crate = "ettlex-mcp"
file = "tests/group_read_tools_tests.rs"
test = "test_s02b_group_member_list_by_group_and_ettle_intersection"
scenario = "SC-S02b-33"

[[slice.tests]]
crate = "ettlex-mcp"
file = "tests/group_read_tools_tests.rs"
test = "test_s02b_group_member_list_include_tombstoned"
scenario = "SC-S02b-34"

[[slice.tests]]
crate = "ettlex-mcp"
file = "tests/group_read_tools_tests.rs"
test = "test_s02b_group_member_list_no_filter_returns_invalid_input"
scenario = "SC-S02b-35"

[[slice.tests]]
crate = "ettlex-mcp"
file = "tests/group_read_tools_tests.rs"
test = "test_s02b_group_member_list_empty_when_no_match"
scenario = "SC-S02b-36"

[[slice.tests]]
crate = "ettlex-mcp"
file = "tests/group_read_tools_tests.rs"
test = "test_s02b_group_member_list_pagination_complete_non_overlapping"
scenario = "SC-S02b-37"

[[slice.tests]]
crate = "ettlex-mcp"
file = "tests/group_read_tools_tests.rs"
test = "test_s02b_group_member_list_does_not_use_apply_path"
scenario = "SC-S02b-38"

[[slice.tests]]
crate = "ettlex-mcp"
file = "tests/group_read_tools_tests.rs"
test = "test_s02b_group_member_list_ordering_deterministic"
scenario = "SC-S02b-39"

[[slice.tests]]
crate = "ettlex-mcp"
file = "tests/group_read_tools_tests.rs"
test = "test_s02b_all_five_tools_registered_at_startup"
scenario = "SC-S02b-40"

[[slice.tests]]
crate = "ettlex-mcp"
file = "tests/group_read_tools_tests.rs"
test = "test_s02b_group_member_list_does_not_mutate_state"
scenario = "SC-S02b-41"

[[slice.tests]]
crate = "ettlex-mcp"
file = "tests/group_read_tools_tests.rs"
test = "test_s02b_no_new_tool_invokes_write_command_path"
scenario = "SC-S02b-42"
```

---

## 12. Acceptance Strategy

### RED gate (per scenario)
1. Write test from Ettle Gherkin spec only — no peeking at production code.
2. Run `cargo build -p ettlex-mcp` — confirm compile failure (missing handler / dispatch arm).
3. Only then write production code.

### GREEN gate (per scenario)
1. Implement minimum production code.
2. Run `make test-slice` — confirm test passes, no regressions.

### Step 4C (per scenario, after GREEN — mandatory before next scenario)
1. Update rustdoc (`///`) on every new public function.
2. Update `handle_tools_list()` in `main.rs` for each new `tool_def`.
3. Run `make lint` — confirm clean.
4. Run `make doc` — confirm no new warnings.
5. Set scenario status = DONE in `handoff/slice_wip.md`.

### Final gates
- `make lint` — clean
- `make test-slice` — all 42 scenarios pass
- `make coverage-check` — coverage ≥ 80%

---

## 13. Plan Integrity Declaration

> No production code will be written before RED evidence exists.
> No code outside the declared slice boundary will be modified except the Makefile and
> handoff/slice_registry.toml (and any declared infrastructure exceptions).
> All replacement targets have been identified and their post-slice structural invariants
> declared.

---

## Appendix — Architecture Notes

### Why call engine handlers directly (not `apply_command`)

The `apply_command` function:
1. Takes `&mut Connection` (acquires a write lock even for reads)
2. Reads `state_version` from `command_log`
3. Appends a provenance event after every successful command
4. Inserts a row into `command_log` → increments `state_version`

Steps 3 and 4 apply to **all** commands dispatched through `apply_command`, including
`Command::RelationGet` and friends. This means a `relation_get` call today mutates
`command_log` and `provenance_events` — a defect.

The fix: MCP read tools call the engine handler functions directly:
```rust
// tools/relation.rs
use ettlex_memory::commands::relation::{handle_relation_get, handle_relation_list};
```

These functions take `&Connection` (not `&mut`), perform no writes, and return `CommandResult`
(destructured by the MCP handler to produce the JSON response).

### `group_member_list` implementation notes

The tool accepts `group_id?: string`, `ettle_id?: string`. At least one must be supplied.

- `group_id` only → calls `SqliteRepo::list_group_members(conn, &group_id, include_tombstoned)`
  via `ettlex_memory::commands::group::handle_group_member_list`
- `ettle_id` only or both → calls new `SqliteRepo::list_group_members_by_filter(conn, group_id, ettle_id, include_tombstoned)`
  (added to store in this slice)
- Neither → returns `InvalidInput`

The new store function issues:
```sql
SELECT id, group_id, ettle_id, created_at, tombstoned_at
FROM group_members
WHERE [group_id = ?1] AND [ettle_id = ?2] AND [tombstoned_at IS NULL]
ORDER BY created_at ASC, id ASC
```
with optional clauses based on which filters are present.

### `relation_list` filter constraint

Per spec, at least one of `source_ettle_id` or `target_ettle_id` must be supplied. The MCP
tool enforces this before calling the engine handler (which additionally accepts
`relation_type` as a standalone filter — the MCP layer does not expose `relation_type` as a
standalone filter per spec W1).

### Pagination convention

`relation_list`, `group_list`, and `group_member_list` use cursor-based pagination consistent
with the existing `ettle_list` convention: opaque base64 URL-safe no-pad cursor encoding
ordering fields. Helpers in `ettlex_memory::commands::read_tools` are reused.

### Tool response shapes (per spec W1)

| Tool | Response fields |
|------|----------------|
| `relation_get` | `relation_id`, `relation_type`, `source_ettle_id`, `target_ettle_id`, `properties_json`, `created_at`, `tombstoned_at` |
| `relation_list` | `{ items: [...], cursor? }` where each item has the same fields as `relation_get` |
| `group_get` | `group_id`, `name`, `created_at`, `tombstoned_at` |
| `group_list` | `{ items: [...], cursor? }` where each item has `group_id`, `name`, `created_at`, `tombstoned_at` |
| `group_member_list` | `{ items: [...], cursor? }` where each item has `group_id`, `ettle_id`, `created_at`, `tombstoned_at` |
