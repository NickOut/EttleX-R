# Leaf Bundle: EttleX Product > Storage Spine (SQLite + CAS + Seed Import) > Snapshot Commit Pipeline (End-to-End) > Snapshot Diff Engine (manifest-to-manifest diff)

## WHY (Rationale)

1. Establish the product-level framing for EttleX as a semantic evolution engine.
   This EP is non-load-bearing for the storage/commit milestones; it exists to keep the tree rooted in a
   recognisable product-level node for rendering and navigation.

2. Provide a durable substrate so semantic state can be anchored, reproduced, and evolved safely.

3. Snapshot commit is only meaningful if canonical state is durable and content-addressed.
   The storage spine is therefore the immediate prerequisite for snapshot commit and later diff/GC work.

4. Provide immutable semantic anchors by committing canonical state into an append-only ledger.
   Enable reliable reproducible diffs and downstream TES generation by persisting a manifest in CAS and
   anchoring it in the snapshot ledger in a single transaction boundary.

This is the first point at which the system becomes self-hosting: a stable commit forms a durable
reference that can be used to reproduce EPT, validate invariants, and generate projections.

Dependency: this Ettle assumes the Storage Spine exists (SQLite schema + CAS + seed import) and is healthy.

5. A snapshot is a semantic anchor: it captures one closure (one EPT) as committed manifest bytes.
   To evolve safely, the system must be able to compute an explicit, deterministic difference between
   two snapshots.

A manifest-to-manifest diff engine provides:

- human-readable semantic change summaries for review and approval,
- machine-readable change sets to drive CIA-style impact reporting,
- deterministic evidence for whether an edit is a refactor vs a semantic modification,
- the foundation for later reachability, compatibility, and drift evaluators.

This engine must treat the manifest as the source of truth for a committed closure, and it must
remain robust as the manifest evolves additively.

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

4. Implement snapshot commit as a single, atomic operation:

compute EPT + digests
→ build snapshot manifest (structured JSON)
→ write manifest to CAS (digest-addressed)
→ append ledger entry referencing manifest_digest

Preconditions (normative):

- SQLite schema is migrated to head.
- facet_snapshots table exists.
- CAS store is available and supports atomic writes.
- Canonical Ettle/EP state exists and EPT computation is deterministic (Phase 0.5).

The commit flow MUST be correct under concurrency, MUST be deterministic given identical canonical state,
MUST be idempotent when re-committing an identical state, and MUST record all required manifest fields.

Required manifest content (minimum):

- manifest_schema_version
- created_at (timestamp)
- repo/profile/policy references:
  - policy_ref (string; may be empty but must exist as field)
  - profile_ref (string; may be empty but must exist as field)
- EPT ordered list:
  - ordered ettle_ids
  - per-ettle ordered EP list (ordinal order) including:
    - ep_id
    - ordinal
    - normative
    - ep_digest (stable digest of normalized EP payload)
- effective constraints / resolution:
  - effective_constraints (possibly empty list; field MUST exist)
  - constraint_resolution (possibly empty; field MUST exist)
- coverage / exceptions:
  - coverage (possibly empty; field MUST exist)
  - exceptions (possibly empty; field MUST exist)
- integrity:
  - root_ettle_id for the commit target
  - ept_digest (digest of canonical EPT representation)
  - manifest_digest (CAS digest of manifest bytes; implicit identity; includes created_at)
  - semantic_manifest_digest (digest of the manifest with created_at excluded; stable comparison key)
  - store_schema_version (current DB schema version/migration head)
  - seed_digest (optional; include if available from provenance)

Ledger append requirements (facet_snapshots table):

- snapshot_id (monotonic or UUID; stable identifier)
- root_ettle_id
- manifest_digest
- created_at
- parent_snapshot_id (nullable; for linear history; optional in v0 but schema-ready)
- status (e.g., committed) if present in schema
- any minimal provenance linkage (e.g., provenance_event_id) if present in schema

CAS requirements:

- manifest JSON stored as `kind: manifest_json` (or equivalent) and indexed in cas_blobs when healthy.
- CAS write MUST be atomic temp→rename.
- If the same manifest_digest already exists with identical bytes, treat as success.

Transaction boundary:

- The commit operation MUST behave as atomic:
  - either (a) ledger entry exists AND referenced manifest exists in CAS, or
  - (b) neither exists (no partial commit).
