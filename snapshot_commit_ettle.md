# Snapshot Commit Pipeline (End-to-End)

## EP 0

**Normative**: Yes

**WHY**: Provide immutable semantic anchors by committing canonical state into an append-only ledger.
Enable reliable reproducible diffs and downstream TES generation by persisting a manifest in CAS and
anchoring it in the snapshot ledger in a single transaction boundary.

This is the first point at which the system becomes self-hosting: a stable commit forms a durable
reference that can be used to reproduce EPT, validate invariants, and generate projections.

Dependency: this Ettle assumes the Storage Spine exists (SQLite schema + CAS + seed import) and is healthy.


**WHAT**: Implement snapshot commit as a single, atomic operation:

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


**HOW**: Implementation approach (normative):
  - Deterministic traversal:
      - EPT computation MUST use deterministic ordering primitives (no hash-iteration dependence).
      - EP ordering per Ettle MUST be ordinal order.
  - Digesting:
      - EP digest MUST be computed from a canonical serialization of normalized EP content.
      - EPT digest MUST be computed from a canonical serialization of the ordered EPT structure.
      - Manifest digest MUST be computed over exact manifest JSON bytes written to CAS.
  - Manifest serialization:
      - MUST be stable JSON with deterministic key ordering and deterministic list ordering.
      - The manifest MUST include `created_at` in the manifest JSON bytes written to CAS.
      - As a result, `manifest_digest` is expected to vary between commits even when canonical state is unchanged.
      - The system MUST therefore compute and record `semantic_manifest_digest` as the stable comparison key:
          - `semantic_manifest_digest` is computed over a canonical serialization of the manifest with `created_at` excluded.
          - All idempotency/determinism comparisons for “same canonical state” MUST use `semantic_manifest_digest`
            (and `ept_digest`), not `manifest_digest`.
      - The manifest MUST record both:
          - `manifest_digest` (CAS digest of full bytes, including created_at)
          - `semantic_manifest_digest` (comparison key excluding created_at)
      - This rule MUST be documented and tested.
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


