# Leaf Bundle: EttleX Product > Storage Spine (SQLite + CAS + Seed Import) > Snapshot Commit Pipeline (End-to-End) > Snapshot Commit Refactor — Action Commands as Canonical Mutation Ingress

## WHY (Rationale)

1. Establish the product-level framing for EttleX as a semantic evolution engine.
   This EP is non-load-bearing for the storage/commit milestones; it exists to keep the tree rooted in a
   recognisable product-level node for rendering and navigation.

2. Provide a durable substrate so semantic state can be anchored, reproduced, and evolved safely.

3. Snapshot commit is only meaningful if canonical state is durable and content-addressed.
   The storage spine is therefore the immediate prerequisite for snapshot commit and later diff/GC work.

4. The snapshot commit pipeline was originally implemented as an internal engine/store API.
   To support MCP as a thin transport over a single command layer, and to prevent split-brain
   behaviour between CLI/engine and MCP/actions paths, the commit operation must be refactored
   to use action:commands as the sole canonical mutation ingress. This EP anchors that refactor
   milestone in the tree.

5. EpUpdate is a canonical mutation and must be routed through the action layer (Apply) in the same way as all other canonical mutations. Adding EpUpdate to the Apply command enum ensures: all validation (ep_id exists, at least one field supplied) is enforced in the action layer; the store layer receives only validated, well-formed mutations; error taxonomy is consistent with all other commands; and the command is available to any transport layer (CLI, MCP) without each transport reimplementing validation logic.

## WHAT (Description)

1. Maintain a minimal product framing so leaf Ettles can be rendered in context.
   This EP does not impose implementation requirements on Phase 1/2 milestones.

2. Establish the platform foundations as refined milestones under this EP:

- Storage spine (SQLite + CAS + seed import)
- Snapshot commit pipeline (manifest + ledger anchor)

3. This Ettle is a structural anchor for the already-implemented Phase 1 Store Spine milestone.
   It represents the existence of:

- SQLite schema + migrations discipline (including facet_snapshots/provenance_events stubs)
- Filesystem CAS with atomic writes
- cas_blobs index population (non-load-bearing)
- Seed Format v0 importer

The normative implementation detail for this milestone is defined in the bootstrap markdown Ettle
“Phase 1 Store Spine (SQLite + CAS + Seed Import)”.

4. Structural anchor for the snapshot commit actions refactor milestone. Represents the
   existence of:

- Command::SnapshotCommit as the sole canonical mutation ingress
- Leaf-scoped selector (leaf_ep_id) with internal root_ettle_id derivation
- CLI re-wired to call action:commands rather than engine/store directly
- Module visibility or lint enforcement preventing direct store/engine calls

The normative implementation detail is defined in the snapshot commit actions refactor seed
(seed_snapshot_commit_actions_refactor_v3.yaml).

5. Add Command::EpUpdate to the action layer Apply enum.

Success criteria (binding):

1. Command::EpUpdate{ep_id, title?, why?, what?, how?} is a valid Apply variant.
2. The action layer validates that at least one optional field is supplied; rejects with typed error EmptyUpdate if none are.
3. The action layer validates that ep_id exists; rejects with typed error NotFound if absent.
4. On successful validation, the action layer delegates to the store layer for the SQL mutation.
5. The action layer does not perform the SQL mutation directly (store boundary preserved).
6. Error taxonomy is consistent with existing commands: NotFound, EmptyUpdate are typed errors.
7. All canonical mutations remain routed through Apply; no direct store/engine call paths are introduced.

## HOW (Implementation)

1. No scenarios. This EP is informational only.

2. Refinement only. Implementation scenarios live in child Ettles.

3. No new implementation scenarios here. The milestone is already delivered.
   This EP exists to:

- provide a stable parent refinement node for snapshot commit,
- preserve the dependency relationship in the refinement tree,
- and ensure rendered views show correct prerequisites.

4. No new implementation scenarios here. This EP exists to:

- provide a stable parent refinement node for ettle:snapshot_commit_actions_refactor,
- preserve the dependency relationship in the refinement tree,
- and ensure rendered views show the actions refactor as a child of the commit pipeline.

5. Scenarios (all MUST be implemented as tests; Gherkin is normative):

Feature: EpUpdate is routable via action layer Apply

Background:
Given a repository with SQLite + CAS store initialised
And at least one Ettle with one EP exists

Scenario: Apply EpUpdate succeeds with valid ep_id and at least one field
When I call action:commands::apply(Command::EpUpdate{ep_id="ep:store:0", why="updated"})
Then the command succeeds
And ep.get("ep:store:0").why equals "updated"

Scenario: Apply EpUpdate rejects empty update at action layer
When I call action:commands::apply(Command::EpUpdate{ep_id="ep:store:0"}) with no fields supplied
Then a typed error EmptyUpdate is returned
And no store mutation occurs

Scenario: Apply EpUpdate rejects unknown ep_id at action layer
When I call action:commands::apply(Command::EpUpdate{ep_id="ep:missing", why="x"})
Then a typed error NotFound is returned
And no store mutation occurs

Scenario: Apply EpUpdate does not mutate the store directly
Given I inspect the call graph during Command::EpUpdate execution
Then the action layer calls store::ep_update (or equivalent store trait method)
And the action layer does not issue SQL directly

Scenario: CLI ep-update delegates to action layer
When I invoke `ettlex ep update --ep-id <ep_id> --why "new why"`
Then the CLI calls action:commands::apply(Command::EpUpdate{ep_id=..., why="new why"})
And the CLI does not call store/engine ep_update directly
