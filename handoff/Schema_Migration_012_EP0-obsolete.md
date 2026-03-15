# Schema Migration 012 — Ettle/EP Structural Model Update with Full CRUD Exposure

**Ettle ID:** `ettle:019ccf15-e2b1-7e33-9794-bf2cf0704178`
**EP:** 0
**Normative:** Yes

---

## WHY

The current Ettle/EP schema uses field names and link structures that are semantically incorrect and operationally obtuse:

  - `eps.ettle_id` is an opaque foreign key with no semantic clarity about the direction of membership.
  - `eps.child_ettle_id` encodes a refinement link from an EP to a child Ettle, but the correct model is
    EP-to-EP refinement across Ettle boundaries, not EP-to-Ettle.
  - `ettles.parent_id` duplicates information derivable from the EP tree and conflates two independent
    link trees: the Ettle abstraction-level hierarchy and the EP refinement trace.
  - `ettles.parent_ep_id` (migration 009) partially corrected this but conflated the two trees further.
  - The MCP and CLI command vocabulary (`EttleCreate`, `EpCreate`, `EpUpdate`) do not expose the structural
    link fields at all, forcing seed import as the only path to establish refinement relationships.
    This is the root cause of the tree corruption discovered in prior sessions.
  - `eps.content_digest` and `eps.content_inline` are a dual-storage pattern that is unnecessary;
    WHY/WHAT/HOW are already surfaced as JSON in the `content` field at the MCP layer.

This Ettle corrects the structural model and exposes full CRUD for both Ettles and EPs — including all
link fields — through the command vocabulary (Apply) and query surfaces, so the MCP and CLI become the
canonical authoring path and seed import is no longer required for structural operations.

Non-goals (explicitly out of scope for this Ettle):
  - EPT computation changes (ept.compute, ept_digest, snapshot commit traversal).
  - Snapshot manifest changes.
  - Constraint, decision, profile, approval, or policy entity changes.
  - DAG support for EP or Ettle parent links (deferred; strict trees only).
  - CLI UX beyond thin command wiring.
  - Seed importer changes or retirement.
  - Any existing test modification beyond #[ignore]-tagging of pre-authorised failures
    (see WHAT section: Expected Failure Registry).

---

## WHAT

### 1. Schema changes — single migration file (012)

The project uses rusqlite 0.29 with the `bundled` feature, which compiles SQLite 3.43.x.
This version fully supports ALTER TABLE RENAME COLUMN (since 3.25.0) and DROP COLUMN
(since 3.35.0). A clean single migration is therefore correct and preferred.

#### 1.1 Migration file

File: `migrations/012_structural_model_update.sql`

This migration MUST:
  - Rename `eps.ettle_id` to `eps.containing_ettle`.
  - Rename `ettles.parent_id` to `ettles.parent_ettle_id`.
  - Drop `ettles.parent_ep_id` (added in migration 009; conflated the two link trees).
  - Drop `eps.child_ettle_id` (replaced by `eps.parent_ep_id` in the new model).
  - Drop `eps.content_digest` and `eps.content_inline` (content is carried in the
    existing `content` JSON field; dual-storage pattern is removed).
  - Add `eps.parent_ep_id TEXT REFERENCES eps(id)` (nullable; new cross-Ettle link).
  - Drop index `idx_eps_ettle_id` (references old column name).
  - Drop index `idx_ettles_parent_id` (references old column name).
  - Create index `idx_eps_containing_ettle ON eps(containing_ettle)`.
  - Create index `idx_eps_containing_ettle_ordinal ON eps(containing_ettle, ordinal)`.
  - Create index `idx_ettles_parent_ettle_id ON ettles(parent_ettle_id)`.
  - Create index `idx_eps_parent_ep_id ON eps(parent_ep_id)`.

`eps.parent_ep_id` starts as NULL for all existing rows. The DB repair that populates
existing `parent_ep_id` values is performed via EpUpdate commands once the command
layer is available in this Ettle; no backfill SQL is needed.

Comment block MUST be included:
```sql
-- Migration 012: Structural model update.
-- Renames ettle_id->containing_ettle, parent_id->parent_ettle_id.
-- Drops child_ettle_id, parent_ep_id (on ettles), content_digest, content_inline.
-- Adds eps.parent_ep_id (cross-Ettle EP refinement link).
-- Requires rusqlite bundled (SQLite >= 3.35.0 for DROP COLUMN).
-- See ettle:019ccf15-e2b1-7e33-9794-bf2cf0704178 EP0 for full specification.
```

#### 1.2 UNIQUE constraint

The existing `UNIQUE (ettle_id, ordinal)` constraint on `eps` must be recreated as
`UNIQUE (containing_ettle, ordinal)` after the rename. SQLite requires a table
recreation to alter constraints; the migration MUST handle this via the
create-new-table / copy / drop-old / rename pattern if the rename alone does not
preserve the constraint name correctly. The agent MUST verify the constraint is
present and correctly named after migration.

---

