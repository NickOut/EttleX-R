# Ettle: Slice 01 — Ettle CRUD (Store → Engine/Action → MCP) v3

**Ettle ID:** ettle:019cf0f7-ae6b-7ea3-8b72-ba0b4052b9c3
**Status:** Generator-ready
**Layers:** Store, Engine/Action, MCP
**Prerequisite:** Slice 00 (infrastructure) MUST be complete

---

## WHY

The existing Ettle CRUD implementation was written against the original schema and domain model. It conflates EP structure with Ettle identity, uses field names that no longer exist (parent_id, ettle_id on EPs), and lacks WHY/WHAT/HOW fields on the Ettle record itself. The new conceptual model (v0.8) places WHY, WHAT, and HOW directly on the Ettle record as the primary reasoning content, introduces a Reasoning Link (reasoning_link_id, reasoning_link_type) for parent traversal, and uses tombstoned_at consistently across all governed records.

The vertical slice approach requires a full replacement of the Ettle CRUD path — Store, Engine/Action, and MCP — as a coherent unit. All layers must reflect the new model simultaneously. The existing implementations are replaced, not extended.

This slice has three concurrent objectives:

1. Implement the new Ettle record shape with WHY/WHAT/HOW and Reasoning Link fields
2. Migrate the architectural structure to match the layering constraint: business logic extracted from `dispatch_mcp_command` into dedicated engine handlers, MCP layer reduced to transport wiring only, store layer reduced to structural persistence only
3. Migrate the Ettle CRUD path from the legacy `EttleXError` to `ExError` (infrastructure from Slice 00)

**Prerequisite:** Slice 00 MUST be complete before this slice runs. This slice assumes `ettlex-errors` and `ettlex-logging` crates exist, `ExErrorKind` includes `AlreadyTombstoned`/`SelfReferentialLink`/`HasActiveDependants`/`MissingLinkType`, and `EttleXError` is retired.

Hard delete is deferred: the Profile system required to govern it is not part of this slice. Tombstone is the only deletion path in this slice.

EP CRUD is explicitly out of scope. EPs are a legacy construct being phased out; the new model stores reasoning content directly on the Ettle record.

---

## WHAT

### Scope

This slice implements full Ettle CRUD across Store, Engine/Action, and MCP layers:
- EttleCreate
- EttleGet
- EttleList (paginated, deterministic)
- EttleUpdate (title, why, what, how, reasoning_link_id, reasoning_link_type)
- EttleTombstone (with dependant check)

Out of scope: hard delete, EP CRUD, Relation CRUD, snapshot changes, Decision/Constraint/Group CRUD, CLI wiring, authentication and authorisation.

### Replacement targets

The following existing code is SUPERSEDED by this slice and MUST be replaced, not extended:

**`ettlex-mcp/src/tools/ettle.rs`**
- `handle_ettle_get` — superseded; new version returns full new field set including why, what, how, reasoning_link_id, reasoning_link_type, tombstoned_at
- `handle_ettle_list` — superseded; new version accepts include_tombstoned, uses new field ordering, returns tombstoned_at
- `handle_ettle_list_eps` — retained as-is (not in scope)
- `handle_ettle_list_decisions` — retained as-is (not in scope)

**`ettlex-engine/src/commands/mcp_command.rs` — EttleCreate and EttleUpdate dispatch**
- `McpCommand::EttleCreate` variant — superseded; new variant accepts why/what/how/reasoning_link_id/reasoning_link_type; must not accept ettle_id
- `McpCommand::EttleUpdate` variant — superseded; new variant accepts why/what/how/reasoning_link_id/reasoning_link_type; field-level patch semantics
- Dispatch arm for `EttleCreate` in `dispatch_mcp_command` — superseded; ALL invariant enforcement must move to a dedicated engine handler; dispatch arm becomes a thin delegation call only
- Dispatch arm for `EttleUpdate` in `dispatch_mcp_command` — superseded; same as above
- New variants to add: `McpCommand::EttleTombstone`, `McpCommand::EttleGet`, `McpCommand::EttleList`
- New result variants to add: `McpCommandResult::EttleTombstone`, `McpCommandResult::EttleUpdate` (was missing)

