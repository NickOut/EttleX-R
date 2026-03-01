# Snapshot Diff Engine (manifest-to-manifest diff)

## EP 0

**Normative**: Yes

**WHY**: A snapshot is a semantic anchor: it captures one closure (one EPT) as committed manifest bytes.
To evolve safely, the system must be able to compute an explicit, deterministic difference between
two snapshots.

A manifest-to-manifest diff engine provides:

- human-readable semantic change summaries for review and approval,
- machine-readable change sets to drive CIA-style impact reporting,
- deterministic evidence for whether an edit is a refactor vs a semantic modification,
- the foundation for later reachability, compatibility, and drift evaluators.

This engine must treat the manifest as the source of truth for a committed closure, and it must
remain robust as the manifest evolves additively.

**WHAT**: Implement a deterministic diff engine that compares two snapshot manifests and produces:

1. A structured diff (JSON) suitable for downstream evaluators.
2. A human-readable summary (Markdown/text) for review.

Inputs (minimum):

- manifest_a (bytes or parsed representation)
- manifest_b (bytes or parsed representation)

Output (minimum):

- diff schema version
- identity:
  - a.manifest_digest, a.semantic_manifest_digest, a.ept_digest
  - b.manifest_digest, b.semantic_manifest_digest, b.ept_digest
- change categories:
  - ept_changes
  - ep_content_changes
  - constraint_changes (family-agnostic)
  - coverage_changes
  - exception_changes
  - metadata_changes (policy_ref, profile_ref, store_schema_version, etc.)
- severity classification per change (none / informational / semantic / breaking)

Required properties (binding):

- Determinism: identical inputs produce byte-identical structured diff output.
- Created-at noise suppression: created_at differences MUST NOT be treated as semantic changes.
- Additive manifest compatibility: unknown future manifest fields MUST be ignored by default
  (reported as "unknown_changes" only when they change).
- Constraint-family agnosticism: diff MUST operate over the constraints envelope without
  requiring knowledge of specific families.

Dependency assumptions (pre-existing):

- snapshot_commit writes manifests to CAS and records manifest_digest + semantic_manifest_digest.
- constraint schema stubs exist and manifests contain a family-agnostic constraints envelope.

Out of scope (for this seed):

- Computing diffs directly from canonical state (DB)
- Merge/conflict resolution
- Automated gate decisions (policy/profile enforcement)

Constraint diff contract (family-agnostic, CORE-compatible):

The diff engine MUST treat the manifest as the canonical semantic boundary.
It MUST NOT traverse or diff the Ettle tree.

`constraint_changes` MUST be computed from the manifest constraints envelope as follows: - declared_ref_changes: additions/removals of constraints.declared_refs (set delta, plus ordered view) - family_changes: - per-family digest change if constraints.families[family].digest changes - per-family outcome deltas if the family section is parseable (otherwise opaque-bytes diff) - abb_sbb_projection_changes: - additions/removals in constraints.applicable_abb and constraints.resolved_sbb - evidence changes in constraints.resolution_evidence (byte-level or canonicalised-object diff) - constraints_digest_change: compare constraints.constraints_digest

The diff output MUST remain stable under additive manifest evolution: - unknown fields are ignored or surfaced under metadata_changes, but MUST NOT break diffing. - unknown constraint families are diffed opaquely via their family digest and presence.

**HOW**: Scenarios (all MUST be implemented as tests; unit/integration; Gherkin is normative):

Feature: Snapshot diff engine

Background:
Given a repository with SQLite + CAS store initialised
And at least two committed snapshots exist with their manifests available

# --- Identity and determinism ---

Scenario: Diff output is deterministic
Given manifest A bytes and manifest B bytes are fixed
When I compute diff(A,B) twice
Then the structured diff JSON bytes are identical
And the human summary is identical

Scenario: Diffing a manifest against itself yields no changes (degenerate fast-path)
Given manifest A is loaded once into memory
When I compute diff(A,A)
Then the diff severity is none
And ept_changes is empty
And ep_content_changes is empty
And constraint_changes is empty
And coverage_changes is empty
And exception_changes is empty
And metadata_changes is empty
And unknown_changes is empty
And the implementation MAY short-circuit by comparing semantic_manifest_digest

Scenario: Diff treats created_at as non-semantic
Given manifest A and manifest B differ only in created_at and manifest_digest
And semantic_manifest_digest is identical
When I compute diff(A,B)
Then the diff classification is "no_semantic_change"
And ept_changes is empty
And ep_content_changes is empty
And constraint_changes is empty

Scenario: Diff detects manifest evolution without breaking
Given manifest B contains additional unknown top-level fields not present in manifest A
When I compute diff(A,B)
Then the diff includes an "unknown_changes" section
And all known change categories are still computed correctly
And the diff does not fail

# --- EPT-level change detection ---