- If CAS write succeeds but ledger append fails, implementation MUST roll back or clean up such that
  the system does not observe a ledger entry pointing at a missing manifest.
  (A leftover CAS blob without a ledger reference is acceptable but SHOULD be avoided where feasible.)

API surface (minimum):

- Engine function: `snapshot_commit(root_ettle_id, policy_ref, profile_ref, options) -> snapshot_id`
- Options MUST include: expected_head (optional) and dry_run (optional).
- Internal CLI wiring MAY be provided as a thin wrapper, but correctness is in the engine.

Out of scope (for this seed):

- Snapshot diff algorithm (manifest-to-manifest diff)
- Garbage collection / reachability
- Full CLI UX beyond invoking commit

Manifest content requirements (CORE frozen fields + additive envelope):
The snapshot manifest MUST contain, at minimum: - schema_version - facet_snapshot_id - realised_ettle_id - ettle_version - created_at # included in manifest bytes; digest is non-deterministic by design - approver - policy_ref - profile_ref - ept # ordered EP ids (the committed EPT) - ep_digests # map ep_id -> digest (for EPs in ept only) - ept_digest # digest of ordered ept structure - constraints # family-agnostic envelope (see below) - coverage # coverage metrics object - exceptions # exception list (may be empty) - manifest_digest # digest over the exact manifest bytes written to CAS - semantic_manifest_digest # digest over a canonicalised copy of the manifest with created_at removed/zeroed

Constraints envelope (anti-lock-in contract): - The manifest MUST contain a top-level `constraints` object that is extensible by family. - The `constraints` object MUST be shaped so that ABB→SBB is represented as ONE family, not the whole schema. - However, to remain compatible with the CORE charter and existing tooling, the `constraints` object MUST also
expose the CORE frozen ABB/SBB projections as stable fields.

Required shape (minimum viable, additive-safe):
constraints:
declared_refs: [] # ordered, deterministic list of constraint refs active on the EPT (may be empty)
families: {} # map family_name -> {active_refs, outcomes, evidence, digest} (may be empty) # Frozen ABB→SBB projections (must exist even if empty):
applicable_abb: [] # list of ABB constraint ids
resolved_sbb: [] # list of resolved SBB constraint ids
resolution_evidence: [] # opaque evidence records (predicate matched, selected ids, etc.)
constraints_digest: <digest> # digest over canonicalised constraints envelope (NOT including created_at)

Determinism rules (binding): - `ept` order MUST be deterministic (by EPT computation rules). - `declared_refs` order MUST be deterministic: - primary: family then kind then id (lexicographic), unless an explicit ordinal is stored. - `families` keys MUST be ordered deterministically in canonicalisation (lexicographic). - Any list inside `families[family].*` MUST be deterministically ordered.

5. Implement a deterministic diff engine that compares two snapshot manifests and produces:

1) A structured diff (JSON) suitable for downstream evaluators.
2) A human-readable summary (Markdown/text) for review.

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

## HOW (Implementation)

1. No scenarios. This EP is informational only.

2. Refinement only. Implementation scenarios live in child Ettles.

3. No new implementation scenarios here. The milestone is already delivered.
   This EP exists to:

- provide a stable parent refinement node for snapshot commit,
- preserve the dependency relationship in the refinement tree,
- and ensure rendered views show correct prerequisites.

4. Implementation approach (normative):

- Deterministic traversal:
  - EPT computation MUST use deterministic ordering primitives (no hash-iteration dependence).
  - EP ordering per Ettle MUST be ordinal order.
- Digesting:
  - EP digest MUST be computed from a canonical serialization of normalized EP content.
  - EPT digest MUST be computed from a canonical serialization of the ordered EPT structure.
  - Manifest digest MUST be computed over exact manifest JSON bytes written to CAS.
- Manifest serialization: - MUST be stable JSON with deterministic key ordering and deterministic list ordering. - The manifest MUST include `created_at` in the manifest JSON bytes written to CAS. - As a result, `manifest_digest` is expected to vary between commits even when canonical state is unchanged. - The system MUST therefore compute and record `semantic_manifest_digest` as the stable comparison key: - `semantic_manifest_digest` is computed over a canonical serialization of the manifest with `created_at` excluded. - All idempotency/determinism comparisons for “same canonical state” MUST use `semantic_manifest_digest`
  (and `ept_digest`), not `manifest_digest`. - The manifest MUST record both: - `manifest_digest` (CAS digest of full bytes, including created_at) - `semantic_manifest_digest` (comparison key excluding created_at) - This rule MUST be documented and tested.
  Scenarios (all MUST be implemented as tests; unit/integration; Gherkin is normative):

