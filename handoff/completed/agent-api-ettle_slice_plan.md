# Vertical Slice Plan: Agent API — Ettle, Relation, and Group Operations

---

## 1. Slice Identifier

`agent-api-ettle`

---

## 2. Change Classification

**A — New behaviour.** The `ettlex-agent-api` crate is currently a stub (`src/lib.rs` contains only a doc comment). This slice introduces all public API surface, modules, and tests from scratch. No existing behaviour is changed.

---

## 3. Slice Boundary Declaration

### In-scope crates and modules

| Crate | Modules (new or changed) |
|---|---|
| `ettlex-agent-api` | `src/lib.rs` (re-exports), `src/boundary/mod.rs` (new), `src/boundary/mapping.rs` (new), `src/operations/mod.rs` (new), `src/operations/ettle.rs` (new), `src/operations/relation.rs` (new, Phase 3), `src/operations/group.rs` (new, Phase 3), `tests/agent_api_ettle_tests.rs` (new), `tests/agent_api_relation_tests.rs` (new, Phase 3), `tests/agent_api_group_tests.rs` (new, Phase 3), `tests/agent_api_conformance_tests.rs` (new), `README.md` (new) |

### Read-only (outside boundary)

All other crates: `ettlex-memory`, `ettlex-engine`, `ettlex-store`, `ettlex-core`, `ettlex-errors`, `ettlex-logging`, `ettlex-mcp`, `ettlex-cli`, `ettlex-core-types`.

### Infrastructure exceptions (mechanical, justified)

1. **`crates/ettlex-memory/src/lib.rs`** — Add pub re-exports of types required by the `ettlex-agent-api` public surface so that downstream callers need only `ettlex-memory` as a workspace dependency. Specifically re-export:
   - `FsStore` from `ettlex_store::cas`
   - `ExError`, `ExErrorKind` from `ettlex_core::errors`
   - `PolicyProvider` from `ettlex_core::policy_provider`
   - `ApprovalRouter` from `ettlex_core::approval_router`
   - `EttleRecord`, `EttleListOpts`, `EttleListPage`, `EttleListItem`, `EttleCursor` from `ettlex_store::model`
   - `RelationRecord`, `RelationListOpts`, `GroupRecord`, `GroupMemberRecord` from `ettlex_store::model`
   - `EttleContext` from `ettlex_memory::memory_manager` (already pub; expose at crate root)
   - `rusqlite::Connection` re-exported for callers that need it without a direct rusqlite dep

   **Justification:** The conformance scenario SC-23 requires that `ettlex-agent-api/Cargo.toml` lists `ettlex-memory` as the only workspace dependency. All types used in the agent API's public function signatures must therefore flow through `ettlex-memory` re-exports. This is an additive mechanical change to `lib.rs` only — no logic is changed.

2. **`crates/ettlex-agent-api/Cargo.toml`** — Add `rusqlite` as a direct non-workspace dependency (needed for the `Connection` type in function signatures). Add `tempfile` and `serde_json` to `[dev-dependencies]`. `ettlex-memory` remains the only workspace dependency.

3. **`makefile`** — Append new test names to `SLICE_TEST_FILTER`. No structural change to targets.

4. **`handoff/slice_registry.toml`** — Append new `[[slice]]` entry on completion.

---

## 4. Replacement Targets

None. This is a pure addition slice. The stub `ettlex-agent-api/src/lib.rs` is extended. No existing function or module is replaced.

---

## 5. Layer Coverage Declaration

| Layer | Covered? | Notes |
|---|---|---|
| Store | No | Store functions already exist; agent-api reads through engine queries |
| Engine | No | Engine handlers already exist; agent-api delegates via ettlex-memory |
| Action (`apply_command`) | Yes (write path) | All agent-api write ops route through `MemoryManager::apply_command` |
| MCP | No | Out of scope |
| CLI | No | Out of scope |
| **Agent API** | **Yes** | All Phase 1 + Phase 2 + Phase 3 operations + conformance |

The test suite exercises the agent-api layer directly. Engine and store behaviour is exercised transitively through agent-api calls on real in-memory SQLite databases.

---

## 6. Deletion Impact Analysis

Not applicable. Classification A — no deletions.

---

## 7. Scenario Sequence for Destructive Slices

Not applicable. Classification A.

---

## 8. Pre-Authorised Failure Registry (PAFR)

None. This slice adds new tests in a new crate. No existing test is affected.

---

## 9. Scenario Inventory

### Phase 1 — EttleGet

#### SC-01 — EttleGet returns full record
- **Layer:** Agent API (`operations/ettle.rs`)
- **File:** `tests/agent_api_ettle_tests.rs`
- **Test:** `test_agent_ettle_get_returns_full_record`
- **Scenario:** Create an Ettle with all fields (title, why, what, how, reasoning_link_id, reasoning_link_type). Call `agent_ettle_get`. Assert all fields match stored values exactly.
- **RED failure:** `agent_ettle_get` does not exist → compile error.
- **Production module:** `src/operations/ettle.rs` — `pub fn agent_ettle_get`.

#### SC-02 — EttleGet returns tombstoned record
- **Layer:** Agent API
- **File:** `tests/agent_api_ettle_tests.rs`
- **Test:** `test_agent_ettle_get_returns_tombstoned`
- **Scenario:** Create and tombstone an Ettle. Call `agent_ettle_get`. Assert `tombstoned_at` is non-null ISO-8601.
- **RED failure:** Compile error.

#### SC-03 — EttleGet NotFound
- **Layer:** Agent API
- **File:** `tests/agent_api_ettle_tests.rs`
- **Test:** `test_agent_ettle_get_not_found`
- **Expected error kind:** `ExErrorKind::NotFound`
- **Scenario:** Call `agent_ettle_get("ettle:does-not-exist")`. Assert `ExErrorKind::NotFound`.
- **RED failure:** Compile error.