**`ettlex-store/src/repo/sqlite_repo.rs` — Ettle persistence**
- `persist_ettle` — superseded; must use new column set; must NOT write parent_id, parent_ep_id, deleted, metadata
- `persist_ettle_tx` — superseded; same as above
- `get_ettle` — superseded; must read new column set; must NOT read parent_id, parent_ep_id, deleted, metadata
- `list_ettles_paginated` — superseded; must use new ordering (created_at ASC, id ASC), support include_tombstoned, use tombstoned_at IS NULL filter, enforce limit min/max

### Post-slice structural invariants

The following MUST hold after this slice. Verified by the architectural conformance scenarios.

- **INV-1:** `dispatch_mcp_command` MUST contain no Ettle business logic. For EttleCreate, EttleUpdate, EttleTombstone, EttleGet, and EttleList, it MUST delegate entirely to engine handler functions. Existence checks, tombstone checks, invariant enforcement, and state_version increment MUST NOT appear in the dispatch match arm.
- **INV-2:** Dedicated engine handler functions MUST exist: `handle_ettle_create`, `handle_ettle_update`, `handle_ettle_tombstone`, `handle_ettle_get`, `handle_ettle_list`.
- **INV-3:** Store functions for Ettle operations MUST contain no domain rule validation. `insert_ettle`, `get_ettle`, `list_ettles`, `update_ettle`, `tombstone_ettle` MUST perform only SQL execution, FK existence checking via query, and rusqlite error conversion to ExError.
- **INV-4:** `ettlex-mcp` tool handlers for ettle.get and ettle.list MUST contain no invariant enforcement.
- **INV-5:** `ettlex-mcp` apply handler for EttleCreate, EttleUpdate, EttleTombstone MUST perform type/required-field validation only and delegate to engine command handlers.
- **INV-6:** state_version increment (INSERT INTO mcp_command_log) MUST be performed by `apply_mcp_command` in `ettlex-engine`, not by any store function or MCP handler.
- **INV-7:** Provenance event append (INSERT INTO provenance_events) MUST be performed by the engine action layer, not by any store function or MCP handler.
- **INV-8:** All new and migrated code in this slice MUST return `Result<T, ExError>` from `ettlex-errors`. No `EttleXError` references MUST appear in any file touched by this slice.

### New Ettle Record Shape

The Ettle record at the store layer:
- id (TEXT PRIMARY KEY, ettle: prefix, ULIDv7)
- title (TEXT NOT NULL)
- why (TEXT NOT NULL DEFAULT '')
- what (TEXT NOT NULL DEFAULT '')
- how (TEXT NOT NULL DEFAULT '')
- reasoning_link_id (TEXT NULL, FK → ettles.id)
- reasoning_link_type (TEXT NULL)
- created_at (TEXT NOT NULL, ISO-8601)
- updated_at (TEXT NOT NULL, ISO-8601)
- tombstoned_at (TEXT NULL, ISO-8601)

Columns removed: parent_id, parent_ep_id, deleted, metadata

### Migration

A new numbered migration (next after 011) performs:
- ADD COLUMN why TEXT NOT NULL DEFAULT ''
- ADD COLUMN what TEXT NOT NULL DEFAULT ''
- ADD COLUMN how TEXT NOT NULL DEFAULT ''
- ADD COLUMN reasoning_link_id TEXT NULL REFERENCES ettles(id)
- ADD COLUMN reasoning_link_type TEXT NULL
- ADD COLUMN tombstoned_at TEXT NULL
- DROP COLUMN deleted
- DROP COLUMN parent_id
- DROP COLUMN parent_ep_id
- DROP COLUMN metadata
- DROP INDEX idx_ettles_parent_id (if exists)
- CREATE INDEX idx_ettles_reasoning_link ON ettles(reasoning_link_id)
- CREATE INDEX idx_ettles_tombstoned ON ettles(tombstoned_at)