### 2. Domain type updates (ettlex-core-types)

  - `EttleRecord` MUST reflect: `id`, `title`, `parent_ettle_id` (Option<String>), `deleted`,
    `created_at`, `updated_at`, `metadata`.
    Old fields `parent_id` and `parent_ep_id` MUST NOT appear.
  - `EpRecord` MUST reflect: `id`, `containing_ettle` (String), `ordinal`, `title` (Option<String>),
    `normative`, `content` (JSON), `digest` (Option<String>), `parent_ep_id` (Option<String>),
    `deleted`, `created_at`, `updated_at`.
    Old fields `ettle_id`, `child_ettle_id`, `content_digest`, `content_inline` MUST NOT appear.
  - Update `EttleCreateCommand`: add `parent_ettle_id: Option<String>`.
  - Update `EttleUpdateCommand`: add `parent_ettle_id: Option<Option<String>>`
    (outer Option = field present in request; inner Option = nullable value).
  - Add `EttleTombstoneCommand { ettle_id: String }`.
  - Update `EpCreateCommand`: `containing_ettle` replaces `ettle_id`; add `parent_ep_id: Option<String>`.
  - Update `EpUpdateCommand`: add `parent_ep_id: Option<Option<String>>`.
  - Add `EpTombstoneCommand { ep_id: String }`.
  - Add error variants:
    `SameEttleParentLink`, `EttleTreeInconsistency`, `SelfReferentialEttle`,
    `EttleCycleDetected`, `EpCycleDetected`, `DeletedEttle`, `DeletedEp`,
    `AlreadyDeleted`, `HasActiveChildren`, `HasActiveEps`, `RefinementIntegrityViolation`.

---

### 3. Integrity constraints (enforced in engine/action layer, not DB triggers)

  **IC-1:** `eps.parent_ep_id` MUST reference an EP whose `containing_ettle` differs from
        the referencing EP's `containing_ettle`. Same-Ettle parent links MUST be rejected
        with typed error `SameEttleParentLink`.

  **IC-2:** If EP B has `parent_ep_id` pointing to EP A (in Ettle X), then B's
        `containing_ettle` (Ettle Y) MUST have `parent_ettle_id = X`. If this consistency
        is violated the command MUST fail with typed error `EttleTreeInconsistency`.

  **IC-3:** An EP MUST have at most one parent EP (`parent_ep_id` is a single nullable
        column; strict tree enforced structurally).

  **IC-4:** `ettles.parent_ettle_id` MUST NOT reference the Ettle itself (no self-loops).
        Violation MUST be rejected with typed error `SelfReferentialEttle`.

  **IC-5:** A cycle in `parent_ettle_id` links MUST be detected and rejected with typed
        error `EttleCycleDetected`.

  **IC-6:** A cycle in `parent_ep_id` links MUST be detected and rejected with typed error
        `EpCycleDetected`.

---

### 4. Command vocabulary changes (action layer)

All commands MUST flow through `action:commands::apply`. No store/engine mutation
functions MAY be called directly from CLI or MCP.

#### 4.1 EttleCreate
```
Input fields:
  - title (required, non-empty string)
  - parent_ettle_id (optional; if supplied, referenced Ettle MUST exist and MUST NOT be deleted)
  - metadata (optional JSON)
Output: { ettle_id, new_state_version }
Invariants:
  - Generated ettle_id MUST be stable (ULIDv7 or equivalent).
  - If parent_ettle_id is supplied and does not exist: typed error NotFound.
  - If parent_ettle_id is supplied and is deleted: typed error DeletedEttle.
```

#### 4.2 EttleUpdate
```
Input fields:
  - ettle_id (required)
  - title (optional; if supplied replaces existing title; MUST be non-empty)
  - parent_ettle_id (optional; if supplied replaces existing parent; null clears the parent)
  - metadata (optional; if supplied replaces existing metadata)
Output: { new_state_version }
Invariants:
  - If ettle_id does not exist: typed error NotFound.
  - If ettle_id is deleted: typed error DeletedEttle.
  - If new parent_ettle_id would create a cycle: typed error EttleCycleDetected.
  - IC-4 applies: self-referential parent rejected with SelfReferentialEttle.
  - Updating parent_ettle_id does NOT implicitly update any EP parent_ep_id fields.
    Tree consistency (IC-2) is the responsibility of the caller.
```

#### 4.3 EttleTombstone
```
Input fields:
  - ettle_id (required)
Output: { new_state_version }
Invariants:
  - Sets deleted = 1 and updated_at; does NOT delete rows (append-only).
  - If ettle_id does not exist: typed error NotFound.
  - If already tombstoned: typed error AlreadyDeleted.
  - Tombstoning an Ettle that has non-deleted child Ettles (via parent_ettle_id)
    MUST be rejected with typed error HasActiveChildren.
  - Tombstoning an Ettle that contains non-deleted EPs MUST be rejected with
    typed error HasActiveEps.
```

#### 4.4 EpCreate
```
Input fields:
  - ettle_id (required; stored as containing_ettle; referenced Ettle MUST exist and not be deleted)
  - ordinal (required; integer >= 0; MUST be unique within the Ettle)
  - normative (required; boolean)
  - title (optional string)
  - why, what, how (optional strings; stored as JSON in content field)
  - parent_ep_id (optional; if supplied IC-1 and IC-2 MUST be enforced)
Output: { ep_id, new_state_version }
Invariants:
  - Generated ep_id MUST be stable (ULIDv7 or equivalent).
  - Ordinal MUST be unique within the Ettle; duplicate rejected with OrdinalConflict.
  - If parent_ep_id does not exist: typed error NotFound.
  - If parent_ep_id is deleted: typed error DeletedEp.
  - IC-1: same-Ettle parent rejected with SameEttleParentLink.
  - IC-2: Ettle tree consistency enforced; violation rejected with EttleTreeInconsistency.
  - IC-6: cycle detection on parent_ep_id; rejected with EpCycleDetected.
```