#### SC-04 — EttleGet byte-identical
- **Layer:** Agent API
- **File:** `tests/agent_api_ettle_tests.rs`
- **Test:** `test_agent_ettle_get_byte_identical`
- **Scenario:** Call `agent_ettle_get` twice without mutation. Serialize both results. Assert equality.
- **RED failure:** Compile error.

#### SC-05 — EttleGet lifecycle events owned by boundary
- **Layer:** Agent API (boundary logging)
- **File:** `tests/agent_api_ettle_tests.rs`
- **Test:** `test_agent_ettle_get_lifecycle_events`
- **Scenario:** Use `TestCapture`. Call `agent_ettle_get`. Assert exactly one `start` and one `end`/`end_error` event with `op="agent_ettle_get"`. Assert no lifecycle events for this op from engine or store.
- **RED failure:** Compile error.
- **Production module:** `src/operations/ettle.rs` — `log_op_start!/end!` wrapping.

---

### Phase 1 — EttleContext

#### SC-06 — agent_ettle_context returns assembled context
- **Layer:** Agent API (delegates to `MemoryManager::assemble_ettle_context`)
- **File:** `tests/agent_api_ettle_tests.rs`
- **Test:** `test_agent_ettle_context_assembled`
- **Scenario:** Create Ettle E with outgoing relations and group memberships. Call `agent_ettle_context(E)`. Assert why/what/how fields present, active relations list populated, active groups list populated.
- **RED failure:** `agent_ettle_context` does not exist → compile error.
- **Production module:** `src/operations/ettle.rs` — `pub fn agent_ettle_context`.

#### SC-07 — agent_ettle_context NotFound
- **Layer:** Agent API
- **File:** `tests/agent_api_ettle_tests.rs`
- **Test:** `test_agent_ettle_context_not_found`
- **Expected error kind:** `ExErrorKind::NotFound`
- **Scenario:** Call `agent_ettle_context("ettle:missing")`. Assert `ExErrorKind::NotFound`.
- **RED failure:** Compile error.

---

### Phase 1 — EttleList

#### SC-08 — EttleList active Ettles in deterministic order
- **Layer:** Agent API
- **File:** `tests/agent_api_ettle_tests.rs`
- **Test:** `test_agent_ettle_list_active_deterministic`
- **Scenario:** Create Ettles A then B; tombstone one. Call `agent_ettle_list(limit: 100, include_tombstoned: false)`. Assert items ordered created_at ASC, id ASC; tombstoned excluded.
- **RED failure:** `agent_ettle_list` does not exist → compile error.
- **Production module:** `src/operations/ettle.rs` — `pub fn agent_ettle_list`.

#### SC-09 — EttleList pagination deterministic and complete
- **Layer:** Agent API
- **File:** `tests/agent_api_ettle_tests.rs`
- **Test:** `test_agent_ettle_list_pagination`
- **Scenario:** Create 10 Ettles. Call `agent_ettle_list(limit: 5)`. Assert 5 items + cursor. Call again with cursor. Assert next 5 items + no cursor. Assert no item appears in both pages.
- **RED failure:** Compile error.

#### SC-10 — EttleList limit=0 rejected
- **Layer:** Agent API
- **File:** `tests/agent_api_ettle_tests.rs`
- **Test:** `test_agent_ettle_list_limit_zero_rejected`
- **Expected error kind:** `ExErrorKind::InvalidInput`
- **Scenario:** Call `agent_ettle_list` with `limit: 0`. Assert `ExErrorKind::InvalidInput`.
- **RED failure:** Compile error.

---

### Phase 2 — EttleCreate

#### SC-11 — EttleCreate title only succeeds
- **Layer:** Agent API (write path via `MemoryManager::apply_command`)
- **File:** `tests/agent_api_ettle_tests.rs`
- **Test:** `test_agent_ettle_create_title_only`
- **Scenario:** Call `agent_ettle_create(title: "My Ettle")`. Assert returned `ettle_id` starts with `"ettle:"`. Call `agent_ettle_get` — assert why="", what="", how="". Assert provenance event `ettle_created` recorded.
- **RED failure:** `agent_ettle_create` does not exist → compile error.
- **Production module:** `src/operations/ettle.rs` — `pub fn agent_ettle_create` routing through `MemoryManager::apply_command`.

#### SC-12 — EttleCreate rejects empty title
- **Layer:** Agent API
- **File:** `tests/agent_api_ettle_tests.rs`
- **Test:** `test_agent_ettle_create_empty_title`
- **Expected error kind:** `ExErrorKind::InvalidTitle`
- **Scenario:** Call `agent_ettle_create(title: "")`. Assert `ExErrorKind::InvalidTitle`. Assert no provenance event.
- **RED failure:** Compile error.

#### SC-13 — EttleCreate rejects caller-supplied ettle_id
- **Layer:** Agent API
- **File:** `tests/agent_api_ettle_tests.rs`
- **Test:** `test_agent_ettle_create_rejects_caller_id`
- **Expected error kind:** `ExErrorKind::InvalidInput`
- **Scenario:** Populate `ettle_id` field on `AgentEttleCreate` and call. Assert `ExErrorKind::InvalidInput`.
- **RED failure:** Compile error.
- **Production module:** `src/operations/ettle.rs` — input guard rejecting non-None `ettle_id`.

#### SC-14 — EttleCreate rejects reasoning_link_id without type
- **Layer:** Agent API
- **File:** `tests/agent_api_ettle_tests.rs`
- **Test:** `test_agent_ettle_create_link_without_type`
- **Expected error kind:** `ExErrorKind::MissingLinkType`
- **Scenario:** Call `agent_ettle_create(title: "T", reasoning_link_id: "ettle:P")` without `reasoning_link_type`. Assert `ExErrorKind::MissingLinkType`.
- **RED failure:** Compile error.