Feature: Snapshot commit pipeline

Background:
Given a repository with SQLite + CAS store initialised
And canonical Ettle/EP state exists in SQLite
And the EPT computation is deterministic and tested (Phase 0.5)

# --- Happy path and core invariants ---

Scenario: Commit writes manifest to CAS and appends ledger entry in one logical commit
Given root_ettle_id "ettle:root" exists
And policy_ref is "policy/default@0"
And profile_ref is "profile/default@0"
When I call snapshot_commit for "ettle:root"
Then a manifest JSON blob is written to CAS
And the CAS blob digest equals manifest_digest recorded in the ledger
And a facet_snapshots ledger row exists referencing that manifest_digest
And the manifest contains an EPT ordered list
And every EP in the manifest has a stable ep_digest
And effective_constraints, constraint_resolution, coverage, and exceptions fields exist (even if empty)

Scenario: Commit is deterministic for identical canonical state
Given the canonical DB state is unchanged between runs
When I call snapshot_commit twice (without modifying canonical state)
Then the produced manifest is semantically identical between the two commits
And the EPT digest is identical
And semantic_manifest_digest is identical between the two commits
And manifest_digest differs between the two commits (created_at varies)

Scenario: Manifest includes created_at and semantic_manifest_digest
When I call snapshot_commit
Then the manifest includes created_at
And the manifest includes semantic_manifest_digest
And semantic_manifest_digest is computed with created_at excluded

Scenario: Commit records policy_ref and profile_ref exactly as provided
Given policy_ref is "policy/foo@1.2"
And profile_ref is "profile/bar@0.9"
When I call snapshot_commit
Then the manifest policy_ref equals "policy/foo@1.2"
And the manifest profile_ref equals "profile/bar@0.9"

Scenario: Commit enforces referential integrity between ledger and CAS
When snapshot_commit succeeds
Then reading CAS at manifest_digest returns valid JSON
And parsing that JSON yields manifest_schema_version and required fields
And loading the ledger row and following manifest_digest always resolves

# --- Idempotency and duplicates ---

Scenario: Commit rejects duplicate ledger entry for same parent head when expected_head is supplied
Given the repository head snapshot_id is H
When I call snapshot_commit with expected_head = H
And the commit succeeds producing new snapshot_id H2
When I call snapshot_commit again with expected_head = H
Then the commit fails with HeadMismatch
And no new ledger row is appended

Scenario: Commit is safe when called concurrently with the same expected_head
Given two workers A and B with expected_head = H
When both call snapshot_commit concurrently
Then exactly one commit succeeds
And the other fails with HeadMismatch (or equivalent optimistic concurrency error)
And the ledger remains a linear history (no duplicate head)

# --- Boundary conditions ---

Scenario: Commit with a single Ettle and a single EP
Given a root tree containing exactly 1 ettle with exactly 1 EP
When snapshot_commit runs
Then manifest EPT contains exactly one ettle_id
And that ettle has exactly one EP entry with ordinal 0

Scenario: Commit with a large but valid EPT (stress)
Given a generated canonical state with N=1000 ettles and M=5000 EPs
When snapshot_commit runs
Then it completes successfully within a reasonable bound for local execution
And manifest size_bytes is recorded in cas_blobs
And no ordering instability occurs (EPT digest stable across two runs)

Scenario: Commit when effective_constraints is empty
Given no constraints exist in canonical state
When snapshot_commit runs
Then manifest effective_constraints is an empty list
And manifest constraint_resolution is present and empty (object or list per schema)

Scenario: Commit when coverage/exceptions are empty
Given no coverage data exists
And no exceptions exist
When snapshot_commit runs
Then manifest coverage is present and empty
And manifest exceptions is present and empty

# --- Negative cases: invalid inputs and missing state ---

Scenario: Commit fails when root_ettle_id does not exist
Given root_ettle_id "ettle:missing" does not exist
When I call snapshot_commit for "ettle:missing"
Then the call fails with NotFound
And no CAS manifest is written
And no ledger row is appended

