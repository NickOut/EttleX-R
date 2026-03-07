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

Scenario: EpUpdate rejects empty update
When I apply Command::EpUpdate{ep_id="ep:store:0"} with no fields supplied
Then a typed error EmptyUpdate is returned

Scenario: EpUpdate on non-existent EP returns NotFound
When I apply Command::EpUpdate{ep_id="ep:missing", why="x"}
Then a typed error NotFound is returned

Scenario: EpUpdate increments state_version
Given the current state_version is N
When I apply Command::EpUpdate{ep_id="ep:store:0", why="new"}
Then the state_version is N+1

Scenario: EpUpdate sets updated_at
Given EP "ep:store:0" has updated_at "T1"
When I apply Command::EpUpdate{ep_id="ep:store:0", why="new"}
Then ep.get("ep:store:0").updated_at is greater than or equal to "T1"

Scenario: EpUpdate is reflected in next snapshot manifest
Given EP "ep:store:0" exists and a snapshot S1 has been committed
When I apply Command::EpUpdate{ep_id="ep:store:0", why="amended"}
And I commit snapshot S2 for the same leaf
Then snapshot.diff(S1, S2) shows ep_changes including "ep:store:0"
And the ep_digest for "ep:store:0" differs between S1 and S2

Scenario: eps schema migration adds title column if absent
Given the database was initialised before title was introduced
When migrations are applied
Then the eps table has a title column of type TEXT nullable
And existing EP rows have title as null
And EpUpdate can set title on those rows
