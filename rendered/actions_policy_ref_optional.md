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

5. ep:snapshot_commit_actions_refactor:0 already specifies policy_ref as optional (policy_ref?) in the SnapshotCommit command signature. The implementation deviates from this: policy_ref is unconditionally required and rejects absent values with InvalidInput.

This deviation was surfaced by exercising the MCP governance sequence (step 1: snapshot commit baseline). The sequence failed because no policy existed in the store and PolicyCreate was absent from the command vocabulary. Even after ProfileCreate was added, SnapshotCommit remained unreachable.

This EP makes the optionality of policy_ref binding and specifies the deterministic defaulting behaviour when it is omitted. It does not introduce a default policy; it defines what the action layer does in the absence of one.

Non-goals: changing profile_ref optionality; altering manifest content obligations; introducing policy evaluation semantics.

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

5. 1. policy_ref on SnapshotCommit is optional
   - The SnapshotCommit command MUST accept an absent or null policy_ref without error.
   - When policy_ref is absent, the action layer MUST apply deterministic defaulting:
     a) If a default policy exists in the policy provider, use it.
     b) If no default policy exists, proceed as permissive pass-through (no policy check applied).
   - The action layer owns defaulting. The MCP transport MUST NOT inject a policy_ref value.
   - The resolved policy_ref (or empty string if none) MUST still be recorded in the snapshot manifest.

6. Binding invariants
   - A SnapshotCommit with absent policy_ref and no default policy MUST succeed (permissive pass-through).
   - A SnapshotCommit with absent policy_ref and a configured default policy MUST apply that policy.
   - A SnapshotCommit with an explicit policy_ref MUST use exactly that policy regardless of defaults.
   - The manifest policy_ref field MUST reflect the resolved value, not the input value (i.e. if defaulting resolved to X, manifest records X; if permissive pass-through, manifest records empty string or null).
   - MCP MUST forward absent policy_ref as absent to the action layer; MCP MUST NOT substitute a value.

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

Feature: SnapshotCommit policy_ref is optional with deterministic defaulting

Background:
Given a repository with SQLite + CAS store initialised
And a valid leaf EP exists
And profile "dev@0" exists

# --- Happy path ---

Scenario: SnapshotCommit succeeds with policy_ref absent and no default policy
Given no default policy is configured
When I call ettlex.apply with Command::SnapshotCommit{leaf_ep_id=<leaf>, profile_ref="dev@0"} omitting policy_ref
Then the snapshot is committed successfully
And a snapshot_id is returned
And the manifest is written to CAS
And the manifest policy_ref field is empty string or null

Scenario: SnapshotCommit succeeds with explicit policy_ref
Given policy "codegen_handoff@1" exists
When I call ettlex.apply with Command::SnapshotCommit{leaf_ep_id=<leaf>, policy_ref="codegen_handoff@1", profile_ref="dev@0"}
Then the snapshot is committed successfully
And the manifest records policy_ref as "codegen_handoff@1"

Scenario: SnapshotCommit with absent policy_ref uses default policy when one is configured
Given policy "default_policy@0" is configured as the default
When I call ettlex.apply with Command::SnapshotCommit{leaf_ep_id=<leaf>, profile_ref="dev@0"} omitting policy_ref
Then the action layer applies "default_policy@0"
And the manifest records the resolved policy_ref

# --- Negative cases ---

Scenario: SnapshotCommit with explicit policy_ref that does not exist fails
When I call ettlex.apply with Command::SnapshotCommit{leaf_ep_id=<leaf>, policy_ref="missing@0", profile_ref="dev@0"}
Then error_code PolicyNotFound is returned
And no snapshot is committed

# --- Explicit error paths ---

Scenario: SnapshotCommit with absent policy_ref and a default policy that denies commit fails with PolicyDenied
Given policy "deny@0" is configured as the default and denies all commits
When I call ettlex.apply with Command::SnapshotCommit{leaf_ep_id=<leaf>, profile_ref="dev@0"} omitting policy_ref
Then error_code PolicyDenied is returned
And no snapshot is committed
And no ledger row is appended

# --- Boundary conditions ---

Scenario: SnapshotCommit with null policy_ref field (explicit null) behaves identically to absent
When I call ettlex.apply with Command::SnapshotCommit{leaf_ep_id=<leaf>, policy_ref=null, profile_ref="dev@0"}
Then the snapshot is committed successfully (permissive pass-through if no default)
And no InvalidInput error is returned

# --- Invariants ---

Scenario: Manifest always records a policy_ref field regardless of input
When I call SnapshotCommit with absent policy_ref
Then the committed manifest contains a policy_ref field
And the field value is either the resolved policy ref or empty string
And the field is never absent from the manifest structure

Scenario: Explicit policy_ref always takes precedence over default
Given "default_policy@0" is configured as default
And "explicit_policy@0" also exists
When I call SnapshotCommit with policy_ref="explicit_policy@0"
Then the manifest records "explicit_policy@0"
And "default_policy@0" is not applied

# --- Idempotency ---

# Not applicable to optionality semantics specifically; SnapshotCommit idempotency is covered in ep:snapshot_commit:0.

# --- Determinism ---

Scenario: Absent policy_ref defaulting is deterministic
Given a fixed default policy and identical canonical state
When I call SnapshotCommit twice with absent policy_ref
Then semantic_manifest_digest is identical between the two commits
And both manifests record the same resolved policy_ref

# --- State transitions ---

Scenario: SnapshotCommit with absent policy_ref transitions from no-snapshot to committed state
Given no snapshots exist for the leaf EP
When I call SnapshotCommit with absent policy_ref
Then snapshot.get_head(ettle_id) returns a non-null manifest_digest
And the snapshot is in committed status

# --- Concurrency ---

# Covered by optimistic concurrency scenarios in ep:snapshot_commit:0. No additional concurrency obligations specific to policy_ref optionality.

# --- Security / authorisation ---

# MCP auth enforcement is governed by ep:mcp_thin_slice. Policy-level access control is governed by the policy evaluation engine.

# --- Observability ---

Scenario: SnapshotCommit with absent policy_ref emits an observable result distinguishable from policy-governed commit
Given no default policy is configured
When I call SnapshotCommit with absent policy_ref
Then the result indicates permissive pass-through (e.g. policy_ref is empty in manifest)
And the commit result tag is SnapshotCommitted

# --- Compatibility / migration ---

Scenario: Existing SnapshotCommit calls that supply policy_ref continue to work unchanged
Given policy "existing@0" exists
When I call SnapshotCommit with policy_ref="existing@0"
Then the snapshot is committed as before
And manifest records policy_ref="existing@0"

# --- Resource / performance ---

# No additional obligations beyond ep:snapshot_commit:0.

# --- Explicit prohibition ---

Scenario: MCP MUST NOT inject a policy_ref into SnapshotCommit
Given no default policy exists
When I call MCP ettlex.apply with SnapshotCommit omitting policy_ref
Then the action layer receives policy_ref as absent (not a MCP-supplied default)
And the MCP request payload does not contain a policy_ref field

# --- Byte-level equivalence ---

Scenario: Manifest bytes from absent policy_ref commit are byte-stable under identical state
Given identical canonical state and no default policy
When I call SnapshotCommit twice with absent policy_ref
Then semantic_manifest_digest is byte-identical between the two commits

# --- Concurrency conflict ---

# Covered by ep:snapshot_commit:0 expected_head scenarios.