Scenario: Commit fails when canonical state violates EPT invariants
Given the canonical state contains a cycle in refinement links
When snapshot_commit runs
Then it fails with CycleDetected (or equivalent)
And no ledger row is appended

Scenario: Commit fails when EP ordinals are non-unique within an ettle
Given an ettle with duplicate ordinal EP entries exists in SQLite
When snapshot_commit runs
Then it fails with OrdinalConflict
And no ledger row is appended

Scenario: Commit fails when an EP references missing CAS content (if EP payload is CAS-backed)
Given an EP row references ep_body_digest D
And CAS does not contain D
When snapshot_commit runs
Then it fails with CasMissing
And no ledger row is appended

# --- Negative cases: CAS and DB failure injection ---

Scenario: CAS write failure prevents ledger append
Given CAS is configured to fail writes (simulated IO error)
When snapshot_commit runs
Then it fails with CasWriteFailed
And no ledger row is appended

Scenario: Ledger append failure does not produce a visible partial commit
Given CAS writes succeed
And SQLite is configured to fail during facet_snapshots insert (simulated)
When snapshot_commit runs
Then it fails with LedgerAppendFailed
And no new facet_snapshots row exists
And the head snapshot_id is unchanged
And (optional) the manifest blob may exist in CAS but is not referenced

Scenario: Transaction rollback leaves no partial DB changes
Given snapshot_commit is interrupted mid-transaction (simulated)
When the process restarts
Then the database contains no partially written snapshot rows for the interrupted commit
And schema invariants remain intact

# --- Consistency checks for manifest content ---

Scenario: Manifest EPT is ordered and stable
When snapshot_commit runs
Then the manifest EPT list is ordered deterministically
And EPT ordering is identical to Phase 0.5 traversal output

Scenario: Every EP entry in the manifest includes required fields
When snapshot_commit runs
Then for each EP entry:
And ep_id is present
And ordinal is present
And normative is present
And ep_digest is present

Scenario: Manifest includes store_schema_version and optional seed_digest
Given the store has applied migration head "0007" (example)
And provenance contains a seed_digest from the last seed import
When snapshot_commit runs
Then manifest store_schema_version equals the current migration head
And manifest seed_digest equals the provenance seed_digest

# --- Dry run behaviour ---

Scenario: Dry-run computes manifest but does not persist
Given options.dry_run is true
When snapshot_commit runs
Then it returns a computed manifest (or digest) in the response
And no CAS blob is written
And no ledger row is appended

# --- Reproducibility across reload ---

Scenario: Commit after DB reload produces stable digests
Given a canonical state was imported and committed once
And the process is restarted
And the canonical state is reloaded from SQLite
When snapshot_commit runs again without changes
Then EPT digest is identical to the prior run
And EP digests are identical to the prior run

Additional scenarios (constraints envelope + future-proofing):

Scenario: Snapshot manifest always contains constraints envelope fields even when no constraints are attached
Given an EPT whose EPs have no attached constraints
When I run snapshot_commit with a selected leaf EP
Then the manifest contains constraints.declared_refs as an empty list
And the manifest contains constraints.families as an empty map
And the manifest contains constraints.applicable_abb as an empty list
And the manifest contains constraints.resolved_sbb as an empty list
And the manifest contains constraints.resolution_evidence as an empty list

Scenario: Deterministic ordering of declared_refs is stable across insertion order
Given EP A and EP B each attach constraints in different insertion orders
And both constraints are active on the committed EPT
When I snapshot_commit twice with identical canonical state except insertion order
Then constraints.declared_refs ordering is identical
And constraints_digest is identical
And semantic_manifest_digest is identical

Scenario: Unknown constraint families are preserved as opaque outcomes without breaking snapshot commit
Given a constraint with family "observability" is attached to an EP
And no evaluation/resolution engine exists for that family (stub phase)
When I snapshot_commit
Then constraints.declared_refs includes that constraint id
And constraints.families may include an entry for "observability" with an empty outcomes list
And snapshot commit succeeds without attempting to interpret the family payload

Scenario: Snapshot commit rejects non-deterministic family outcome ordering
Given a constraints.families entry is produced from a HashMap iteration order
When I attempt to commit a snapshot
Then the system detects ordering non-determinism under determinism checks
And snapshot commit fails with DeterminismViolation (unless waived by policy)

5. Scenarios (all MUST be implemented as tests; unit/integration; Gherkin is normative):

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