---

### Phase 2 — EttleUpdate

#### SC-15 — EttleUpdate changes WHY/WHAT/HOW
- **Layer:** Agent API
- **File:** `tests/agent_api_ettle_tests.rs`
- **Test:** `test_agent_ettle_update_fields`
- **Scenario:** Create Ettle E. Call `agent_ettle_update(ettle_id: E, why: "W2", what: "X2", how: "H2")`. Assert `agent_ettle_get` returns updated values. Assert provenance event `ettle_updated`.
- **RED failure:** `agent_ettle_update` does not exist → compile error.
- **Production module:** `src/operations/ettle.rs` — `pub fn agent_ettle_update`.

#### SC-16 — EttleUpdate clears reasoning_link via double-Option
- **Layer:** Agent API
- **File:** `tests/agent_api_ettle_tests.rs`
- **Test:** `test_agent_ettle_update_clears_reasoning_link`
- **Scenario:** Create Ettle E with reasoning_link_id = P. Call `agent_ettle_update(ettle_id: E, reasoning_link_id: Some(None))`. Assert `agent_ettle_get(E).reasoning_link_id` is null.
- **RED failure:** Compile error.
- **Production module:** `src/operations/ettle.rs` — `AgentEttleUpdate` with double-Option.

#### SC-17 — EttleUpdate preserves unspecified fields
- **Layer:** Agent API
- **File:** `tests/agent_api_ettle_tests.rs`
- **Test:** `test_agent_ettle_update_preserves_unspecified`
- **Scenario:** Create Ettle E with why="W", what="X", how="H". Call `agent_ettle_update(ettle_id: E, title: "T2")`. Assert `agent_ettle_get(E).why = "W"`.
- **RED failure:** Compile error.

#### SC-18 — EttleUpdate rejects tombstoned Ettle
- **Layer:** Agent API
- **File:** `tests/agent_api_ettle_tests.rs`
- **Test:** `test_agent_ettle_update_rejects_tombstoned`
- **Expected error kind:** `ExErrorKind::AlreadyTombstoned`
- **Scenario:** Tombstone Ettle E. Call `agent_ettle_update`. Assert `ExErrorKind::AlreadyTombstoned`.
- **RED failure:** Compile error.

---

### Phase 2 — EttleTombstone

#### SC-19 — EttleTombstone marks Ettle inactive
- **Layer:** Agent API
- **File:** `tests/agent_api_ettle_tests.rs`
- **Test:** `test_agent_ettle_tombstone_marks_inactive`
- **Scenario:** Create Ettle E with no active dependants. Call `agent_ettle_tombstone(E)`. Assert `agent_ettle_get(E).tombstoned_at` is non-null. Assert row still exists. Assert provenance event `ettle_tombstoned`.
- **RED failure:** `agent_ettle_tombstone` does not exist → compile error.
- **Production module:** `src/operations/ettle.rs` — `pub fn agent_ettle_tombstone`.

#### SC-20 — EttleTombstone rejects Ettle with active dependants
- **Layer:** Agent API
- **File:** `tests/agent_api_ettle_tests.rs`
- **Test:** `test_agent_ettle_tombstone_rejects_active_dependants`
- **Expected error kind:** `ExErrorKind::HasActiveDependants`
- **Scenario:** Create Ettles P and C where C has reasoning_link to P. Call `agent_ettle_tombstone(P)`. Assert `ExErrorKind::HasActiveDependants`.
- **RED failure:** Compile error.

---

### OCC and provenance

#### SC-21 — OCC correct version succeeds
- **Layer:** Agent API (write path)
- **File:** `tests/agent_api_ettle_tests.rs`
- **Test:** `test_agent_occ_correct_version`
- **Scenario:** Read `state_version = V`. Call `agent_ettle_create` with `expected_state_version = Some(V)`. Assert success; `new_state_version = V + 1`.
- **RED failure:** Compile error.
- **Production module:** `src/operations/ettle.rs` — `expected_state_version` parameter threaded through `MemoryManager::apply_command`.

#### SC-22 — OCC wrong version fails
- **Layer:** Agent API (write path)
- **File:** `tests/agent_api_ettle_tests.rs`
- **Test:** `test_agent_occ_wrong_version`
- **Expected error kind:** `ExErrorKind::HeadMismatch`
- **Scenario:** Read `state_version = V` (V > 0). Call `agent_ettle_create` with `expected_state_version = Some(V - 1)`. Assert `ExErrorKind::HeadMismatch`. Assert no provenance event appended.
- **RED failure:** Compile error.

---

### Phase 3 — Relations

#### SC-23 — agent_relation_create succeeds
- **Layer:** Agent API (write path via `MemoryManager::apply_command`)
- **File:** `tests/agent_api_relation_tests.rs`
- **Test:** `test_agent_relation_create_succeeds`
- **Scenario:** Create Ettles A and B. Call `agent_relation_create(source: A, target: B, relation_type: "refinement")`. Assert returned `relation_id` starts with `"rel:"`. Call `agent_relation_get` — assert full record matches. Assert provenance event recorded.
- **RED failure:** `agent_relation_create` does not exist → compile error.
- **Production module:** `src/operations/relation.rs` — `pub fn agent_relation_create`.

#### SC-24 — agent_relation_create rejects caller-supplied id
- **Layer:** Agent API
- **File:** `tests/agent_api_relation_tests.rs`
- **Test:** `test_agent_relation_create_rejects_caller_id`
- **Expected error kind:** `ExErrorKind::InvalidInput`
- **Scenario:** Call `agent_relation_create` with `relation_id: Some("rel:manual")`. Assert `ExErrorKind::InvalidInput`.
- **RED failure:** Compile error.