Migration MUST be additive for surviving data. All existing rows receive empty strings for why/what/how and NULL for reasoning_link_id, reasoning_link_type, tombstoned_at.

### Command Vocabulary (Action Layer)

#### EttleCreate
Input: title (required, non-empty after trimming), why (optional, default ''), what (optional, default ''), how (optional, default ''), reasoning_link_id (optional, nullable), reasoning_link_type (optional, nullable, required if reasoning_link_id is set)
Output: { ettle_id }
Invariants (all enforced in engine, not MCP or store):
- title must be non-empty after trimming whitespace
- ettle_id is auto-generated (ULIDv7 prefixed ettle:); caller MUST NOT supply ettle_id
- If reasoning_link_id is supplied, referenced Ettle must exist and must not be tombstoned
- If reasoning_link_id is supplied, reasoning_link_type must also be supplied
- reasoning_link_type must be a non-empty string when supplied
- state_version increments by 1
- Provenance event ettle_created appended

#### EttleGet
Input: ettle_id
Output: { id, title, why, what, how, reasoning_link_id, reasoning_link_type, created_at, updated_at, tombstoned_at }
Invariants:
- Returns NotFound if ettle_id does not exist
- Returns the record including tombstoned records (caller sees tombstoned_at)
- No state_version increment (read-only)
- Repeated calls with identical state return byte-identical JSON output

#### EttleList
Input: limit (optional, default 100, min 1, max 1000), cursor (optional, opaque base64), include_tombstoned (optional bool, default false)
Output: { items: [{ id, title, tombstoned_at }], cursor? }
Invariants:
- Ordering: (created_at ASC, id ASC) — deterministic
- Excludes tombstoned records by default; include_tombstoned=true includes them
- Cursor is opaque base64-encoded (created_at, id) pair
- No state_version increment (read-only)
- Repeated calls with identical parameters and identical state return byte-identical JSON output
- limit=1 is valid; limit=1000 is the maximum; limit=0 and limit≥1001 are rejected with InvalidInput

#### EttleUpdate
Input: ettle_id (required), title (optional), why (optional), what (optional), how (optional), reasoning_link_id (optional, null clears the link), reasoning_link_type (optional, null clears the type)
Output: {}
Invariants (all enforced in engine):
- At least one field must be supplied
- ettle_id must exist and must not be tombstoned
- If reasoning_link_id is set to a non-null value, referenced Ettle must exist and not be tombstoned
- An Ettle MUST NOT be set as its own reasoning_link_id (SelfReferentialLink)
- If reasoning_link_id is cleared (null), reasoning_link_type is also cleared
- If reasoning_link_id is set to a non-null value, reasoning_link_type must be supplied and non-empty
- Unspecified optional fields are preserved unchanged
- state_version increments by 1
- Provenance event ettle_updated appended

#### EttleTombstone
Input: ettle_id (required)
Output: {}
Invariants (all enforced in engine):
- ettle_id must exist and must not already be tombstoned (AlreadyTombstoned)
- Ettle must have no active (non-tombstoned) dependants: no other Ettle has reasoning_link_id = this ettle_id where tombstoned_at IS NULL
- Row is retained; tombstoned_at is set to current UTC timestamp
- state_version increments by 1
- Provenance event ettle_tombstoned appended

### Error Kinds Used

All from ExErrorKind in `ettlex-errors` (all available after Slice 00):
- `InvalidInput` — missing required fields, invalid limit values, caller-supplied ettle_id, no fields in update
- `InvalidTitle` — title is empty or whitespace-only
- `NotFound` — ettle_id does not exist
- `AlreadyTombstoned` — operation on a tombstoned Ettle; or referencing a tombstoned Ettle as reasoning_link_id
- `SelfReferentialLink` — reasoning_link_id set to own ettle_id
- `HasActiveDependants` — EttleTombstone blocked by active child Ettles
- `MissingLinkType` — reasoning_link_id supplied without reasoning_link_type
- `HeadMismatch` — expected_state_version does not match current state_version

