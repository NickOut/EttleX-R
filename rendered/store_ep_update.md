# Leaf Bundle: EttleX Product > Storage Spine (SQLite + CAS + Seed Import)

## WHY (Rationale)

1. Establish the product-level framing for EttleX as a semantic evolution engine.
   This EP is non-load-bearing for the storage/commit milestones; it exists to keep the tree rooted in a
   recognisable product-level node for rendering and navigation.

2. Provide a durable substrate so semantic state can be anchored, reproduced, and evolved safely.

3. EP at ordinal 6 of ettle:store contains two errors introduced before EpUpdate was available to correct them:

1) WHAT success criterion 7 incorrectly references MCP transport surface.
2) HOW contains a scenario (MCP tool ep.update delegates to EpUpdate command) that belongs in ettle:mcp_thin_slice, not ettle:store.
   This EP supersedes ordinal 6. Ordinal 6 should be treated as non-normative once EpUpdate is available to mark it as such.

## WHAT (Description)

1. Maintain a minimal product framing so leaf Ettles can be rendered in context.
   This EP does not impose implementation requirements on Phase 1/2 milestones.

2. Establish the platform foundations as refined milestones under this EP:

- Storage spine (SQLite + CAS + seed import)
- Snapshot commit pipeline (manifest + ledger anchor)

3. Implement EpUpdate as a canonical Apply command that mutates an existing EP's content fields in place.

Success criteria (binding):

1. EpUpdate accepts ep_id and any combination of: title, why, what, how.
2. At least one field must be supplied; empty update is rejected.
3. All supplied fields replace their current values; omitted fields are preserved unchanged.
4. state_version increments on successful EpUpdate.
5. updated_at is set on the eps row.
6. EpUpdate on a non-existent ep_id returns typed error NotFound.

Out of scope for this EP:

- EttleUpdate (separate concern; Ettles have minimal mutable fields at this stage)
- EpTombstone
- MCP transport surface (belongs in ettle:mcp_thin_slice)

Binding schema rule:

- eps table MUST have a title TEXT field.
- If not present, an additive migration MUST add it (nullable initially to accommodate existing rows).
- EpUpdate MAY be used to populate title on existing rows.
- EpCreate SHOULD accept title as an optional field from this point forward.

## HOW (Implementation)

1. No scenarios. This EP is informational only.

2. Refinement only. Implementation scenarios live in child Ettles.

3. Scenarios (all MUST be implemented as tests; Gherkin is normative):

Feature: EpUpdate mutates EP content fields in canonical state

Background:
Given a repository with SQLite + CAS store initialised
And at least one Ettle with one EP exists

# --- Happy path ---

Scenario: EpUpdate replaces why field only
Given EP "ep:store:0" exists with why "original why text"
When I apply Command::EpUpdate{ep_id="ep:store:0", why="updated why text"}
Then ep.get("ep:store:0").why equals "updated why text"
And ep.get("ep:store:0").what is unchanged
And ep.get("ep:store:0").how is unchanged

Scenario: EpUpdate replaces all content fields
Given EP "ep:store:0" exists
When I apply Command::EpUpdate{ep_id="ep:store:0", why="w", what="x", how="y"}
Then ep.get("ep:store:0").why equals "w"
And ep.get("ep:store:0").what equals "x"
And ep.get("ep:store:0").how equals "y"

Scenario: EpUpdate sets title on an existing EP
Given EP "ep:store:0" exists with no title
When I apply Command::EpUpdate{ep_id="ep:store:0", title="Storage Spine Anchor"}
Then ep.get("ep:store:0").title equals "Storage Spine Anchor"
And why/what/how are unchanged

# --- Negative cases ---

Scenario: EpUpdate rejects empty update
When I apply Command::EpUpdate{ep_id="ep:store:0"} with no fields supplied
Then a typed error EmptyUpdate is returned
And state_version is unchanged

Scenario: EpUpdate on non-existent EP returns NotFound
When I apply Command::EpUpdate{ep_id="ep:missing", why="x"}
Then a typed error NotFound is returned
And state_version is unchanged

# --- Explicit error paths ---

Scenario: EpUpdate with all fields null is rejected as EmptyUpdate
When I apply Command::EpUpdate{ep_id="ep:store:0", why=null, what=null, how=null, title=null}
Then a typed error EmptyUpdate is returned
And the EP is unchanged

# --- Boundary conditions ---

Scenario: EpUpdate with large content fields succeeds
When I apply Command::EpUpdate{ep_id="ep:store:0", how=<50KB Gherkin text>}
Then ep.get("ep:store:0").how equals the supplied text
And state_version increments

# --- Invariants ---

Scenario: EpUpdate increments state_version
Given the current state_version is N
When I apply Command::EpUpdate{ep_id="ep:store:0", why="new"}
Then the state_version is N+1

Scenario: EpUpdate sets updated_at
Given EP "ep:store:0" has updated_at "T1"
When I apply Command::EpUpdate{ep_id="ep:store:0", why="new"}
Then ep.get("ep:store:0").updated_at is greater than or equal to "T1"

