# Leaf Bundle: EttleX Product > Storage Spine (SQLite + CAS + Seed Import) > Snapshot Commit Pipeline (End-to-End) > Action Read Tools — Canonical Queries for Authoring, Inspection, Diff, and Decision Context > MCP Thin Slice (Transport-Only; Apply + Queries; Authoring-Grade)

## WHY (Rationale)

1. Establish the product-level framing for EttleX as a semantic evolution engine.
   This EP is non-load-bearing for the storage/commit milestones; it exists to keep the tree rooted in a
   recognisable product-level node for rendering and navigation.

2. Provide a durable substrate so semantic state can be anchored, reproduced, and evolved safely.

3. Snapshot commit is only meaningful if canonical state is durable and content-addressed.
   The storage spine is therefore the immediate prerequisite for snapshot commit and later diff/GC work.

4. The action command layer is only viable as a complete interface if agents and MCP/CLI can
   also observe canonical state and derived projections through stable query surfaces. Read
   tools are the query complement to the command layer: without them, authoring is blind and
   seeds remain the fallback inspection mechanism. This EP anchors the action read tools
   milestone as a distinct child of the commit pipeline.

5. Apply-only mutation is viable only if the agent (and MCP/CLI) can observe canonical state and derived
   projections through stable query surfaces. Without read tools, authoring becomes blind and seeds become the
   fallback.

This seed defines the minimal action:query/read surfaces required so that MCP can be generated as a thin
transport over a complete command+query vocabulary. 6. The MCP tool surface must expose ep.update so that the EpUpdate command is accessible to MCP clients without those clients needing to know the action layer internals. Consistent with the thin transport principle: the MCP tool does no validation or business logic; it maps tool call parameters to Command::EpUpdate and delegates to the action layer.

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

4. Structural anchor for the action read tools milestone. Represents the existence of:

- Deterministic query surfaces for Ettles, EPs, constraints, decisions, snapshots,
  manifests, EPT projections, and snapshot diff
- Pagination and filtering support for large-tree scale
- Decision context queries (non-snapshot-semantic)
- MCP-ready thin transport wiring over the same action:query surface

The normative implementation detail is defined in the action read tools seed
(seed_action_read_tools_v3_rewrite.yaml).

5. Implement a set of read/query operations (non-mutating) exposed from the same application layer as
   action commands.

Binding rules:

- Read tools MUST NOT mutate canonical state.
- Read tools MAY read canonical DB state.
- Projection computations (EPT, manifest load, diff compute) MUST be deterministic for identical inputs.
- MCP (when added) MUST wrap these query surfaces directly (thin transport).

Minimum query/tool set:

1. State identity / version
   - state.get_version() -> { state_version, semantic_head_digest? }

2. Ettle queries
   - ettle.get(ettle_id)
   - ettle.list(options?)
   - ettle.list_eps(ettle_id)

   ettle.list(options?) MUST support scale-safe enumeration:
   - options MAY include: prefix_filter?, title_contains?, limit?, cursor?
   - The implementation MUST enforce a default limit if limit is omitted.
   - The implementation MUST support cursor-based pagination (opaque cursor preferred).
   - Ordering MUST be deterministic (stable sort key: ettle_id ascending, unless overridden by an explicit sort option).

3. EP queries
   - ep.get(ep_id)
   - ep.list_children(ep_id)
   - ep.list_parents(ep_id)

   ep.list_parents(ep_id) MUST reflect the refinement invariants:
   - Under the one-parent constraint, an EP MUST have at most one parent in the refinement graph.
   - If data corruption or legacy state produces multiple parents, the query MUST NOT silently pick one:
     - it MUST return a typed error RefinementIntegrityViolation and include the conflicting parent ids.

4. Constraint queries
   - constraint.get(constraint_id)
   - constraint.list_by_family(family)
   - ep.list_constraints(ep_id)

4.1) Decision queries (governance; non-snapshot-semantic)

- decision.get(decision_id)
- decision.list(options?)
- decision.list_by_target(target_kind, target_id, include_tombstoned?)
- ep.list_decisions(ep_id, include_ancestors?, status_filter?)
- ettle.list_decisions(ettle_id, include_eps?, include_ancestors?)
- ept.compute_decision_context(leaf_ep_id, status_filter?)

Decision query invariants: - Decision queries MUST NOT affect snapshot semantics. - Decision queries MUST NOT mutate canonical state. - Ancestor enumeration MUST respect refinement invariants. - Deterministic ordering rules MUST be enforced.