### Concurrency Model

SQLite with rusqlite (bundled) serialises all write operations at the database level. The OCC mechanism via expected_state_version provides an additional application-level guard. No additional concurrency primitives are required in this slice.

### Logging (boundary ownership)

The MCP entry point owns the lifecycle log events for each Ettle operation:
- `log_op_start!` at the top of each tool handler
- `log_op_end!` on success
- `log_op_error!` on any ExError return

Engine handler functions MUST NOT emit lifecycle start/end events. They MAY emit DEBUG/TRACE events for internal steps.

---

## HOW

### Scenarios (all MUST be implemented as tests; Gherkin is normative)

#### Feature: EttleCreate — happy path

```gherkin
Scenario: EttleCreate with title only succeeds
  Given an empty store
  When I apply EttleCreate { title: "My Ettle" }
  Then a new ettle_id is returned with prefix "ettle:"
  And state_version increments by 1
  And ettle.get(ettle_id) returns title="My Ettle", why="", what="", how=""
  And reasoning_link_id is null, reasoning_link_type is null
  And tombstoned_at is null
  And a provenance event ettle_created is recorded

Scenario: EttleCreate with WHY/WHAT/HOW fields
  When I apply EttleCreate { title: "T", why: "W", what: "X", how: "H" }
  Then ettle.get returns why="W", what="X", how="H"

Scenario: EttleCreate with reasoning_link_id
  Given Ettle P exists and is not tombstoned
  When I apply EttleCreate { title: "Child", reasoning_link_id: P, reasoning_link_type: "refinement" }
  Then ettle.get(new_id).reasoning_link_id = P
  And ettle.get(new_id).reasoning_link_type = "refinement"
```

#### Feature: EttleCreate — negative and error paths

```gherkin
Scenario: EttleCreate rejects empty title
  When I apply EttleCreate { title: "" }
  Then error kind InvalidTitle is returned
  And state_version is unchanged
  And no provenance event is appended

Scenario: EttleCreate rejects whitespace-only title
  When I apply EttleCreate { title: "   " }
  Then error kind InvalidTitle is returned

Scenario: EttleCreate rejects caller-supplied ettle_id
  When I apply EttleCreate { title: "T", ettle_id: "ettle:manual" }
  Then error kind InvalidInput is returned

Scenario: EttleCreate rejects reasoning_link_id without reasoning_link_type
  Given Ettle P exists
  When I apply EttleCreate { title: "T", reasoning_link_id: P }
  Then error kind MissingLinkType is returned

Scenario: EttleCreate rejects reasoning_link_id pointing to non-existent Ettle
  When I apply EttleCreate { title: "T", reasoning_link_id: "ettle:missing", reasoning_link_type: "refinement" }
  Then error kind NotFound is returned

Scenario: EttleCreate rejects reasoning_link_id pointing to tombstoned Ettle
  Given Ettle P is tombstoned
  When I apply EttleCreate { title: "T", reasoning_link_id: P, reasoning_link_type: "refinement" }
  Then error kind AlreadyTombstoned is returned
```

#### Feature: EttleCreate — idempotency and uniqueness

```gherkin
Scenario: Two identical EttleCreate calls produce two distinct Ettles
  When I apply EttleCreate { title: "Duplicate" }
  And I apply EttleCreate { title: "Duplicate" }
  Then two distinct ettle_ids are returned
  And both Ettles exist independently in the store
  And state_version has incremented by 2
```

#### Feature: EttleGet