#### 4.5 EpUpdate
```
Input fields:
  - ep_id (required)
  - title (optional; replaces existing)
  - why, what, how (optional strings; each replaces its respective field in content JSON)
  - normative (optional; replaces existing)
  - parent_ep_id (optional; replaces existing parent; null clears the parent link)
Output: { new_state_version }
Invariants:
  - If ep_id does not exist: typed error NotFound.
  - If ep_id is deleted: typed error DeletedEp.
  - IC-1, IC-2, IC-6 apply when parent_ep_id is being set or changed.
  - Updating parent_ep_id MUST NOT implicitly update ettles.parent_ettle_id.
    Ettle tree consistency (IC-2) is the responsibility of the caller.
```

#### 4.6 EpTombstone
```
Input fields:
  - ep_id (required)
Output: { new_state_version }
Invariants:
  - Sets deleted = 1 and updated_at; does NOT delete rows.
  - If ep_id does not exist: typed error NotFound.
  - If already tombstoned: typed error AlreadyDeleted.
  - Tombstoning an EP that has child EPs (via parent_ep_id on other EPs) MUST be
    rejected with typed error HasActiveChildren.
```

---

### 5. Query surface changes (action:queries)

  - `ettle.get(ettle_id)` MUST return: `id`, `title`, `parent_ettle_id`, `deleted`, `created_at`,
    `updated_at`, `metadata`, and the list of non-deleted EP ids belonging to the Ettle
    (via `containing_ettle` lookup). Does NOT return `parent_id` or `parent_ep_id`.
  - `ep.get(ep_id)` MUST return: `id`, `containing_ettle`, `ordinal`, `title`, `normative`,
    `why`, `what`, `how`, `digest`, `parent_ep_id`, `deleted`, `created_at`, `updated_at`.
    Does NOT return `ettle_id`, `child_ettle_id`, `content_digest`, `content_inline`.
  - `ep.list_children(ep_id)` MUST return EPs where `parent_ep_id = ep_id` (direct
    children only; non-deleted by default; `include_deleted` option available).
  - `ep.list_parents(ep_id)` MUST return the single parent EP via `parent_ep_id` on this
    EP, or empty if root. If the data is corrupt and multiple parents are somehow
    present (impossible under new schema but guarded for safety), MUST return
    `RefinementIntegrityViolation`.
  - `ettle.list_children(ettle_id)` MUST return Ettles where `parent_ettle_id = ettle_id`.
  - `ettle.list_parents(ettle_id)` MUST return the single parent Ettle via `parent_ettle_id`,
    or empty if root.
  - All existing query tools that previously referenced `ettle_id` on EPs MUST be updated
    to reference `containing_ettle`.
  - All existing query tools that previously referenced `child_ettle_id` or `parent_id` MUST
    be updated to use the new field names and link model.

---

### 6. Leaf EP definition (updated)

  A leaf EP is an EP that has no child EPs — i.e., no other EP has `parent_ep_id`
  pointing to it. This replaces the previous definition (leaf EP = EP with
  `child_ettle_id = null`). All code that detects leaf status MUST use the new definition.

---

### 7. MCP and CLI layer

  - MCP (`ettlex.apply`) MUST accept all new command fields defined in section 4.
  - MCP MUST NOT implement any validation logic beyond schema validation (types,
    required fields). All semantic validation (IC-1 through IC-6, tombstone guards)
    lives in the action layer.
  - CLI MUST provide thin wrappers for all six commands (`EttleCreate`, `EttleUpdate`,
    `EttleTombstone`, `EpCreate`, `EpUpdate`, `EpTombstone`). No business logic in CLI.
  - MCP tool `ettle.get` response MUST include `parent_ettle_id` (not `parent_id`).
  - MCP tool `ep.get` response MUST include `containing_ettle` (not `ettle_id`) and
    `parent_ep_id` (not `child_ettle_id`).

---

### 8. Expected Failure Registry (pre-authorised; see HOW for #[ignore] protocol)

The following test files contain scenarios that WILL fail as a direct consequence of
this migration. They are pre-authorised failures. The code generation agent MUST
`#[ignore]`-tag them and MUST NOT modify their test logic or production code to pass them:

  - **EFR-01:** `ettlex-engine/tests/action_read_tools_integration_tests.rs`
    Reason: references old field names (`ettle_id`, `child_ettle_id`); EPT traversal logic changed.
  - **EFR-02:** `ettlex-engine/tests/snapshot_commit_by_leaf_tests.rs`
    Reason: leaf detection uses old `child_ettle_id = null` definition.
  - **EFR-03:** `ettlex-engine/tests/snapshot_commit_legacy_resolution_tests.rs`
    Reason: legacy root->leaf resolution traverses `child_ettle_id`.
  - **EFR-04:** `ettlex-engine/tests/snapshot_commit_tests.rs`
    Reason: EPT traversal in commit pipeline uses old field names.
  - **EFR-05:** `ettlex-engine/tests/snapshot_commit_determinism_tests.rs`
    Reason: EPT traversal used in determinism harness.
  - **EFR-06:** `ettlex-engine/tests/snapshot_commit_idempotency_tests.rs`
    Reason: EPT traversal used in idempotency harness.
  - **EFR-07:** `ettlex-engine/tests/decision_tests.rs`
    Reason: ancestor walk for `include_ancestors` uses old traversal.
  - **EFR-08:** `ettlex-engine/tests/identity_contract_tests.rs`
    Reason: asserts on serialised field names (`ettle_id`, `child_ettle_id`).
  - **EFR-09:** `ettlex-store/tests/round_trip_test.rs`
    Reason: round-trip asserts on old schema field names.
  - **EFR-10:** `ettlex-store/tests/hydration_test.rs`
    Reason: hydration logic reads `ettle_id` and `child_ettle_id` columns.
  - **EFR-11:** `ettlex-store/tests/seed_parse_test.rs`
    Reason: seed parser writes `ettle_id` and `child_ettle_id` columns.
  - **EFR-12:** `ettlex-store/tests/migrations_test.rs`
    Reason: asserts on schema shape after migrations 009/010 which are superseded.
  - **EFR-13:** `ettlex-engine/tests/mcp_ep_update_tests.rs`
    Reason: references `ettle_id` in EP response shape assertions.
  - **EFR-14:** `ettlex-engine/tests/ep_update_engine_tests.rs`
    Reason: references `ettle_id` and `child_ettle_id` in engine-level EP tests.