5. Snapshot/manifest queries
   - snapshot.get(snapshot_id)
   - snapshot.list(ettle_id? leaf_ep_id?)
   - manifest.get_by_snapshot(snapshot_id) -> bytes + digests
   - manifest.get_by_digest(manifest_digest) -> bytes

6. Projection queries
   - ept.compute(leaf_ep_id) -> ordered EP ids + digests (or a stable projection)
   - snapshot.diff(a_ref, b_ref) -> { structured_diff_json, human_summary }
     (read-only; MUST operate on manifest bytes; MUST NOT read canonical state for semantic comparison)

7. Profile and approval queries (Phase 1: profile handling + route_for_approval support)
   - profile.get(profile_ref) -> { profile_ref, profile_digest, payload_json, metadata? }
   - profile.resolve(profile_ref?) -> { profile_ref, profile_digest, parsed_profile }
   - profile.get_default() -> { profile_ref, profile_digest? }
   - profile.list(options?) -> stable ordered list (MUST enforce default limit, MUST support cursor)

   - approval.get(approval_token) -> { approval_token, request_digest, semantic_request_digest, payload_json }
   - approval.list(options?) -> stable ordered list (MUST enforce default limit, MUST support cursor)
   - approval.list_by_kind(kind, options?) -> stable ordered list (optional; if omitted MUST return a typed NotImplemented error)

   Binding rules for profile/approval read tools:
   - These tools MUST NOT mutate canonical state.
   - These tools MUST be satisfied by the canonical CAS+SQLite state (no hidden caches that break determinism).
   - Canonical JSON output MUST be stable (key order + numeric representation) for identical canonical inputs.
   - profile.resolve(profile_ref=null) MUST follow deterministic defaulting rules defined by Profiles Core.
   - approval.get MUST fail with ApprovalNotFound for unknown token.
   - approval.get MUST fail with ApprovalStorageCorrupt (or StorageError) if SQLite index exists but CAS blob is missing.
   - Missing cas_blobs index rows MUST NOT prevent successful reads if CAS contains the referenced blobs.

8. Constraint predicate evaluation (read-mode / preview; optional but recommended for authoring)
   - constraint_predicates.preview(profile_ref?, context, candidates) -> ResolutionResult
     (read-only; MUST NOT create approval requests; MUST be deterministic; MUST be suitable for dry-run inspection)

   If constraint_predicates.preview is NOT implemented in Phase 1:
   - the action layer MUST still expose enough to allow agents to reason about constraint ambiguity before commit,
     either via a dry_run SnapshotCommit output or by returning a typed NotImplemented from preview.

6) Add ep.update as an MCP tool, as a thin transport wrapper over Command::EpUpdate.

Success criteria (binding):

1. ep.update is a declared MCP tool with parameters: ep_id (required), title (optional), why (optional), what (optional), how (optional).
2. The tool maps parameters directly to Command::EpUpdate and calls action:commands::apply.
3. No validation logic is implemented in the MCP tool itself; all validation is deferred to the action layer.
4. On success the tool returns ep_id in the response payload.
5. Typed errors from the action layer (EmptyUpdate, NotFound) are surfaced as structured MCP error responses.
6. The tool is consistent with the thin transport pattern established for all other MCP tools in this ettle.

## HOW (Implementation)

1. No scenarios. This EP is informational only.

2. Refinement only. Implementation scenarios live in child Ettles.

3. No new implementation scenarios here. The milestone is already delivered.
   This EP exists to:

- provide a stable parent refinement node for snapshot commit,
- preserve the dependency relationship in the refinement tree,
- and ensure rendered views show correct prerequisites.

4. No new implementation scenarios here. This EP exists to:

- provide a stable parent refinement node for ettle:action_read_tools,
- keep read tools as a distinct milestone from the actions refactor (ep:snapshot_commit:1),
- and ensure rendered views show the full command+query vocabulary as children of the
  commit pipeline.

5. Scenarios (all MUST be implemented as tests; Gherkin is normative):

Feature: Action read tools provide deterministic inspection surfaces

Background:
Given a repository with SQLite + CAS store initialised
And at least one Ettle exists with EPs and links
And at least one snapshot has been committed

# --- Non-mutation invariant ---

Scenario: Read tools never mutate canonical state
Given state_version is V1
When I call all read tools once
Then state_version remains V1
And no ledger rows are appended
And no CAS blobs are written

# --- Ettle/EP queries ---