```gherkin
Scenario: EttleGet returns full record
  Given Ettle E exists with why="W", what="X", how="H"
  When I call ettle.get(E)
  Then response contains id, title, why="W", what="X", how="H"
  And reasoning_link_id is null, tombstoned_at is null

Scenario: EttleGet returns tombstoned record with tombstoned_at set
  Given Ettle E is tombstoned
  When I call ettle.get(E)
  Then response contains tombstoned_at as non-null timestamp

Scenario: EttleGet returns NotFound for unknown id
  When I call ettle.get("ettle:unknown")
  Then error kind NotFound is returned

Scenario: EttleGet is deterministic across repeated calls
  Given Ettle E exists with all fields populated
  When I call ettle.get(E) twice in succession without modifying state
  Then both responses are byte-identical JSON
  And field ordering in the JSON is stable
```

#### Feature: EttleList — happy path and ordering

```gherkin
Scenario: EttleList returns active Ettles in deterministic order
  Given Ettles A and B exist (A created before B)
  When I call ettle.list()
  Then response items are [A, B] in created_at ASC order
  And tombstoned Ettles are excluded

Scenario: EttleList with include_tombstoned returns all
  Given Ettle A is active, Ettle B is tombstoned
  When I call ettle.list(include_tombstoned: true)
  Then response contains both A and B

Scenario: EttleList is deterministic across repeated calls
  Given 5 Ettles exist
  When I call ettle.list() twice without modifying state
  Then both responses are byte-identical JSON
```

#### Feature: EttleList — pagination

```gherkin
Scenario: EttleList pagination is deterministic
  Given 10 Ettles exist
  When I call ettle.list(limit: 5)
  Then 5 items are returned and a cursor is present
  When I call ettle.list(limit: 5, cursor: <cursor>)
  Then the next 5 items are returned with no cursor
  And no item appears in both pages

Scenario: EttleList last page returns no cursor
  Given 3 Ettles exist
  When I call ettle.list(limit: 5)
  Then 3 items are returned and no cursor is present

Scenario: EttleList with limit=1 returns single item
  Given at least 2 Ettles exist
  When I call ettle.list(limit: 1)
  Then exactly 1 item is returned and a cursor is present

Scenario: EttleList with limit=1000 succeeds
  Given 1000 Ettles exist
  When I call ettle.list(limit: 1000)
  Then 1000 items are returned and the call completes within the configured time budget
```

#### Feature: EttleList — boundary and negative cases

```gherkin
Scenario: EttleList with limit=0 is rejected
  When I call ettle.list(limit: 0)
  Then error kind InvalidInput is returned

Scenario: EttleList with limit exceeding maximum is rejected
  When I call ettle.list(limit: 1001)
  Then error kind InvalidInput is returned

Scenario: EttleList with invalid cursor returns error
  When I call ettle.list(cursor: "not-valid-base64!!")
  Then error kind InvalidInput is returned
```

#### Feature: EttleUpdate — happy path

```gherkin
Scenario: EttleUpdate changes title
  Given Ettle E with title "Old"
  When I apply EttleUpdate { ettle_id: E, title: "New" }
  Then ettle.get(E).title = "New"
  And state_version increments by 1
  And provenance event ettle_updated is recorded

Scenario: EttleUpdate changes WHY/WHAT/HOW
  Given Ettle E
  When I apply EttleUpdate { ettle_id: E, why: "W2", what: "X2", how: "H2" }
  Then ettle.get(E) returns why="W2", what="X2", how="H2"

Scenario: EttleUpdate sets reasoning_link_id
  Given Ettle E and Ettle P both exist
  When I apply EttleUpdate { ettle_id: E, reasoning_link_id: P, reasoning_link_type: "option" }
  Then ettle.get(E).reasoning_link_id = P
  And ettle.get(E).reasoning_link_type = "option"

Scenario: EttleUpdate clears reasoning_link_id
  Given Ettle E has reasoning_link_id = P
  When I apply EttleUpdate { ettle_id: E, reasoning_link_id: null }
  Then ettle.get(E).reasoning_link_id is null
  And ettle.get(E).reasoning_link_type is null

Scenario: EttleUpdate preserves unspecified fields
  Given Ettle E with title="T", why="W", what="X", how="H"
  When I apply EttleUpdate { ettle_id: E, title: "T2" }
  Then ettle.get(E).why = "W"
  And ettle.get(E).what = "X"
  And ettle.get(E).how = "H"
```