#### SC-25 — agent_relation_create unknown relation type
- **Layer:** Agent API
- **File:** `tests/agent_api_relation_tests.rs`
- **Test:** `test_agent_relation_create_unknown_type`
- **Expected error kind:** `ExErrorKind::InvalidInput`
- **Scenario:** Call `agent_relation_create` with `relation_type: "does-not-exist"`. Assert `ExErrorKind::InvalidInput`.
- **RED failure:** Compile error.

#### SC-26 — agent_relation_create rejects self-referential
- **Layer:** Agent API
- **File:** `tests/agent_api_relation_tests.rs`
- **Test:** `test_agent_relation_create_self_referential`
- **Expected error kind:** `ExErrorKind::SelfReferentialLink`
- **Scenario:** Create Ettle A. Call `agent_relation_create(source: A, target: A, ...)`. Assert `ExErrorKind::SelfReferentialLink`.
- **RED failure:** Compile error.

#### SC-27 — agent_relation_create rejects missing source
- **Layer:** Agent API
- **File:** `tests/agent_api_relation_tests.rs`
- **Test:** `test_agent_relation_create_missing_source`
- **Expected error kind:** `ExErrorKind::NotFound`
- **Scenario:** Call `agent_relation_create(source: "ettle:missing", target: B, ...)`. Assert `ExErrorKind::NotFound`.
- **RED failure:** Compile error.

#### SC-28 — agent_relation_create rejects tombstoned source
- **Layer:** Agent API
- **File:** `tests/agent_api_relation_tests.rs`
- **Test:** `test_agent_relation_create_tombstoned_source`
- **Expected error kind:** `ExErrorKind::AlreadyTombstoned`
- **Scenario:** Create and tombstone Ettle P. Create Ettle Q. Call `agent_relation_create(source: P, target: Q, ...)`. Assert `ExErrorKind::AlreadyTombstoned`.
- **RED failure:** Compile error.

#### SC-29 — agent_relation_get returns full record
- **Layer:** Agent API
- **File:** `tests/agent_api_relation_tests.rs`
- **Test:** `test_agent_relation_get_returns_full_record`
- **Scenario:** Create Relation R. Call `agent_relation_get(R)`. Assert all fields (id, source_ettle_id, target_ettle_id, relation_type, properties_json, created_at, tombstoned_at=null) match.
- **RED failure:** `agent_relation_get` does not exist → compile error.
- **Production module:** `src/operations/relation.rs` — `pub fn agent_relation_get`.

#### SC-30 — agent_relation_get not found
- **Layer:** Agent API
- **File:** `tests/agent_api_relation_tests.rs`
- **Test:** `test_agent_relation_get_not_found`
- **Expected error kind:** `ExErrorKind::NotFound`
- **Scenario:** Call `agent_relation_get("rel:missing")`. Assert `ExErrorKind::NotFound`.
- **RED failure:** Compile error.

#### SC-31 — agent_relation_list by source
- **Layer:** Agent API
- **File:** `tests/agent_api_relation_tests.rs`
- **Test:** `test_agent_relation_list_by_source`
- **Scenario:** Create Ettles A, B, C. Create relations A→B and A→C and B→C. Call `agent_relation_list(source: A)`. Assert only A→B and A→C are returned.
- **RED failure:** `agent_relation_list` does not exist → compile error.
- **Production module:** `src/operations/relation.rs` — `pub fn agent_relation_list`.

#### SC-32 — agent_relation_list no filter fails
- **Layer:** Agent API
- **File:** `tests/agent_api_relation_tests.rs`
- **Test:** `test_agent_relation_list_no_filter_fails`
- **Expected error kind:** `ExErrorKind::InvalidInput`
- **Scenario:** Call `agent_relation_list` with no source, no target, no relation_type. Assert `ExErrorKind::InvalidInput`.
- **RED failure:** Compile error.

#### SC-33 — agent_relation_list ordering deterministic
- **Layer:** Agent API
- **File:** `tests/agent_api_relation_tests.rs`
- **Test:** `test_agent_relation_list_ordering_deterministic`
- **Scenario:** Create multiple relations from same source. Call `agent_relation_list(source: A)` twice. Assert both responses have identical ordering.
- **RED failure:** Compile error.

#### SC-34 — agent_relation_tombstone marks inactive
- **Layer:** Agent API
- **File:** `tests/agent_api_relation_tests.rs`
- **Test:** `test_agent_relation_tombstone_marks_inactive`
- **Scenario:** Create Relation R. Call `agent_relation_tombstone(R)`. Assert `agent_relation_get(R).tombstoned_at` is non-null. Assert provenance event recorded.
- **RED failure:** `agent_relation_tombstone` does not exist → compile error.
- **Production module:** `src/operations/relation.rs` — `pub fn agent_relation_tombstone`.

#### SC-35 — agent_relation_tombstone not found
- **Layer:** Agent API
- **File:** `tests/agent_api_relation_tests.rs`
- **Test:** `test_agent_relation_tombstone_not_found`
- **Expected error kind:** `ExErrorKind::NotFound`
- **Scenario:** Call `agent_relation_tombstone("rel:missing")`. Assert `ExErrorKind::NotFound`.
- **RED failure:** Compile error.

#### SC-36 — agent_relation_tombstone already tombstoned
- **Layer:** Agent API
- **File:** `tests/agent_api_relation_tests.rs`
- **Test:** `test_agent_relation_tombstone_already_tombstoned`
- **Expected error kind:** `ExErrorKind::AlreadyTombstoned`
- **Scenario:** Tombstone Relation R. Call `agent_relation_tombstone(R)` again. Assert `ExErrorKind::AlreadyTombstoned`.
- **RED failure:** Compile error.

