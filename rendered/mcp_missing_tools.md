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
transport over a complete command+query vocabulary. 6. ep:mcp_thin_slice:0 defines a normative tool surface in WHAT listing all required MCP tools, but the HOW provides Gherkin scenarios only for the tools that were implemented at time of authoring. The following tools are listed in WHAT but have no HOW scenario coverage: state.get_version, ep.list_children, ep.list_parents, ep.list_constraints, constraint.get, constraint.list_by_family, all decision queries (decision.get, decision.list, decision.list_by_target, ep.list_decisions, ettle.list_decisions, ept.compute_decision_context), manifest.get_by_digest, ept.compute, profile.resolve, approval.list. Additionally policy.export is absent from both WHAT and HOW entirely.

A prior attempt at this version (ordinal 1) incorrectly included PolicyCreate command vocabulary changes and SnapshotCommit policy_ref optionality in its WHAT. Those belong in ettle:store (ordinal 3) and ettle:snapshot_commit_actions_refactor (ordinal 1) respectively. This version corrects that overreach: it references those changes as dependencies but does not re-specify them.

Non-goals: re-specifying command vocabulary (owned by ettle:store); re-specifying SnapshotCommit policy_ref optionality (owned by ettle:snapshot_commit_actions_refactor); introducing new domain semantics beyond transport wiring.

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

6) All constraints from ep:mcp_thin_slice:0 remain in force. This EP adds the following.

1) policy.export added to normative MCP tool surface
   - MCP MUST expose policy.export(policy_ref, export_kind) as a read-only tool.
   - It MUST delegate to the action query PolicyExport without additional logic.
   - It MUST require policy_provider to be Some; if None it MUST return NotImplemented.
   - It MUST surface PolicyNotFound, PolicyExportFailed, PolicyExportTooLarge as stable error codes.
   - Output MUST be deterministic for identical inputs.

1) All listed-but-unimplemented tools from ep:mcp_thin_slice:0 MUST be wired
   The following tools MUST be implemented as transport-only wrappers over the corresponding action queries:
   - state.get_version
   - ep.list_children
   - ep.list_parents
   - ep.list_constraints
   - constraint.get
   - constraint.list_by_family (options: include_tombstoned?, limit?, cursor?)
   - decision.get
   - decision.list (options: limit?, cursor?)
   - decision.list_by_target (target_kind, target_id, include_tombstoned?)
   - ep.list_decisions (ep_id, include_ancestors?, status_filter?)
   - ettle.list_decisions (ettle_id, include_eps?, include_ancestors?)
   - ept.compute_decision_context (leaf_ep_id)
   - manifest.get_by_digest
   - ept.compute
   - profile.resolve
   - approval.list (options: limit?, cursor?)

1) Dependencies (not re-specified here)
   - PolicyCreate in Apply vocabulary: specified in ettle:store ordinal 3.
   - SnapshotCommit policy_ref optionality: specified in ettle:snapshot_commit_actions_refactor ordinal 1.
   - Both MUST be implemented before the full tool surface is exercisable.

1) Transport-only invariant applies to all new tools
   - Each tool MUST delegate to the corresponding action query with no additional filtering, projection, or semantic logic.
   - Output MUST be deterministic for identical action-layer outputs.
   - Each tool MUST surface the action layer's typed errors as stable MCP error codes.

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
Then a typed error InvalidInput (or PredicateTypeError) is returned 6. Scenarios (all MUST be implemented as tests; Gherkin is normative).
All scenarios from ep:mcp_thin_slice:0 remain in force.

Feature: MCP exposes remaining query tools as transport-only wrappers

Background:
Given the action layer is fully initialised with all query surfaces implemented
And MCP server is started in dev mode with a valid token "t:dev"
And at least one Ettle with EPs, constraints, decisions, and a committed snapshot exists

# --- Happy path ---

Scenario: state.get_version returns current state_version
When I call MCP state.get_version()
Then the response includes state_version as a non-negative integer
And result is deterministic under no-change

Scenario: ep.list_children returns child EPs for a parent EP
Given an EP has child Ettles pointing to it via parent_ep_id
When I call MCP ep.list_children(ep_id)
Then a list of child EPs is returned
And the result matches the action query output

Scenario: ep.list_parents returns the single parent EP
Given an EP has exactly one structural parent
When I call MCP ep.list_parents(ep_id)
Then exactly one parent EP is returned

Scenario: ep.list_constraints returns constraints ordered by ordinal
Given constraints are attached to an EP
When I call MCP ep.list_constraints(ep_id)
Then constraints are returned ordered by ep_constraint_refs.ordinal

Scenario: constraint.get returns the constraint record
Given constraint "c:1" exists
When I call MCP constraint.get("c:1")
Then the response includes id, family, and payload