Scenario: ettle.get returns complete metadata and EP membership
When I call ettle.get(ettle_id)
Then the response includes id, title, and metadata
And the response includes the list of EP ids belonging to the Ettle

Scenario: ettle.list enforces a default limit
Given there are more than 200 Ettles in the store
When I call ettle.list() with no options
Then at most the default limit of results is returned
And a cursor is returned if additional results exist
And the returned ordering is deterministic

Scenario: ettle.list supports cursor-based pagination deterministically
Given there are more than 500 Ettles in the store
When I call ettle.list(limit=100)
Then I receive page_1 with 100 Ettles and a cursor_1
When I call ettle.list(limit=100, cursor=cursor_1)
Then I receive page_2 with 100 Ettles and a cursor_2
And page_1 and page_2 contain no duplicates
And concatenating pages yields the same ordering as a single full enumeration would (conceptually)
And repeating the same calls returns identical pages (deterministic)

Scenario: ettle.list supports filtering without breaking determinism
Given there exist Ettles with ids starting with "ettle:a:" and "ettle:b:"
When I call ettle.list(prefix_filter="ettle:a:", limit=100)
Then all returned Ettles have ids starting with "ettle:a:"
And ordering is deterministic

Scenario: ep.list_children returns deterministic ordering
Given an EP has multiple children
When I call ep.list_children twice
Then the returned child list is identical
And ordering matches the canonical ordering rule for refine links

Scenario: ep.list_parents returns the single parent under the refinement invariant
Given a child EP is linked under exactly one parent EP
When I call ep.list_parents(child_ep)
Then exactly one parent EP id is returned
And the result is deterministic

Scenario: ep.list_parents rejects multiple-parent corruption
Given a child EP is linked under two parent EPs due to corrupted or legacy state
When I call ep.list_parents(child_ep)
Then a typed error RefinementIntegrityViolation is returned
And the error includes both parent EP ids
And the query does not silently choose one

# --- Constraint queries ---

Scenario: constraint.list_by_family returns only non-tombstoned by default
Given constraints exist in family "f:demo" including tombstoned ones
When I call constraint.list_by_family("f:demo") without options
Then tombstoned constraints are excluded
When I call with include_tombstoned=true
Then tombstoned constraints are included with tombstone flags

Scenario: ep.list_constraints is deterministic and ordered
Given multiple constraints are attached to an EP
When I call ep.list_constraints twice
Then the results are identical and ordered deterministically

# --- Manifest and snapshot queries ---

Scenario: manifest.get_by_snapshot returns recorded digests and bytes
When I call manifest.get_by_snapshot(snapshot_id)
Then I receive manifest bytes
And I receive manifest_digest and semantic_manifest_digest
And semantic_manifest_digest matches digest computed with created_at excluded

Scenario: manifest.get_by_digest rejects unknown digest
When I call manifest.get_by_digest("nope")
Then a typed error NotFound is returned

# --- EPT compute query ---

Scenario: ept.compute returns deterministic EPT
When I call ept.compute(leaf_ep_id) twice
Then the ordered EP list is identical
And the returned ept_digest is identical

Scenario: ept.compute fails fast on ambiguity
Given the refinement graph is ambiguous
When I call ept.compute(leaf_ep_id)
Then a typed error EptAmbiguous is returned

# --- Diff query binding ---

Scenario: snapshot.diff operates only on manifest bytes
Given I have snapshot A and snapshot B
When I call snapshot.diff(snapshot_id(A), snapshot_id(B))
Then the implementation resolves both to manifest bytes
And diff output is produced without reading canonical DB state for semantic comparison

Scenario: snapshot.diff rejects missing manifest
Given snapshot A references a missing CAS manifest blob
When I call snapshot.diff(A,B)
Then a typed error StorageError is returned

Scenario: snapshot.diff output is deterministic
When I call snapshot.diff(A,B) twice with identical manifest bytes
Then structured diff bytes are identical

# --- Decision queries (non-snapshot-semantic) ---

Scenario: decision.list is deterministic
Given multiple decisions exist
When I call decision.list() twice
Then both results are byte-identical after canonical serialization

Scenario: ep.list_decisions includes ancestor decisions when requested
Given a refinement chain ep:root -> ep:leaf exists
And decision "d:1" is linked to ep:root
When I call ep.list_decisions(ep:leaf, include_ancestors=true)
Then decision "d:1" is returned

Scenario: ept.compute_decision_context returns deterministic structure
Given a leaf EP with decisions across its EPT
When I call ept.compute_decision_context(leaf_ep_id) twice
Then the returned structure is identical