---

### Phase 3 — Groups

#### SC-37 — agent_group_create succeeds
- **Layer:** Agent API
- **File:** `tests/agent_api_group_tests.rs`
- **Test:** `test_agent_group_create_succeeds`
- **Scenario:** Call `agent_group_create(name: "My Group")`. Assert returned `group_id` starts with `"grp:"`. Call `agent_group_get` — assert name matches. Assert provenance event recorded.
- **RED failure:** `agent_group_create` does not exist → compile error.
- **Production module:** `src/operations/group.rs` — `pub fn agent_group_create`.

#### SC-38 — agent_group_create empty name fails
- **Layer:** Agent API
- **File:** `tests/agent_api_group_tests.rs`
- **Test:** `test_agent_group_create_empty_name_fails`
- **Expected error kind:** `ExErrorKind::InvalidTitle`
- **Scenario:** Call `agent_group_create(name: "")`. Assert `ExErrorKind::InvalidTitle`.
- **RED failure:** Compile error.

#### SC-39 — agent_group_get returns full record
- **Layer:** Agent API
- **File:** `tests/agent_api_group_tests.rs`
- **Test:** `test_agent_group_get_returns_full_record`
- **Scenario:** Create Group G. Call `agent_group_get(G)`. Assert all fields (id, name, created_at, tombstoned_at=null) match.
- **RED failure:** `agent_group_get` does not exist → compile error.
- **Production module:** `src/operations/group.rs` — `pub fn agent_group_get`.

#### SC-40 — agent_group_get not found
- **Layer:** Agent API
- **File:** `tests/agent_api_group_tests.rs`
- **Test:** `test_agent_group_get_not_found`
- **Expected error kind:** `ExErrorKind::NotFound`
- **Scenario:** Call `agent_group_get("grp:missing")`. Assert `ExErrorKind::NotFound`.
- **RED failure:** Compile error.

#### SC-41 — agent_group_list returns active in deterministic order
- **Layer:** Agent API
- **File:** `tests/agent_api_group_tests.rs`
- **Test:** `test_agent_group_list_active_deterministic`
- **Scenario:** Create Groups A and B. Call `agent_group_list(include_tombstoned: false)` twice. Assert both calls return identical ordering. Assert active groups only (no tombstoned).
- **RED failure:** `agent_group_list` does not exist → compile error.
- **Production module:** `src/operations/group.rs` — `pub fn agent_group_list`.

#### SC-42 — agent_group_member_add succeeds
- **Layer:** Agent API
- **File:** `tests/agent_api_group_tests.rs`
- **Test:** `test_agent_group_member_add_succeeds`
- **Scenario:** Create Group G and Ettle E. Call `agent_group_member_add(group_id: G, ettle_id: E)`. Assert membership recorded. Call `agent_group_member_list(group_id: G)` — assert E appears. Assert provenance event recorded.
- **RED failure:** `agent_group_member_add` does not exist → compile error.
- **Production module:** `src/operations/group.rs` — `pub fn agent_group_member_add`.

#### SC-43 — agent_group_member_add duplicate fails
- **Layer:** Agent API
- **File:** `tests/agent_api_group_tests.rs`
- **Test:** `test_agent_group_member_add_duplicate_fails`
- **Expected error kind:** `ExErrorKind::ConstraintViolation` (or `DuplicateMapping` — confirm at implementation)
- **Scenario:** Add Ettle E to Group G. Add E to G again (active). Assert appropriate error kind.
- **RED failure:** Compile error.

#### SC-44 — agent_group_member_add tombstoned group fails
- **Layer:** Agent API
- **File:** `tests/agent_api_group_tests.rs`
- **Test:** `test_agent_group_member_add_tombstoned_group_fails`
- **Expected error kind:** `ExErrorKind::AlreadyTombstoned`
- **Scenario:** Create and tombstone Group G. Create Ettle E. Call `agent_group_member_add(group_id: G, ettle_id: E)`. Assert `ExErrorKind::AlreadyTombstoned`.
- **RED failure:** Compile error.

#### SC-45 — agent_group_member_remove tombstones membership
- **Layer:** Agent API
- **File:** `tests/agent_api_group_tests.rs`
- **Test:** `test_agent_group_member_remove_marks_tombstoned`
- **Scenario:** Add Ettle E to Group G. Call `agent_group_member_remove(group_id: G, ettle_id: E)`. Assert `agent_group_member_list(include_tombstoned: true)` shows membership as tombstoned. Assert provenance event recorded.
- **RED failure:** `agent_group_member_remove` does not exist → compile error.
- **Production module:** `src/operations/group.rs` — `pub fn agent_group_member_remove`.

#### SC-46 — agent_group_member_remove not found fails
- **Layer:** Agent API
- **File:** `tests/agent_api_group_tests.rs`
- **Test:** `test_agent_group_member_remove_not_found_fails`
- **Expected error kind:** `ExErrorKind::NotFound`
- **Scenario:** Create Group G and Ettle E (never added). Call `agent_group_member_remove(group_id: G, ettle_id: E)`. Assert `ExErrorKind::NotFound`.
- **RED failure:** Compile error.

#### SC-47 — agent_group_member_list by group_id
- **Layer:** Agent API
- **File:** `tests/agent_api_group_tests.rs`
- **Test:** `test_agent_group_member_list_by_group_id`
- **Scenario:** Add Ettles E1 and E2 to Group G; add E1 to Group H. Call `agent_group_member_list(group_id: G)`. Assert E1 and E2 appear; E1's H membership does not appear.
- **RED failure:** `agent_group_member_list` does not exist → compile error.
- **Production module:** `src/operations/group.rs` — `pub fn agent_group_member_list`.