Scenario: constraint.list_by_family returns non-tombstoned constraints by default
Given constraints exist in family "f:demo" including tombstoned ones
When I call MCP constraint.list_by_family("f:demo")
Then tombstoned constraints are excluded

Scenario: decision.get returns the decision record
Given decision "d:1" exists
When I call MCP decision.get("d:1")
Then the response includes id, title, status, and evidence summary

Scenario: decision.list returns paginated decisions
Given more than 100 decisions exist
When I call MCP decision.list(limit=50)
Then 50 decisions are returned with a cursor

Scenario: decision.list_by_target returns decisions linked to a target
Given decisions are linked to EP "ep:x"
When I call MCP decision.list_by_target(target_kind="ep", target_id="ep:x")
Then only decisions linked to ep:x are returned

Scenario: ep.list_decisions with include_ancestors returns ancestor decisions
Given a refinement chain ep:root -> ep:leaf with a decision on ep:root
When I call MCP ep.list_decisions("ep:leaf", include_ancestors=true)
Then the ancestor decision is returned

Scenario: ettle.list_decisions with include_eps returns EP-linked decisions
Given an Ettle has decisions linked to its EPs
When I call MCP ettle.list_decisions(ettle_id, include_eps=true)
Then EP-linked decisions are included in the response

Scenario: ept.compute_decision_context returns decisions across the EPT chain
Given a leaf EP with decisions across its EPT
When I call MCP ept.compute_decision_context(leaf_ep_id)
Then decisions for each EP in the chain are returned

Scenario: manifest.get_by_digest retrieves manifest bytes from CAS
Given a committed snapshot with manifest_digest D
When I call MCP manifest.get_by_digest(D)
Then manifest bytes are returned

Scenario: ept.compute returns ordered EP list and digest
Given a valid leaf EP
When I call MCP ept.compute(leaf_ep_id)
Then ept_ep_ids and ept_digest are returned

Scenario: profile.resolve with explicit ref returns that profile
Given profile "dev@0" exists
When I call MCP profile.resolve("dev@0")
Then profile_ref, profile_digest, and parsed_profile are returned

Scenario: profile.resolve with null resolves to default profile
Given a default profile is configured
When I call MCP profile.resolve(null)
Then the default profile is returned

Scenario: approval.list returns paginated approvals in deterministic order
Given more than 100 approval requests exist
When I call MCP approval.list(limit=50)
Then 50 approvals are returned ordered by created_at ASC, approval_token ASC
And a cursor is returned

Scenario: policy.export returns HANDOFF block content with markers stripped
Given policy "codegen_handoff@1" exists with well-formed HANDOFF markers
When I call MCP policy.export(policy_ref="codegen_handoff@1", export_kind="codegen_handoff")
Then the result text contains HANDOFF block content
And no HANDOFF: START or HANDOFF: END markers appear in the result

# --- Negative cases ---

Scenario: ep.list_children returns empty list for a leaf EP
Given a leaf EP with no child Ettles
When I call MCP ep.list_children(leaf_ep_id)
Then an empty list is returned without error

Scenario: constraint.get surfaces NotFound for unknown id
When I call MCP constraint.get("c:missing")
Then error_code NotFound is returned

Scenario: decision.get surfaces NotFound for unknown id
When I call MCP decision.get("d:missing")
Then error_code NotFound is returned

Scenario: manifest.get_by_digest surfaces MissingBlob for unknown digest
When I call MCP manifest.get_by_digest("deadbeef")
Then error_code MissingBlob is returned

Scenario: ept.compute surfaces NotFound for unknown EP
When I call MCP ept.compute("ep:missing")
Then error_code NotFound is returned

Scenario: profile.resolve surfaces ProfileNotFound for unknown ref
When I call MCP profile.resolve("missing@0")
Then error_code ProfileNotFound is returned

Scenario: policy.export surfaces PolicyNotFound for unknown policy_ref
When I call MCP policy.export(policy_ref="nonexistent@0", export_kind="codegen_handoff")
Then error_code PolicyNotFound is returned

Scenario: policy.export surfaces PolicyExportFailed for unknown export_kind
When I call MCP policy.export(policy_ref="codegen_handoff@1", export_kind="unknown")
Then error_code PolicyExportFailed is returned

# --- Explicit error paths ---

Scenario: ep.list_parents surfaces RefinementIntegrityViolation on multi-parent corruption
Given an EP has two structural parents due to corrupted state
When I call MCP ep.list_parents(ep_id)
Then error_code RefinementIntegrityViolation is returned

Scenario: ept.compute surfaces EptAmbiguous on ambiguous refinement graph
Given the refinement graph is ambiguous
When I call MCP ept.compute(leaf_ep_id)
Then error_code EptAmbiguous is returned

Scenario: constraint.list_by_family with include_tombstoned=true returns tombstoned entries
Given constraints in family "f:demo" include tombstoned ones
When I call MCP constraint.list_by_family("f:demo", include_tombstoned=true)
Then tombstoned constraints are included with tombstone flags