Scenario: Decision queries do not alter snapshot semantics
Given a committed snapshot S1
And decision state changes but no EP or constraint changes occur
When I call snapshot.diff(S1,S1)
Then diff classification remains "identical"

# --- Boundary conditions ---

Scenario: Read queries scale to large trees
Given an Ettle contains 10,000 EPs
When I call ettle.list_eps and ep.list_children repeatedly
Then each call completes within configured time budget
And memory usage remains within configured limits

Feature: Profile and approval queries provide deterministic inspection surfaces

Background:
Given a repository with SQLite + CAS store initialised
And profile "profile/default@0" exists
And at least one approval request exists with token T1 (status pending)

Scenario: profile.get returns digest and payload and is deterministic
When I call profile.get("profile/default@0") twice
Then both responses are byte-identical after canonical serialization
And both include profile_digest and payload_json

Scenario: profile.resolve(null) uses deterministic default profile
Given repo config default_profile_ref is not set
When I call profile.resolve(null)
Then it resolves to "profile/default@0"

Scenario: profile.resolve rejects unknown profile_ref
When I call profile.resolve("profile/missing@0")
Then a typed error ProfileNotFound is returned

Scenario: profile.list enforces default limit and supports cursor pagination deterministically
Given there are more than 300 profiles stored
When I call profile.list() with no options
Then at most the default limit is returned
And a cursor is returned if additional results exist
When I call profile.list(limit=100, cursor=that cursor)
Then I receive the next page without duplicates
And repeating the same calls returns identical pages

Scenario: approval.get returns recorded digests and bytes
When I call approval.get(T1)
Then I receive payload_json
And I receive request_digest and semantic_request_digest
And semantic_request_digest matches digest computed with created_at excluded

Scenario: approval.get rejects unknown token
When I call approval.get("approval:missing")
Then a typed error ApprovalNotFound is returned

Scenario: approval.get fails when SQLite row exists but CAS blob is missing
Given SQLite has an approval_requests row for token T2 referencing digest D2
And CAS does not contain digest D2
When I call approval.get(T2)
Then a typed error ApprovalStorageCorrupt (or StorageError) is returned

Scenario: approval.list enforces default limit and deterministic ordering
Given there are more than 300 approval requests stored
When I call approval.list() with no options
Then at most the default limit is returned
And ordering is deterministic
And repeated calls return identical output

Feature: Predicate preview is read-only and deterministic

Background:
Given profile "profile/default@0" exists
And candidates include two eligible nodes A and B with equal priority
And context is { env: "prod" }

Scenario: constraint_predicates.preview does not create approval requests even when route_for_approval would apply
Given profile/default@0 has ambiguity_policy route_for_approval
And approval request count is N
When I call constraint_predicates.preview(profile_ref="profile/default@0", context, candidates)
Then a ResolutionResult is returned (selected/no_match/ambiguous/routed_for_approval depending on semantics)
And approval request count remains N

Scenario: constraint_predicates.preview is deterministic
When I call constraint_predicates.preview twice with identical inputs
Then the returned ResolutionResult is byte-identical after canonical serialization

Scenario: constraint_predicates.preview rejects invalid context deterministically
Given context has an unsupported type (e.g., array)
When I call constraint_predicates.preview
Then a typed error InvalidInput (or PredicateTypeError) is returned 6. Scenarios (all MUST be implemented as tests; Gherkin is normative):

Feature: ep.update MCP tool delegates to action layer

Background:
Given a repository with SQLite + CAS store initialised
And at least one Ettle with one EP exists
And the MCP server is running

Scenario: ep.update succeeds and returns ep_id
When I call MCP tool ep.update with {ep_id="ep:store:0", why="via mcp"}
Then the tool succeeds
And ep.get("ep:store:0").why equals "via mcp"
And the response payload contains ep_id "ep:store:0"

Scenario: ep.update with no optional fields returns structured error
When I call MCP tool ep.update with {ep_id="ep:store:0"} and no other fields
Then the tool returns a structured error with code EmptyUpdate

Scenario: ep.update with unknown ep_id returns structured error
When I call MCP tool ep.update with {ep_id="ep:missing", why="x"}
Then the tool returns a structured error with code NotFound

Scenario: ep.update performs no validation logic itself
Given I inspect the MCP tool implementation
Then the tool contains no field presence checks or ep_id existence checks
And all such validation is performed by the action layer