---

## HOW

Layer-by-layer implementation instructions. No function signatures are specified.
All layers MUST be implemented in order. The code generation agent MUST NOT implement
behavioural logic in a higher layer (MCP/CLI) that belongs in a lower layer (engine/store).

---

### LAYER 1: Store (ettlex-store)

#### Migration file

Create `migrations/012_structural_model_update.sql`.

rusqlite 0.29 bundles SQLite 3.43.x, which fully supports ALTER TABLE RENAME COLUMN
(since 3.25.0) and DROP COLUMN (since 3.35.0). Use these directly.

Required operations in order:
```sql
1.  ALTER TABLE eps RENAME COLUMN ettle_id TO containing_ettle;
2.  ALTER TABLE ettles RENAME COLUMN parent_id TO parent_ettle_id;
3.  ALTER TABLE ettles DROP COLUMN parent_ep_id;
4.  ALTER TABLE eps DROP COLUMN child_ettle_id;
5.  ALTER TABLE eps DROP COLUMN content_digest;
6.  ALTER TABLE eps DROP COLUMN content_inline;
7.  ALTER TABLE eps ADD COLUMN parent_ep_id TEXT REFERENCES eps(id);
8.  DROP INDEX idx_eps_ettle_id;
9.  DROP INDEX idx_ettles_parent_id;
10. CREATE INDEX idx_eps_containing_ettle ON eps(containing_ettle);
11. CREATE INDEX idx_eps_containing_ettle_ordinal ON eps(containing_ettle, ordinal);
12. CREATE INDEX idx_ettles_parent_ettle_id ON ettles(parent_ettle_id);
13. CREATE INDEX idx_eps_parent_ep_id ON eps(parent_ep_id);
```

UNIQUE constraint: The existing `UNIQUE (ettle_id, ordinal)` constraint is defined
in the CREATE TABLE statement in migration 001. SQLite preserves this constraint
through a column rename, but the agent MUST verify post-migration that the
`UNIQUE (containing_ettle, ordinal)` constraint is in effect. If verification fails,
the agent MUST recreate the table with the correct constraint (create new table,
copy data, drop old table, rename new table) and document this in the migration file.

Include comment block per WHAT section 1.1.

#### Store-layer row types
  - Update `EttleRow` struct: use `parent_ettle_id` replacing `parent_id`; remove `parent_ep_id`.
  - Update `EpRow` struct: `containing_ettle` replaces `ettle_id`; add `parent_ep_id: Option<String>`;
    remove `child_ettle_id`, `content_digest`, `content_inline`.
  - Update all SELECT projection lists to use new column names.
  - Update all INSERT and UPDATE statements to use new column names.
  - Old field names (`ettle_id`, `child_ettle_id`, `parent_id`, `parent_ep_id` on ettles,
    `content_digest`, `content_inline`) MUST NOT appear anywhere in store code after this.

#### Migration test
MUST verify:
  a) Migration 012 applies cleanly on top of migrations 001-011.
  b) `eps` table has `containing_ettle` column; `ettle_id` column DOES NOT exist.
  c) `eps` table has `parent_ep_id` column; `child_ettle_id`, `content_digest`,
     `content_inline` columns DO NOT exist.
  d) `ettles` table has `parent_ettle_id` column; `parent_id` column DOES NOT exist;
     `parent_ep_id` column DOES NOT exist.
  e) Indexes `idx_eps_containing_ettle`, `idx_eps_containing_ettle_ordinal`,
     `idx_ettles_parent_ettle_id`, `idx_eps_parent_ep_id` all exist.
  f) Indexes `idx_eps_ettle_id` and `idx_ettles_parent_id` DO NOT exist.
  g) `UNIQUE (containing_ettle, ordinal)` constraint is in effect on `eps`.
  h) Existing data rows survive migration with values correctly preserved in
     renamed columns; `parent_ep_id` is NULL for all pre-existing rows.

---

### LAYER 2: Core types (ettlex-core-types)

  - Update `EttleRecord`: `parent_ettle_id: Option<String>`; remove `parent_id`, `parent_ep_id`.
  - Update `EpRecord`: `containing_ettle: String`; add `parent_ep_id: Option<String>`;
    remove `ettle_id`, `child_ettle_id`, `content_digest`, `content_inline`.
  - Update `EttleCreateCommand`: add `parent_ettle_id: Option<String>`.
  - Update `EttleUpdateCommand`: add `parent_ettle_id: Option<Option<String>>`.
  - Add `EttleTombstoneCommand { ettle_id: String }`.
  - Update `EpCreateCommand`: `containing_ettle` replaces `ettle_id`;
    add `parent_ep_id: Option<String>`.
  - Update `EpUpdateCommand`: add `parent_ep_id: Option<Option<String>>`.
  - Add `EpTombstoneCommand { ep_id: String }`.
  - Add error variants: `SameEttleParentLink`, `EttleTreeInconsistency`,
    `SelfReferentialEttle`, `EttleCycleDetected`, `EpCycleDetected`, `DeletedEttle`,
    `DeletedEp`, `AlreadyDeleted`, `HasActiveChildren`, `HasActiveEps`.