Scenario: EpUpdate preserves omitted fields exactly
Given EP "ep:store:0" exists with why="original", what="w", how="h", title="t"
When I apply Command::EpUpdate{ep_id="ep:store:0", why="updated"}
Then ep.get("ep:store:0").what equals "w"
And ep.get("ep:store:0").how equals "h"
And ep.get("ep:store:0").title equals "t"

Scenario: EpUpdate does not change the EP's ordinal, ettle_id, or child_ettle_id
Given EP "ep:store:0" has ordinal 0, ettle_id "ettle:store", child_ettle_id null
When I apply Command::EpUpdate{ep_id="ep:store:0", why="new"}
Then ep.get("ep:store:0").ordinal equals 0
And ep.get("ep:store:0").ettle_id equals "ettle:store"
And ep.get("ep:store:0").child_ettle_id is unchanged

# --- Idempotency ---

Scenario: EpUpdate is NOT idempotent by design — repeated identical updates each increment state_version
Given state_version is V
When I apply Command::EpUpdate{ep_id="ep:store:0", why="same"} twice
Then state_version is V+2
And ep.get("ep:store:0").why equals "same" after both calls

# --- Determinism ---

Scenario: EpUpdate result is deterministic — same inputs produce same stored state
When I apply Command::EpUpdate{ep_id="ep:store:0", why="deterministic"}
Then ep.get("ep:store:0").why equals "deterministic"
And the stored bytes are identical to a second application with the same input

# --- State transitions ---

Scenario: EpUpdate is reflected in next snapshot manifest
Given EP "ep:store:0" exists and a snapshot S1 has been committed
When I apply Command::EpUpdate{ep_id="ep:store:0", why="amended"}
And I commit snapshot S2 for the same leaf
Then snapshot.diff(S1, S2) shows ep_changes including "ep:store:0"
And the ep_digest for "ep:store:0" differs between S1 and S2

Scenario: EpUpdate between two snapshots is visible in diff
Given snapshot S1 was committed when EP "ep:store:0" had why="before"
When I apply Command::EpUpdate{ep_id="ep:store:0", why="after"}
And I commit snapshot S2
Then snapshot.diff(S1, S2).ep_changes includes a record for "ep:store:0"
And the a_digest and b_digest differ

# --- Concurrency ---

Scenario: Concurrent EpUpdate calls on the same EP with expected_state_version produce exactly one success
Given state_version is V
And two concurrent callers A and B both have expected_state_version=V
When both call Command::EpUpdate{ep_id="ep:store:0", why="A"} and {why="B"} simultaneously
Then exactly one succeeds and state_version becomes V+1
And the other returns HeadMismatch
And ep.get("ep:store:0").why is the value from the successful caller

Scenario: Sequential EpUpdate calls without expected_state_version both succeed
Given state_version is V
When I apply Command::EpUpdate{ep_id="ep:store:0", why="first"}
And I apply Command::EpUpdate{ep_id="ep:store:0", why="second"}
Then state_version is V+2
And ep.get("ep:store:0").why equals "second"

# --- Security / authorisation ---

# Auth enforcement is governed by ep:mcp_thin_slice. No command-level auth for EpUpdate.

# --- Observability ---

Scenario: EpUpdate success is reflected in state.get_version
Given state_version is V
When I apply Command::EpUpdate{ep_id="ep:store:0", why="obs"}
Then state.get_version() returns V+1

# --- Compatibility / migration ---

Scenario: eps schema migration adds title column if absent
Given the database was initialised before title was introduced
When migrations are applied
Then the eps table has a title column of type TEXT nullable
And existing EP rows have title as null
And EpUpdate can set title on those rows

Scenario: EpUpdate on EPs created before title field was introduced succeeds
Given EP "ep:store:0" was created before the title column existed and has title null
When I apply Command::EpUpdate{ep_id="ep:store:0", title="Backfilled Title"}
Then ep.get("ep:store:0").title equals "Backfilled Title"

# --- Resource / performance ---

# No specific performance obligations defined for EpUpdate.

# --- Explicit prohibition ---

Scenario: EpUpdate MUST NOT create a new EP
Given EP "ep:store:0" exists
When I apply Command::EpUpdate{ep_id="ep:store:0", why="new"}
Then ettle.list_eps("ettle:store") returns the same number of EPs as before
And no new ep_id is generated

Scenario: EpUpdate MUST NOT silently discard supplied fields
When I apply Command::EpUpdate{ep_id="ep:store:0", why="explicit"}
Then ep.get("ep:store:0").why equals "explicit"
And the implementation does not apply a default value in place of the supplied value

# --- Byte-level equivalence ---

Scenario: ep.get after EpUpdate returns byte-identical results for identical canonical state
When I apply Command::EpUpdate{ep_id="ep:store:0", why="stable"}
And I call ep.get("ep:store:0") twice without intervening mutations
Then both responses are byte-identical after canonical JSON serialization

# --- Concurrency conflict ---

# Covered under Concurrency above via expected_state_version HeadMismatch scenario.