# --- Boundary conditions ---

Scenario: approval.list with no approvals returns empty list without error
Given no approval requests exist
When I call MCP approval.list()
Then an empty list is returned
And no cursor is returned

Scenario: decision.list with limit=1 returns exactly one result and a cursor
Given more than one decision exists
When I call MCP decision.list(limit=1)
Then exactly one decision is returned
And a cursor is returned

Scenario: policy.export surfaces PolicyExportTooLarge when result exceeds byte limit
Given a policy with HANDOFF blocks exceeding the configured byte limit
When I call MCP policy.export for that policy
Then error_code PolicyExportTooLarge is returned

# --- Invariants ---

Scenario: All new query tools do not mutate canonical state
Given state_version is V
When I call each new query tool once (state.get_version, ep.list_children, ep.list_parents, ep.list_constraints, constraint.get, constraint.list_by_family, decision.get, decision.list, decision.list_by_target, ep.list_decisions, ettle.list_decisions, ept.compute_decision_context, manifest.get_by_digest, ept.compute, profile.resolve, approval.list, policy.export)
Then state_version remains V
And no ledger rows are appended
And no CAS blobs are written

Scenario: All new tools are pure transport — output matches action query output exactly
When I call MCP constraint.get("c:1")
And I call action query constraint.get("c:1") directly
Then the JSON bytes are identical after canonical serialization

# --- Idempotency / repeatability ---

Scenario: All new query tools are idempotent under no state change
When I call each new query tool twice with identical inputs and no intervening mutations
Then both responses are identical for each tool

# --- Determinism / ordering ---

Scenario: state.get_version increments by exactly 1 after each mutation
Given state_version is V
When I call any Apply command successfully
Then state.get_version() returns V+1

Scenario: ept.compute returns byte-identical output for identical state
When I call MCP ept.compute(leaf_ep_id) twice without state change
Then both responses are byte-identical after canonical JSON serialization

Scenario: ept.compute_decision_context returns byte-identical output for identical state
When I call MCP ept.compute_decision_context(leaf_ep_id) twice without state change
Then both responses are byte-identical after canonical JSON serialization

Scenario: policy.export is deterministic
When I call MCP policy.export("codegen_handoff@1", "codegen_handoff") twice
Then both results are byte-identical

Scenario: approval.list ordering is deterministic
When I call MCP approval.list() twice without state change
Then both responses are byte-identical after canonical JSON serialization

# --- State transitions ---

Scenario: state.get_version reflects state after PolicyCreate
Given state_version is V
When PolicyCreate succeeds
Then state.get_version() returns V+1

Scenario: decision.list reflects newly created decision
When I create a decision via ettlex.apply
Then decision.list() includes that decision

# --- Concurrency ---

# Query tools are read-only; no concurrency conflicts possible. Concurrent reads return consistent snapshots of canonical state.

# --- Security / authorisation ---

# Auth enforcement for all tools is governed by ep:mcp_thin_slice:0. No tool-specific auth beyond the transport layer.

# --- Observability ---

Scenario: state.get_version is usable as an OCC guard
Given state_version is V
When I call ettlex.apply with expected_state_version=V and a valid command
Then the command succeeds and state.get_version() returns V+1
When I call ettlex.apply with expected_state_version=V again
Then error_code HeadMismatch is returned

# --- Compatibility / migration ---

Scenario: Tools introduced in this EP do not affect behaviour of tools from ep:mcp_thin_slice:0
Given the system is running with all tools from ep:mcp_thin_slice:0 operational
When the new tools from this EP are added
Then ettle.get, ettle.list, ep.get, snapshot.diff, profile.get, approval.get and all other existing tools continue to return identical outputs

# --- Resource / performance ---

Scenario: ept.compute_decision_context completes within time budget for large EPTs
Given a leaf EP whose EPT chain contains 1000 EPs each with linked decisions
When I call MCP ept.compute_decision_context(leaf_ep_id)
Then the query completes within the configured time budget

# --- Explicit prohibition ---

Scenario: New query tools MUST NOT perform domain logic beyond delegation
When I call ep.list_children(ep_id)
Then MCP does not filter, sort, or project beyond what the action query returns
And the result is identical to the direct action query output

Scenario: policy.export MUST NOT return raw policy text — only extracted HANDOFF content
When I call MCP policy.export("codegen_handoff@1", "codegen_handoff")
Then the result does not include content outside HANDOFF markers
And it does not include the full raw policy document

# --- Byte-level equivalence ---

Scenario: MCP canonical JSON serialization is stable for all new tools
When I call each new tool twice with identical inputs and no state change
Then raw JSON bytes are identical for each call pair

# --- Concurrency conflict ---

# Not applicable to read-only tools. Concurrent reads are safe by design.