#### Feature: EttleUpdate — negative and error paths

```gherkin
Scenario: EttleUpdate rejects self-referential reasoning_link_id
  Given Ettle E
  When I apply EttleUpdate { ettle_id: E, reasoning_link_id: E, reasoning_link_type: "refinement" }
  Then error kind SelfReferentialLink is returned
  And ettle.get(E).reasoning_link_id is unchanged

Scenario: EttleUpdate rejects reasoning_link_id without reasoning_link_type
  Given Ettle E and Ettle P
  When I apply EttleUpdate { ettle_id: E, reasoning_link_id: P }
  Then error kind MissingLinkType is returned

Scenario: EttleUpdate rejects update to non-existent Ettle
  When I apply EttleUpdate { ettle_id: "ettle:missing", title: "X" }
  Then error kind NotFound is returned

Scenario: EttleUpdate rejects update to tombstoned Ettle
  Given Ettle E is tombstoned
  When I apply EttleUpdate { ettle_id: E, title: "X" }
  Then error kind AlreadyTombstoned is returned

Scenario: EttleUpdate with no fields supplied is rejected
  Given Ettle E
  When I apply EttleUpdate { ettle_id: E } with no optional fields
  Then error kind InvalidInput is returned
```

#### Feature: EttleTombstone — happy path and state transitions

```gherkin
Scenario: EttleTombstone marks Ettle inactive; row retained
  Given Ettle E with no active dependants
  When I apply EttleTombstone { ettle_id: E }
  Then ettle.get(E).tombstoned_at is non-null
  And the row still exists in the database
  And state_version increments by 1
  And provenance event ettle_tombstoned is recorded

Scenario: Tombstoned Ettle cannot be updated
  Given Ettle E is tombstoned
  When I apply EttleUpdate { ettle_id: E, title: "X" }
  Then error kind AlreadyTombstoned is returned

Scenario: Tombstoned Ettle cannot be tombstoned again
  Given Ettle E is tombstoned
  When I apply EttleTombstone { ettle_id: E }
  Then error kind AlreadyTombstoned is returned

Scenario: EttleTombstone allows tombstone when dependant is itself tombstoned
  Given Ettle P, Ettle C with reasoning_link_id = P and C is tombstoned
  When I apply EttleTombstone { ettle_id: P }
  Then the command succeeds
  And ettle.get(P).tombstoned_at is non-null
```

#### Feature: EttleTombstone — negative cases

```gherkin
Scenario: EttleTombstone rejects Ettle with active dependants
  Given Ettle P, Ettle C with reasoning_link_id = P and C is not tombstoned
  When I apply EttleTombstone { ettle_id: P }
  Then error kind HasActiveDependants is returned
  And ettle.get(P).tombstoned_at is null

Scenario: EttleTombstone rejects non-existent Ettle
  When I apply EttleTombstone { ettle_id: "ettle:missing" }
  Then error kind NotFound is returned
```

#### Feature: Hard delete prohibition

```gherkin
Scenario: Hard delete is not exposed via MCP in this slice
  When I attempt to invoke any hard delete operation via ettlex.apply
  Then the command is not recognised
  And no row is deleted from the database
```

#### Feature: OCC (Optimistic Concurrency)

```gherkin
Scenario: Command with correct expected_state_version succeeds
  Given state_version is V
  When I apply any write command with expected_state_version = V
  Then the command succeeds and new_state_version = V + 1

Scenario: Command with wrong expected_state_version fails
  Given state_version is V
  When I apply any write command with expected_state_version = V - 1
  Then error kind HeadMismatch is returned
  And state_version remains V
  And no provenance event is appended
```

#### Feature: Provenance / observability