Scenario: Diff detects EPT change (closure changed)
Given manifest A has ept_digest "ept:1" and manifest B has ept_digest "ept:2"
When I compute diff(A,B)
Then ept_changes indicates "changed"
And severity is at least "semantic"
And the human summary lists added/removed/moved EP references

Scenario: Diff detects EP digest changes within same EPT structure
Given manifest A and B have identical EPT structure (same ordered EP ids)
But one EP digest differs
When I compute diff(A,B)
Then ep_content_changes lists that EP id
And severity is "semantic"

Scenario: Diff detects ordinal reordering as EPT change
Given manifest B reorders EPs within an Ettle without changing EP content
When I compute diff(A,B)
Then ept_changes is non-empty
And the summary calls out an ordering change

# --- Constraint diffs (family-agnostic) ---

Scenario: Diff detects addition of a constraint of unknown family
Given manifest A constraints.declared is empty
And manifest B constraints.declared includes constraint_id "c:vendor" family "vendor_ext"
When I compute diff(A,B)
Then constraint_changes.declared.added contains "c:vendor"
And the diff does not attempt to interpret the payload
And severity is "semantic" unless profile marks it informational

Scenario: Diff detects change in constraint payload via digest
Given manifest A contains constraint "c:1" with payload_digest "d1"
And manifest B contains constraint "c:1" with payload_digest "d2"
When I compute diff(A,B)
Then constraint_changes.payload_changed contains "c:1"
And the human summary shows "constraint payload changed" without parsing family content

Scenario: Diff maintains CORE ABB/SBB projection parity
Given manifests include both the family-agnostic constraints envelope
And the CORE ABB/SBB projection fields
When I compute diff(A,B)
Then constraint_changes are computed from the constraints envelope
And ABB/SBB fields are treated as redundant projections
And if ABB/SBB fields disagree with the envelope, the diff reports InvariantViolation

# --- Coverage and exceptions ---

Scenario: Diff detects coverage metric changes
Given manifest A coverage differs from manifest B coverage
When I compute diff(A,B)
Then coverage_changes is populated
And severity is "informational" unless policy marks it gating

Scenario: Diff detects exception list changes
Given manifest A exceptions is empty
And manifest B exceptions contains one exception
When I compute diff(A,B)
Then exception_changes.added contains that exception
And severity is at least "semantic"

# --- Metadata and governance references ---

Scenario: Diff detects policy_ref/profile_ref changes
Given manifest A policy_ref is "policy/x@1" and manifest B policy_ref is "policy/y@1"
When I compute diff(A,B)
Then metadata_changes includes policy_ref
And severity is "informational" unless policy requires re-approval

Scenario: Diff detects store schema version change
Given manifest A store_schema_version differs from manifest B store_schema_version
When I compute diff(A,B)
Then metadata_changes includes store_schema_version
And severity is "informational"

# --- Negative cases ---

Scenario: Diff rejects invalid manifest schema_version
Given manifest A has schema_version "not-an-int"
When I compute diff(A,B)
Then a typed error InvalidManifest is returned

Scenario: Diff rejects non-canonical JSON ordering in structured diff output
Given code changes that emit structured diff with non-deterministic key order
When I compute diff(A,B) twice
Then the determinism test fails
And the failure is reported as DeterminismViolation

Scenario: Diff fails fast on missing required digests
Given manifest A is missing semantic_manifest_digest
When I compute diff(A,B)
Then a typed error MissingField is returned

# --- Boundary conditions ---

Scenario: Diff handles large manifests efficiently
Given manifest A and B are each 5MB
When I compute diff(A,B)
Then diff completes within configured time budget
And peak memory usage remains within configured limits

Scenario: Diff handles identical manifests by returning a minimal diff
Given manifest A and manifest B are byte-identical
When I compute diff(A,B)
Then diff classification is "identical"
And all change categories are empty

Additional scenarios (constraints envelope + negative cases):

Scenario: Diff detects constraint reference additions/removals even when ABB/SBB projections are empty
Given manifest A constraints.declared_refs is empty
And manifest B constraints.declared_refs contains constraint/c1
And both manifests have empty applicable_abb/resolved_sbb
When I compute diff(A,B)
Then constraint_changes.declared_ref_changes.added contains constraint/c1
And constraint_changes.abb_sbb_projection_changes is empty
And overall severity is at least "semantic" (not "telemetry-only")

Scenario: Diff treats unknown constraint family outcomes as opaque and stable
Given manifest A has constraints.families["observability"].digest = D1
And manifest B has constraints.families["observability"].digest = D2
And the diff engine does not understand "observability"
When I compute diff(A,B)
Then constraint_changes.family_changes["observability"].digest_changed is true
And no parsing error occurs

Scenario: Diff ignores additive unknown manifest fields while still computing known category diffs
Given manifest B includes a new top-level field "new_field_x" not present in manifest A
When I compute diff(A,B)
Then diff computation succeeds
And metadata_changes includes "new_field_x"
And category diffs (ept/ep/constraints/coverage/exceptions) are still correct