---

### LAYER 3: Engine (ettlex-engine)

  - Implement command handlers for all six commands per WHAT section 4.
  - Each handler MUST enforce IC-1 through IC-6 before writing to store.
  - Cycle detection (IC-5, IC-6): walk parent chain upward; depth limit 1000.
  - Update `ettle.get`: read `parent_ettle_id`; return in response.
  - Update `ep.get`: read `containing_ettle` and `parent_ep_id`; return in response.
  - Implement `ep.list_children(ep_id)`: EPs where `parent_ep_id = ep_id`.
  - Implement `ep.list_parents(ep_id)`: single parent EP via `parent_ep_id`; empty if null.
  - Implement `ettle.list_children(ettle_id)`: Ettles where `parent_ettle_id = ettle_id`.
  - Implement `ettle.list_parents(ettle_id)`: single parent Ettle via `parent_ettle_id`.
  - Update leaf EP detection: leaf = no non-deleted EP has `parent_ep_id` = this `ep_id`.
  - Update all engine code that reads `ettle_id`, `child_ettle_id`, `parent_id`,
    or `parent_ep_id` (on Ettles) to use the new field names.
  - **NOTE:** EPT computation, snapshot commit traversal, and ancestor walks MUST NOT
    be updated in this Ettle. Those subsystems reference the old link model and will
    fail. They MUST be `#[ignore]`-tagged per EFR-01 through EFR-14. The agent MUST
    NOT attempt to fix EPT traversal in order to make those tests pass.

---

### LAYER 4: Actions (ettlex-engine action layer)

  - Register all six commands in the action dispatch table:
    `EttleCreate`, `EttleUpdate`, `EttleTombstone`, `EpCreate`, `EpUpdate`, `EpTombstone`.
  - Delegate directly to engine handlers; no semantic logic in dispatch layer.
  - Increment `state_version` by 1 for each successful command.
  - Append one provenance event per successful mutation:
    `ettle_created`, `ettle_updated`, `ettle_tombstoned`,
    `ep_created`, `ep_updated`, `ep_tombstoned`.
  - Failed commands MUST NOT append provenance events.

---

### LAYER 5: MCP (ettlex-mcp)

  - Update `ettlex.apply` schema to accept all new command fields for all six commands.
  - Type/required field validation only; IC enforcement is the engine's responsibility.
  - Update `ettle.get` tool response: emit `parent_ettle_id` (not `parent_id`);
    `ep_ids` list from `containing_ettle` lookup.
  - Update `ep.get` tool response: emit `containing_ettle` (not `ettle_id`); `parent_ep_id`
    (not `child_ettle_id`); no `content_digest` or `content_inline`.
  - Add/update MCP tools: `ettle.list_children`, `ettle.list_parents`,
    `ep.list_children`, `ep.list_parents`.
  - All tool responses MUST use canonical JSON with stable key ordering.

---

### LAYER 6: CLI (ettlex-cli)

  - Add/update subcommands: `ettle create`, `ettle update`, `ettle tombstone`,
    `ep create`, `ep update`, `ep tombstone`.
  - All subcommands call `action:commands::apply` with the appropriate command DTO.
  - No direct store or engine calls from CLI.
  - Print `new_state_version` on success; typed error code and message on failure.

---

### EXPECTED FAILURE PROTOCOL (mandatory; code generation agent MUST follow)

When the full test suite is run after implementing this Ettle, tests listed in WHAT
section 8 (EFR-01 through EFR-14) WILL fail. The code generation agent MUST:

  1. Apply `#[ignore = "pending: schema_migration_012"]` to each failing test.
  2. MUST NOT modify the test logic of any ignored test.
  3. MUST NOT modify production code outside layers 1-6 to make a pre-authorised
     failing test pass.
  4. MUST NOT modify any test file NOT listed in EFR-01 through EFR-14 to make
     tests pass. Any failure outside that list is either:
       a) a bug introduced by this implementation — fix within scope, or
       b) a pre-existing failure — report in completion report; do NOT silently fix.
  5. MUST document all `#[ignore]`-tagged tests in the completion report under
     "Pre-Authorised Failures" with: test file path, test function name,
     EFR reference, one-line reason.

---

### Scenarios (all MUST be implemented as tests; Gherkin is normative)