```gherkin
Scenario: Each successful mutation appends exactly one provenance event
  When I apply any of EttleCreate, EttleUpdate, EttleTombstone successfully
  Then exactly one provenance_event row is appended
  And the event kind matches the command (ettle_created, ettle_updated, ettle_tombstoned)

Scenario: Failed commands append no provenance events
  When I apply a command that returns any typed error
  Then no provenance_event row is appended
  And the total provenance_event count is unchanged
```

#### Feature: Deterministic byte-level output

```gherkin
Scenario: EttleGet output is byte-identical across repeated calls
  Given Ettle E exists with all fields populated
  When I call ettle.get(E) twice without modifying state
  Then the JSON responses are byte-identical
  And field ordering in the JSON is stable

Scenario: EttleList output is byte-identical across repeated calls
  Given 5 Ettles exist
  When I call ettle.list() twice without modifying state
  Then the JSON responses are byte-identical
  And item ordering is stable
```

#### Feature: Resource limits

```gherkin
Scenario: EttleCreate with large WHY/WHAT/HOW fields succeeds
  When I apply EttleCreate { title: "T", why: <100KB string>, what: <100KB string>, how: <100KB string> }
  Then the command succeeds within the configured time budget
  And ettle.get returns all three fields fully and correctly

Scenario: EttleList at maximum limit succeeds within time budget
  Given 1000 Ettles exist
  When I call ettle.list(limit: 1000)
  Then 1000 items are returned and the call completes within the configured time budget
```

#### Feature: Migration

```gherkin
Scenario: Migration applies cleanly on top of existing migrations
  Given a database with all prior migrations applied
  When the new migration is applied
  Then no error occurs
  And the ettles table has columns why, what, how, reasoning_link_id, reasoning_link_type, tombstoned_at
  And the deleted, parent_id, parent_ep_id, and metadata columns are absent
  And existing rows have why='', what='', how='', reasoning_link_id=NULL, reasoning_link_type=NULL, tombstoned_at=NULL

Scenario: Pre-existing Ettle rows survive migration with values preserved
  Given Ettle rows exist before the migration with id and title populated
  When the migration is applied
  Then all pre-existing rows are present
  And their id and title values are unchanged
```

#### Feature: Architectural conformance (INV-1 through INV-8)

```gherkin
Scenario: dispatch_mcp_command contains no Ettle business logic (INV-1)
  When I inspect the EttleCreate, EttleUpdate, EttleTombstone match arms in dispatch_mcp_command
  Then each arm contains exactly one function call to a dedicated engine handler
  And no existence checks, tombstone checks, or field validation logic appear in the arm body

Scenario: Dedicated engine handler functions exist for all five Ettle operations (INV-2)
  When I inspect ettlex-engine/src/
  Then handle_ettle_create is defined
  And handle_ettle_update is defined
  And handle_ettle_tombstone is defined
  And handle_ettle_get is defined
  And handle_ettle_list is defined

Scenario: Store Ettle functions contain no domain rule validation (INV-3)
  When I inspect insert_ettle, get_ettle, list_ettles, update_ettle, tombstone_ettle in sqlite_repo.rs
  Then no existence checks appear (no "SELECT COUNT" or "get_ettle" calls inside these functions)
  And no tombstoned_at IS NULL checks appear in update or tombstone functions
  And no dependant checks appear in tombstone function
  And each function performs only SQL execution and error conversion

Scenario: state_version increment is owned by apply_mcp_command (INV-6)
  When I search for INSERT INTO mcp_command_log in the workspace
  Then it appears only in apply_mcp_command in ettlex-engine
  And not in any store function or MCP tool handler

Scenario: Provenance event append is owned by the engine action layer (INV-7)
  When I search for INSERT INTO provenance_events in the workspace
  Then it appears only in engine action layer functions
  And not in any store function or MCP tool handler

Scenario: No EttleXError references in files touched by this slice (INV-8)
  When I inspect all files modified by this slice
  Then no import of EttleXError is present
  And no use of EttleXError is present
  And all fallible public functions return Result<T, ExError>

Scenario: MCP ettle.get handler contains no invariant enforcement (INV-4)
  When I inspect handle_ettle_get in ettlex-mcp/src/tools/ettle.rs
  Then the function contains only: parameter deserialisation, a call to the engine query handler, and response serialisation
  And no existence checks, tombstone logic, or business rule validation appear
```