#### SC-48 — agent_group_member_list no filter fails
- **Layer:** Agent API
- **File:** `tests/agent_api_group_tests.rs`
- **Test:** `test_agent_group_member_list_no_filter_fails`
- **Expected error kind:** `ExErrorKind::InvalidInput`
- **Scenario:** Call `agent_group_member_list` with neither `group_id` nor `ettle_id` specified. Assert `ExErrorKind::InvalidInput`.
- **RED failure:** Compile error.

---

### Architectural conformance

#### SC-49 — Agent API depends only on ettlex-memory (workspace)
- **Layer:** Conformance
- **File:** `tests/agent_api_conformance_tests.rs`
- **Test:** `test_agent_api_only_memory_dep`
- **Scenario:** Parse `ettlex-agent-api/Cargo.toml`. Assert `ettlex-memory` is the only key under `[dependencies]` starting with `ettlex-`. Assert `ettlex-engine`, `ettlex-store`, `ettlex-core` do not appear.
- **RED failure:** Test body compiles but fails until Cargo.toml is correct.

#### SC-50 — All write operations route through MemoryManager
- **Layer:** Conformance
- **File:** `tests/agent_api_conformance_tests.rs`
- **Test:** `test_agent_api_writes_route_through_memory`
- **Scenario:** Inspect source of `operations/ettle.rs`, `operations/relation.rs`, `operations/group.rs`. Assert no direct `ettlex_engine` import or `apply_command` call from `ettlex_engine`. Assert all mutation paths use `ettlex_memory`.
- **RED failure:** Structural assertion; fails until operations modules conform.

#### SC-51 — Exactly one boundary mapping module
- **Layer:** Conformance
- **File:** `tests/agent_api_conformance_tests.rs`
- **Test:** `test_agent_api_single_boundary_module`
- **Scenario:** Assert `ettlex-agent-api/src/boundary/mapping.rs` exists. Assert operation modules contain no `From<ExError>` or equivalent mapping logic.
- **RED failure:** Compile error (module absent).
- **Production module:** `src/boundary/mapping.rs`.

#### SC-52 — No WHY/WHAT/HOW content in log output
- **Layer:** Conformance
- **File:** `tests/agent_api_conformance_tests.rs`
- **Test:** `test_agent_api_no_why_what_how_in_logs`
- **Scenario:** Search `operations/*.rs` source for `why`, `what`, `how` used as log field names in `log_op_*` invocations. Assert zero matches.
- **RED failure:** Structural assertion; fails if any logging macro includes these field values.

#### SC-53 — No apply_mcp_command reference in workspace
- **Layer:** Conformance
- **File:** `tests/agent_api_conformance_tests.rs`
- **Test:** `test_agent_api_no_apply_mcp_command`
- **Scenario:** `grep -r` workspace source for `apply_mcp_command`. Assert zero results.
- **RED failure:** Will pass immediately (already retired in Slice 02); guard against regression.

---

## 10. Makefile Update Plan

Append the following 53 test names to `SLICE_TEST_FILTER` (joining with `|` inside the existing `test(/.../)` expression):

```
test_agent_ettle_get_returns_full_record
test_agent_ettle_get_returns_tombstoned
test_agent_ettle_get_not_found
test_agent_ettle_get_byte_identical
test_agent_ettle_get_lifecycle_events
test_agent_ettle_context_assembled
test_agent_ettle_context_not_found
test_agent_ettle_list_active_deterministic
test_agent_ettle_list_pagination
test_agent_ettle_list_limit_zero_rejected
test_agent_ettle_create_title_only
test_agent_ettle_create_empty_title
test_agent_ettle_create_rejects_caller_id
test_agent_ettle_create_link_without_type
test_agent_ettle_update_fields
test_agent_ettle_update_clears_reasoning_link
test_agent_ettle_update_preserves_unspecified
test_agent_ettle_update_rejects_tombstoned
test_agent_ettle_tombstone_marks_inactive
test_agent_ettle_tombstone_rejects_active_dependants
test_agent_occ_correct_version
test_agent_occ_wrong_version
test_agent_relation_create_succeeds
test_agent_relation_create_rejects_caller_id
test_agent_relation_create_unknown_type
test_agent_relation_create_self_referential
test_agent_relation_create_missing_source
test_agent_relation_create_tombstoned_source
test_agent_relation_get_returns_full_record
test_agent_relation_get_not_found
test_agent_relation_list_by_source
test_agent_relation_list_no_filter_fails
test_agent_relation_list_ordering_deterministic
test_agent_relation_tombstone_marks_inactive
test_agent_relation_tombstone_not_found
test_agent_relation_tombstone_already_tombstoned
test_agent_group_create_succeeds
test_agent_group_create_empty_name_fails
test_agent_group_get_returns_full_record
test_agent_group_get_not_found
test_agent_group_list_active_deterministic
test_agent_group_member_add_succeeds
test_agent_group_member_add_duplicate_fails
test_agent_group_member_add_tombstoned_group_fails
test_agent_group_member_remove_marks_tombstoned
test_agent_group_member_remove_not_found_fails
test_agent_group_member_list_by_group_id
test_agent_group_member_list_no_filter_fails
test_agent_api_only_memory_dep
test_agent_api_writes_route_through_memory
test_agent_api_single_boundary_module
test_agent_api_no_why_what_how_in_logs
test_agent_api_no_apply_mcp_command
```

**53 new test names.** None conflict with names already registered in `handoff/slice_registry.toml`. Existing `test`, `test-full`, and `test-slice` targets are unchanged in structure.

---

## 11. Slice Registry Update Plan