```gherkin
Feature: Migration 012 applies cleanly and produces correct schema

  Background:
    Given a database with migrations 001-011 applied

  Scenario: Migration 012 applies without error
    When migration 012 is applied
    Then no SQL error occurs
    And the migration runner records migration 012 as applied

  Scenario: eps table has correct columns after migration 012
    When migration 012 is applied
    Then column containing_ettle exists on eps
    And column parent_ep_id exists on eps
    And column ettle_id does NOT exist on eps
    And column child_ettle_id does NOT exist on eps
    And column content_digest does NOT exist on eps
    And column content_inline does NOT exist on eps

  Scenario: ettles table has correct columns after migration 012
    When migration 012 is applied
    Then column parent_ettle_id exists on ettles
    And column parent_id does NOT exist on ettles
    And column parent_ep_id does NOT exist on ettles

  Scenario: New indexes exist and old indexes are absent after migration 012
    When migration 012 is applied
    Then idx_eps_containing_ettle exists
    And idx_eps_containing_ettle_ordinal exists
    And idx_ettles_parent_ettle_id exists
    And idx_eps_parent_ep_id exists
    And idx_eps_ettle_id does NOT exist
    And idx_ettles_parent_id does NOT exist

  Scenario: UNIQUE (containing_ettle, ordinal) constraint is in effect after migration 012
    When migration 012 is applied
    And I insert two EPs with the same containing_ettle and same ordinal
    Then a UNIQUE constraint violation error is returned

  Scenario: Existing rows survive migration with values preserved
    Given existing Ettle and EP rows before migration 012
    When migration 012 is applied
    Then EP rows have containing_ettle equal to the value previously in ettle_id
    And Ettle rows have parent_ettle_id equal to the value previously in parent_id
    And EP rows have parent_ep_id as NULL

Feature: EttleCreate command with parent_ettle_id support

  Background:
    Given migration 012 has been applied and action layer is available

  Scenario: EttleCreate with title only creates a root Ettle
    When I apply Command::EttleCreate{title="My Ettle"}
    Then a new ettle_id is returned
    And ettle.get(ettle_id).parent_ettle_id is null
    And state_version increments by 1
    And provenance event ettle_created is recorded

  Scenario: EttleCreate with parent_ettle_id creates a child Ettle
    Given Ettle P exists
    When I apply Command::EttleCreate{title="Child", parent_ettle_id=P}
    Then ettle.get(new_ettle_id).parent_ettle_id equals P

  Scenario: EttleCreate rejects empty title
    When I apply Command::EttleCreate{title=""}
    Then a typed error InvalidInput is returned
    And state_version is unchanged

  Scenario: EttleCreate rejects non-existent parent_ettle_id
    When I apply Command::EttleCreate{title="X", parent_ettle_id="ettle:missing"}
    Then a typed error NotFound is returned

  Scenario: EttleCreate rejects deleted parent_ettle_id
    Given Ettle P is tombstoned
    When I apply Command::EttleCreate{title="X", parent_ettle_id=P}
    Then a typed error DeletedEttle is returned

  Scenario: EttleCreate is atomic — no partial rows on failure
    When I apply Command::EttleCreate with invalid parent_ettle_id
    Then a typed error is returned and no ettle row is created

Feature: EttleUpdate command

  Scenario: EttleUpdate changes title
    Given Ettle E with title "Old"
    When I apply Command::EttleUpdate{ettle_id=E, title="New"}
    Then ettle.get(E).title equals "New"
    And state_version increments by 1
    And provenance event ettle_updated is recorded

  Scenario: EttleUpdate sets parent_ettle_id
    Given Ettle E with parent_ettle_id null, Ettle P exists
    When I apply Command::EttleUpdate{ettle_id=E, parent_ettle_id=P}
    Then ettle.get(E).parent_ettle_id equals P

  Scenario: EttleUpdate clears parent_ettle_id
    Given Ettle E with parent_ettle_id=P
    When I apply Command::EttleUpdate{ettle_id=E, parent_ettle_id=null}
    Then ettle.get(E).parent_ettle_id is null

  Scenario: EttleUpdate rejects self-referential parent (IC-4)
    Given Ettle E
    When I apply Command::EttleUpdate{ettle_id=E, parent_ettle_id=E}
    Then a typed error SelfReferentialEttle is returned
    And ettle.get(E).parent_ettle_id is unchanged

  Scenario: EttleUpdate rejects cycle-forming parent (IC-5)
    Given Ettle A with parent_ettle_id=null, Ettle B with parent_ettle_id=A
    When I apply Command::EttleUpdate{ettle_id=A, parent_ettle_id=B}
    Then a typed error EttleCycleDetected is returned

  Scenario: EttleUpdate rejects tombstoned Ettle
    Given Ettle E is tombstoned
    When I apply Command::EttleUpdate{ettle_id=E, title="New"}
    Then a typed error DeletedEttle is returned

  Scenario: EttleUpdate with no optional fields is a no-op success
    Given Ettle E with title "Same"
    When I apply Command::EttleUpdate{ettle_id=E} with no optional fields
    Then the command succeeds and state_version increments by 1

Feature: EttleTombstone command

  Scenario: EttleTombstone marks Ettle as deleted; row retained
    Given Ettle E with no child Ettles and no EPs
    When I apply Command::EttleTombstone{ettle_id=E}
    Then ettle.get(E).deleted is true
    And the row still exists in the DB
    And state_version increments by 1
    And provenance event ettle_tombstoned is recorded

  Scenario: EttleTombstone rejects Ettle with active child Ettles
    Given Ettle P, Ettle C with parent_ettle_id=P
    When I apply Command::EttleTombstone{ettle_id=P}
    Then a typed error HasActiveChildren is returned

  Scenario: EttleTombstone rejects Ettle with active EPs
    Given Ettle E, EP X with containing_ettle=E and deleted=false
    When I apply Command::EttleTombstone{ettle_id=E}
    Then a typed error HasActiveEps is returned

  Scenario: EttleTombstone rejects already-tombstoned Ettle
    Given Ettle E is tombstoned
    When I apply Command::EttleTombstone{ettle_id=E}
    Then a typed error AlreadyDeleted is returned

  Scenario: EttleTombstone rejects non-existent Ettle
    When I apply Command::EttleTombstone{ettle_id="ettle:missing"}
    Then a typed error NotFound is returned

Feature: EpCreate command with parent_ep_id support

  Background:
    Given migration 012 has been applied and Ettle E exists

  Scenario: EpCreate with minimum fields succeeds
    When I apply Command::EpCreate{containing_ettle=E, ordinal=0, normative=true}
    Then a new ep_id is returned
    And ep.get(ep_id).containing_ettle equals E
    And ep.get(ep_id).parent_ep_id is null
    And state_version increments by 1
    And provenance event ep_created is recorded

  Scenario: EpCreate with cross-Ettle parent_ep_id succeeds (IC-2 satisfied)
    Given Ettle P with parent_ettle_id=null, EP parent_ep with containing_ettle=P
    And Ettle E with parent_ettle_id=P
    When I apply Command::EpCreate{containing_ettle=E, ordinal=0, normative=true, parent_ep_id=parent_ep}
    Then ep.get(new_ep_id).parent_ep_id equals parent_ep

  Scenario: EpCreate rejects parent_ep_id in same Ettle (IC-1)
    Given EP sibling with containing_ettle=E
    When I apply Command::EpCreate{containing_ettle=E, ordinal=1, normative=true, parent_ep_id=sibling}
    Then a typed error SameEttleParentLink is returned
    And state_version is unchanged

  Scenario: EpCreate rejects Ettle-tree-inconsistent parent_ep_id (IC-2)
    Given Ettle X (unrelated to E), EP foreign_ep with containing_ettle=X
    When I apply Command::EpCreate{containing_ettle=E, ordinal=0, normative=true, parent_ep_id=foreign_ep}
    Then a typed error EttleTreeInconsistency is returned

  Scenario: EpCreate rejects duplicate ordinal within Ettle
    Given EP with ordinal 0 already exists in Ettle E
    When I apply Command::EpCreate{containing_ettle=E, ordinal=0, normative=true}
    Then a typed error OrdinalConflict is returned

  Scenario: EpCreate rejects non-existent containing_ettle
    When I apply Command::EpCreate{containing_ettle="ettle:missing", ordinal=0, normative=true}
    Then a typed error NotFound is returned

  Scenario: EpCreate rejects deleted containing_ettle
    Given Ettle E is tombstoned
    When I apply Command::EpCreate{containing_ettle=E, ordinal=0, normative=true}
    Then a typed error DeletedEttle is returned

  Scenario: EpCreate stores WHY/WHAT/HOW in content JSON
    When I apply Command::EpCreate{containing_ettle=E, ordinal=0, normative=true, why="W", what="X", how="H"}
    Then ep.get(ep_id).why equals "W"
    And ep.get(ep_id).what equals "X"
    And ep.get(ep_id).how equals "H"

Feature: EpUpdate command with parent_ep_id support

  Background:
    Given migration 012 has been applied, Ettle E exists
    And EP X with containing_ettle=E, ordinal=0, parent_ep_id=null

  Scenario: EpUpdate changes WHY/WHAT/HOW
    When I apply Command::EpUpdate{ep_id=X, why="new why", what="new what", how="new how"}
    Then ep.get(X) returns the new values
    And state_version increments by 1
    And provenance event ep_updated is recorded

  Scenario: EpUpdate sets cross-Ettle parent_ep_id (valid)
    Given Ettle P with parent_ettle_id=null, EP parent_ep with containing_ettle=P
    And Ettle E has parent_ettle_id=P
    When I apply Command::EpUpdate{ep_id=X, parent_ep_id=parent_ep}
    Then ep.get(X).parent_ep_id equals parent_ep

  Scenario: EpUpdate clears parent_ep_id
    Given EP X has parent_ep_id set
    When I apply Command::EpUpdate{ep_id=X, parent_ep_id=null}
    Then ep.get(X).parent_ep_id is null

  Scenario: EpUpdate rejects same-Ettle parent_ep_id (IC-1)
    Given EP sibling with containing_ettle=E
    When I apply Command::EpUpdate{ep_id=X, parent_ep_id=sibling}
    Then a typed error SameEttleParentLink is returned

  Scenario: EpUpdate rejects Ettle-tree-inconsistent parent_ep_id (IC-2)
    Given Ettle Y unrelated to E, EP foreign with containing_ettle=Y
    When I apply Command::EpUpdate{ep_id=X, parent_ep_id=foreign}
    Then a typed error EttleTreeInconsistency is returned

  Scenario: EpUpdate rejects cycle-forming parent_ep_id (IC-6)
    Given EP X in Ettle E, EP Y in Ettle F, Y.parent_ep_id=X
    When I apply Command::EpUpdate{ep_id=X, parent_ep_id=Y}
    Then a typed error EpCycleDetected is returned

  Scenario: EpUpdate rejects tombstoned EP
    Given EP X is tombstoned
    When I apply Command::EpUpdate{ep_id=X, why="new"}
    Then a typed error DeletedEp is returned

  Scenario: EpUpdate rejects non-existent EP
    When I apply Command::EpUpdate{ep_id="ep:missing", why="new"}
    Then a typed error NotFound is returned

  Scenario: EpUpdate with no optional fields is a no-op success
    Given EP X with why="original"
    When I apply Command::EpUpdate{ep_id=X} with no optional fields
    Then the command succeeds, state_version increments by 1
    And ep.get(X).why is still "original"

Feature: EpTombstone command

  Scenario: EpTombstone marks EP as deleted; row retained
    Given EP X with no child EPs
    When I apply Command::EpTombstone{ep_id=X}
    Then ep.get(X).deleted is true
    And the row still exists in the DB
    And state_version increments by 1
    And provenance event ep_tombstoned is recorded

  Scenario: EpTombstone rejects EP with active child EPs
    Given EP X, EP Y with parent_ep_id=X
    When I apply Command::EpTombstone{ep_id=X}
    Then a typed error HasActiveChildren is returned
    And ep.get(X).deleted is false

  Scenario: EpTombstone rejects already-tombstoned EP
    Given EP X is tombstoned
    When I apply Command::EpTombstone{ep_id=X}
    Then a typed error AlreadyDeleted is returned

  Scenario: EpTombstone rejects non-existent EP
    When I apply Command::EpTombstone{ep_id="ep:missing"}
    Then a typed error NotFound is returned

Feature: Leaf EP definition uses new structural model

  Scenario: EP with no child EPs is a leaf
    Given EP X, no other EP has parent_ep_id=X
    When I check if X is a leaf EP
    Then X is identified as a leaf EP

  Scenario: EP with at least one child EP is not a leaf
    Given EP X, EP Y with parent_ep_id=X
    When I check if X is a leaf EP
    Then X is NOT identified as a leaf EP

  Scenario: Deleted child EP does not prevent parent from being a leaf
    Given EP X, EP Y with parent_ep_id=X and deleted=true
    When I check if X is a leaf EP
    Then X IS identified as a leaf EP

Feature: Query surface returns correct fields

  Scenario: ettle.get returns parent_ettle_id not parent_id
    Given Ettle C with parent_ettle_id=P
    When I call ettle.get(C)
    Then the response contains field parent_ettle_id
    And the response does NOT contain field parent_id or parent_ep_id

  Scenario: ettle.get returns list of EP ids belonging to Ettle
    Given Ettle E, EPs X Y Z with containing_ettle=E
    When I call ettle.get(E)
    Then the response ep_ids list contains X, Y, Z
    And deleted EPs are excluded by default

  Scenario: ep.get returns containing_ettle not ettle_id
    Given EP X with containing_ettle=E
    When I call ep.get(X)
    Then the response contains field containing_ettle equal to E
    And the response does NOT contain ettle_id, child_ettle_id, content_digest, or content_inline

  Scenario: ep.list_children returns direct child EPs only
    Given EP parent, EPs child_a and child_b with parent_ep_id=parent
    And EP grandchild with parent_ep_id=child_a
    When I call ep.list_children(parent)
    Then the response contains child_a and child_b but NOT grandchild

  Scenario: ep.list_parents returns single parent EP
    Given EP child with parent_ep_id=parent_ep
    When I call ep.list_parents(child)
    Then exactly one EP id is returned: parent_ep

  Scenario: ep.list_parents returns empty for root EP
    Given EP root with parent_ep_id=null
    When I call ep.list_parents(root)
    Then an empty list is returned

  Scenario: ettle.list_children returns direct child Ettles
    Given Ettle parent, Ettles child_a and child_b with parent_ettle_id=parent
    When I call ettle.list_children(parent)
    Then the response contains child_a and child_b

  Scenario: ettle.list_parents returns single parent Ettle
    Given Ettle child with parent_ettle_id=parent_ettle
    When I call ettle.list_parents(child)
    Then exactly one Ettle id is returned: parent_ettle

Feature: MCP transport exposes new command fields

  Background:
    Given MCP server is started and migration 012 has been applied

  Scenario: MCP EttleCreate accepts parent_ettle_id
    Given Ettle P exists
    When I call ettlex.apply via MCP with EttleCreate{title="T", parent_ettle_id=P}
    Then new_state_version increments by 1
    And ettle.get(new_id).parent_ettle_id equals P

  Scenario: MCP EpCreate accepts parent_ep_id
    Given a valid cross-Ettle parent setup (IC-2 satisfied)
    When I call ettlex.apply via MCP with EpCreate{..., parent_ep_id=valid_parent}
    Then ep.get(new_id).parent_ep_id equals valid_parent

  Scenario: MCP ettle.get response contains parent_ettle_id not parent_id
    Given Ettle C with parent_ettle_id=P
    When I call MCP tool ettle.get(C)
    Then response JSON contains key parent_ettle_id and NOT parent_id

  Scenario: MCP ep.get response contains containing_ettle not ettle_id
    Given EP X
    When I call MCP tool ep.get(X)
    Then response JSON contains containing_ettle and NOT ettle_id or child_ettle_id

  Scenario: MCP surfaces SameEttleParentLink as stable error_code
    When I call ettlex.apply via MCP with EpCreate using same-Ettle parent_ep_id
    Then error_code SameEttleParentLink is returned

  Scenario: MCP surfaces EttleCycleDetected as stable error_code
    When I call ettlex.apply via MCP with a cycle-forming EttleUpdate
    Then error_code EttleCycleDetected is returned

Feature: Optimistic concurrency on all commands

  Scenario: Command with correct expected_state_version succeeds
    Given state_version is V
    When I apply any command with expected_state_version=V
    Then the command succeeds and new_state_version is V+1

  Scenario: Command with wrong expected_state_version fails
    Given state_version is V
    When I apply any command with expected_state_version=V-1
    Then a typed error HeadMismatch is returned
    And state_version remains V and no row is created or modified

Feature: Provenance and observability

  Scenario: Each successful mutation appends exactly one provenance event
    When I apply any of the six commands successfully
    Then exactly one provenance_event row is appended
    And the event kind matches the command
    And the event includes a correlation_id

  Scenario: Failed commands append no provenance events
    When I apply a command that returns any typed error
    Then no provenance_event row is appended and the database state is unchanged

  # N/A: Performance thresholds — not specified for CRUD at this tier.
  # N/A: Security/auth — no auth changes in this Ettle.
  # N/A: Byte-level determinism — CRUD outputs are not digest-compared here.
  # N/A: Concurrency beyond OCC — SQLite serialises writes.
```