### MCP Response Shapes

#### ettle.get response
```json
{
  "id": "ettle:...",
  "title": "...",
  "why": "...",
  "what": "...",
  "how": "...",
  "reasoning_link_id": null,
  "reasoning_link_type": null,
  "created_at": "...",
  "updated_at": "...",
  "tombstoned_at": null
}
```

#### ettle.list response
```json
{
  "items": [
    { "id": "ettle:...", "title": "...", "tombstoned_at": null }
  ],
  "cursor": "<opaque base64 or absent>"
}
```

#### ettlex.apply EttleCreate response
```json
{ "new_state_version": 1, "result": { "tag": "EttleCreate", "ettle_id": "ettle:..." } }
```

#### ettlex.apply EttleUpdate response
```json
{ "new_state_version": 2, "result": { "tag": "EttleUpdate" } }
```

#### ettlex.apply EttleTombstone response
```json
{ "new_state_version": 3, "result": { "tag": "EttleTombstone" } }
```

### Expected Failure Registry (pre-authorised)

**EFR-01:** All tests in `ettlex-engine/tests/` that reference old Ettle field names (parent_id, deleted boolean) or old command shapes without WHY/WHAT/HOW.

**EFR-02:** All tests in `ettlex-store/tests/` that assert on the old ettles table schema (absence of why/what/how columns, presence of deleted column).

**EFR-03:** MCP integration tests in `ettlex-mcp/tests/` that assert ettle.get or ettle.list response shapes without the new fields.

**EFR-04:** Any test that calls `dispatch_mcp_command`'s EttleCreate or EttleUpdate arms directly and asserts on the old inline business logic structure.

The code generator MUST enumerate each specific test function in the Pre-Authorised Failure Registry section of the completion report.

---

## Scenario Coverage Reference

| Category | Status | Notes |
|---|---|---|
| 1. Happy path | Covered | All five operations have positive scenarios |
| 2. Negative cases | Covered | Invalid inputs, missing fields, wrong state across all operations |
| 3. Explicit error paths | Covered | All 8 ExErrorKind variants exercised |
| 4. Boundary conditions | Covered | limit=0, limit=1, limit=1000, limit=1001; last page no cursor; whitespace-only title |
| 5. Invariants | Covered | Auto-generated ID; title non-empty; link type required with link id; self-referential rejection; row retained after tombstone |
| 6. Idempotency | Covered | Two identical EttleCreate calls produce two distinct Ettles |
| 7. Determinism / ordering | Covered | EttleList ordering specified; byte-identical output scenarios |
| 8. State transitions | Covered | Active → Tombstoned; tombstoned cannot be updated or re-tombstoned |
| 9. Concurrency | Covered | OCC HeadMismatch; SQLite serialisation noted in WHAT |
| 10. Security / authorisation | N/A | Deferred; noted as out of scope |
| 11. Observability | Covered | Provenance event per mutation; no event on failure; logging boundary ownership specified |
| 12. Compatibility / migration | Covered | Migration applies cleanly; existing rows survive; removed columns verified absent |
| 13. Resource / performance | Covered | 100KB field round-trip; 1000-item list within time budget |
| 14. Explicit prohibitions | Covered | Hard delete not exposed; no row deleted scenario |
| 15. Byte-level determinism | Covered | EttleGet and EttleList byte-identical JSON across repeated calls |
| 16. Concurrency conflict / OCC | Covered | Correct and incorrect expected_state_version scenarios |
| 17. Architectural conformance | Covered | INV-1 through INV-8 each have explicit scenarios |