```toml
[[slice]]
id = "agent-api-ettle"
ettle_id = "ettle:019cf3a7-0b26-78f0-95b5-fab62af26b71"
description = "Implement ettlex-agent-api crate: Phase 1 (EttleGet, EttleList, EttleContext), Phase 2 (EttleCreate, EttleUpdate, EttleTombstone), Phase 3 (Relation and Group operations), and architectural conformance tests"
layers = ["agent-api"]
status = "complete"

[[slice.tests]]
crate = "ettlex-agent-api"
file = "tests/agent_api_ettle_tests.rs"
test = "test_agent_ettle_get_returns_full_record"
scenario = "SC-01"

[[slice.tests]]
crate = "ettlex-agent-api"
file = "tests/agent_api_ettle_tests.rs"
test = "test_agent_ettle_get_returns_tombstoned"
scenario = "SC-02"

[[slice.tests]]
crate = "ettlex-agent-api"
file = "tests/agent_api_ettle_tests.rs"
test = "test_agent_ettle_get_not_found"
scenario = "SC-03"

[[slice.tests]]
crate = "ettlex-agent-api"
file = "tests/agent_api_ettle_tests.rs"
test = "test_agent_ettle_get_byte_identical"
scenario = "SC-04"

[[slice.tests]]
crate = "ettlex-agent-api"
file = "tests/agent_api_ettle_tests.rs"
test = "test_agent_ettle_get_lifecycle_events"
scenario = "SC-05"

[[slice.tests]]
crate = "ettlex-agent-api"
file = "tests/agent_api_ettle_tests.rs"
test = "test_agent_ettle_context_assembled"
scenario = "SC-06"

[[slice.tests]]
crate = "ettlex-agent-api"
file = "tests/agent_api_ettle_tests.rs"
test = "test_agent_ettle_context_not_found"
scenario = "SC-07"

[[slice.tests]]
crate = "ettlex-agent-api"
file = "tests/agent_api_ettle_tests.rs"
test = "test_agent_ettle_list_active_deterministic"
scenario = "SC-08"

[[slice.tests]]
crate = "ettlex-agent-api"
file = "tests/agent_api_ettle_tests.rs"
test = "test_agent_ettle_list_pagination"
scenario = "SC-09"

[[slice.tests]]
crate = "ettlex-agent-api"
file = "tests/agent_api_ettle_tests.rs"
test = "test_agent_ettle_list_limit_zero_rejected"
scenario = "SC-10"

[[slice.tests]]
crate = "ettlex-agent-api"
file = "tests/agent_api_ettle_tests.rs"
test = "test_agent_ettle_create_title_only"
scenario = "SC-11"

[[slice.tests]]
crate = "ettlex-agent-api"
file = "tests/agent_api_ettle_tests.rs"
test = "test_agent_ettle_create_empty_title"
scenario = "SC-12"

[[slice.tests]]
crate = "ettlex-agent-api"
file = "tests/agent_api_ettle_tests.rs"
test = "test_agent_ettle_create_rejects_caller_id"
scenario = "SC-13"

[[slice.tests]]
crate = "ettlex-agent-api"
file = "tests/agent_api_ettle_tests.rs"
test = "test_agent_ettle_create_link_without_type"
scenario = "SC-14"

[[slice.tests]]
crate = "ettlex-agent-api"
file = "tests/agent_api_ettle_tests.rs"
test = "test_agent_ettle_update_fields"
scenario = "SC-15"

[[slice.tests]]
crate = "ettlex-agent-api"
file = "tests/agent_api_ettle_tests.rs"
test = "test_agent_ettle_update_clears_reasoning_link"
scenario = "SC-16"

[[slice.tests]]
crate = "ettlex-agent-api"
file = "tests/agent_api_ettle_tests.rs"
test = "test_agent_ettle_update_preserves_unspecified"
scenario = "SC-17"

[[slice.tests]]
crate = "ettlex-agent-api"
file = "tests/agent_api_ettle_tests.rs"
test = "test_agent_ettle_update_rejects_tombstoned"
scenario = "SC-18"

[[slice.tests]]
crate = "ettlex-agent-api"
file = "tests/agent_api_ettle_tests.rs"
test = "test_agent_ettle_tombstone_marks_inactive"
scenario = "SC-19"

[[slice.tests]]
crate = "ettlex-agent-api"
file = "tests/agent_api_ettle_tests.rs"
test = "test_agent_ettle_tombstone_rejects_active_dependants"
scenario = "SC-20"

[[slice.tests]]
crate = "ettlex-agent-api"
file = "tests/agent_api_ettle_tests.rs"
test = "test_agent_occ_correct_version"
scenario = "SC-21"

[[slice.tests]]
crate = "ettlex-agent-api"
file = "tests/agent_api_ettle_tests.rs"
test = "test_agent_occ_wrong_version"
scenario = "SC-22"

[[slice.tests]]
crate = "ettlex-agent-api"
file = "tests/agent_api_relation_tests.rs"
test = "test_agent_relation_create_succeeds"
scenario = "SC-23"

[[slice.tests]]
crate = "ettlex-agent-api"
file = "tests/agent_api_relation_tests.rs"
test = "test_agent_relation_create_rejects_caller_id"
scenario = "SC-24"

[[slice.tests]]
crate = "ettlex-agent-api"
file = "tests/agent_api_relation_tests.rs"
test = "test_agent_relation_create_unknown_type"
scenario = "SC-25"

[[slice.tests]]
crate = "ettlex-agent-api"
file = "tests/agent_api_relation_tests.rs"
test = "test_agent_relation_create_self_referential"
scenario = "SC-26"

[[slice.tests]]
crate = "ettlex-agent-api"
file = "tests/agent_api_relation_tests.rs"
test = "test_agent_relation_create_missing_source"
scenario = "SC-27"

[[slice.tests]]
crate = "ettlex-agent-api"
file = "tests/agent_api_relation_tests.rs"
test = "test_agent_relation_create_tombstoned_source"
scenario = "SC-28"

[[slice.tests]]
crate = "ettlex-agent-api"
file = "tests/agent_api_relation_tests.rs"
test = "test_agent_relation_get_returns_full_record"
scenario = "SC-29"

[[slice.tests]]
crate = "ettlex-agent-api"
file = "tests/agent_api_relation_tests.rs"
test = "test_agent_relation_get_not_found"
scenario = "SC-30"

[[slice.tests]]
crate = "ettlex-agent-api"
file = "tests/agent_api_relation_tests.rs"
test = "test_agent_relation_list_by_source"
scenario = "SC-31"

[[slice.tests]]
crate = "ettlex-agent-api"
file = "tests/agent_api_relation_tests.rs"
test = "test_agent_relation_list_no_filter_fails"
scenario = "SC-32"

[[slice.tests]]
crate = "ettlex-agent-api"
file = "tests/agent_api_relation_tests.rs"
test = "test_agent_relation_list_ordering_deterministic"
scenario = "SC-33"

[[slice.tests]]
crate = "ettlex-agent-api"
file = "tests/agent_api_relation_tests.rs"
test = "test_agent_relation_tombstone_marks_inactive"
scenario = "SC-34"

[[slice.tests]]
crate = "ettlex-agent-api"
file = "tests/agent_api_relation_tests.rs"
test = "test_agent_relation_tombstone_not_found"
scenario = "SC-35"

[[slice.tests]]
crate = "ettlex-agent-api"
file = "tests/agent_api_relation_tests.rs"
test = "test_agent_relation_tombstone_already_tombstoned"
scenario = "SC-36"

[[slice.tests]]
crate = "ettlex-agent-api"
file = "tests/agent_api_group_tests.rs"
test = "test_agent_group_create_succeeds"
scenario = "SC-37"

[[slice.tests]]
crate = "ettlex-agent-api"
file = "tests/agent_api_group_tests.rs"
test = "test_agent_group_create_empty_name_fails"
scenario = "SC-38"

[[slice.tests]]
crate = "ettlex-agent-api"
file = "tests/agent_api_group_tests.rs"
test = "test_agent_group_get_returns_full_record"
scenario = "SC-39"

[[slice.tests]]
crate = "ettlex-agent-api"
file = "tests/agent_api_group_tests.rs"
test = "test_agent_group_get_not_found"
scenario = "SC-40"

[[slice.tests]]
crate = "ettlex-agent-api"
file = "tests/agent_api_group_tests.rs"
test = "test_agent_group_list_active_deterministic"
scenario = "SC-41"

[[slice.tests]]
crate = "ettlex-agent-api"
file = "tests/agent_api_group_tests.rs"
test = "test_agent_group_member_add_succeeds"
scenario = "SC-42"

[[slice.tests]]
crate = "ettlex-agent-api"
file = "tests/agent_api_group_tests.rs"
test = "test_agent_group_member_add_duplicate_fails"
scenario = "SC-43"

[[slice.tests]]
crate = "ettlex-agent-api"
file = "tests/agent_api_group_tests.rs"
test = "test_agent_group_member_add_tombstoned_group_fails"
scenario = "SC-44"

[[slice.tests]]
crate = "ettlex-agent-api"
file = "tests/agent_api_group_tests.rs"
test = "test_agent_group_member_remove_marks_tombstoned"
scenario = "SC-45"

[[slice.tests]]
crate = "ettlex-agent-api"
file = "tests/agent_api_group_tests.rs"
test = "test_agent_group_member_remove_not_found_fails"
scenario = "SC-46"

[[slice.tests]]
crate = "ettlex-agent-api"
file = "tests/agent_api_group_tests.rs"
test = "test_agent_group_member_list_by_group_id"
scenario = "SC-47"

[[slice.tests]]
crate = "ettlex-agent-api"
file = "tests/agent_api_group_tests.rs"
test = "test_agent_group_member_list_no_filter_fails"
scenario = "SC-48"

[[slice.tests]]
crate = "ettlex-agent-api"
file = "tests/agent_api_conformance_tests.rs"
test = "test_agent_api_only_memory_dep"
scenario = "SC-49"

[[slice.tests]]
crate = "ettlex-agent-api"
file = "tests/agent_api_conformance_tests.rs"
test = "test_agent_api_writes_route_through_memory"
scenario = "SC-50"

[[slice.tests]]
crate = "ettlex-agent-api"
file = "tests/agent_api_conformance_tests.rs"
test = "test_agent_api_single_boundary_module"
scenario = "SC-51"

[[slice.tests]]
crate = "ettlex-agent-api"
file = "tests/agent_api_conformance_tests.rs"
test = "test_agent_api_no_why_what_how_in_logs"
scenario = "SC-52"

[[slice.tests]]
crate = "ettlex-agent-api"
file = "tests/agent_api_conformance_tests.rs"
test = "test_agent_api_no_apply_mcp_command"
scenario = "SC-53"
```

---

## 12. Acceptance Strategy

```
make test-slice    # All 53 new tests + all prior slice tests must pass
make lint          # Clean (no banned patterns, no clippy warnings, fmt clean)
make doc           # No new warnings
make coverage-check  # ≥ 80% (COVERAGE_MIN unchanged)
```

---

## 13. Plan Integrity Declaration

> No production code will be written before RED evidence exists.
> No code outside the declared slice boundary will be modified except the Makefile and handoff/slice_registry.toml (and any declared infrastructure exceptions).
> All replacement targets have been identified and their post-slice structural invariants declared.

---

*Plan written: 2026-03-24*
