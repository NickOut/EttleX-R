# Leaf Bundle: EttleX Product > Storage Spine (SQLite + CAS + Seed Import) > Constraint Schema Stubs (CORE spine extensibility contract)

## WHY (Rationale)

1. Establish the product-level framing for EttleX as a semantic evolution engine.
   This EP is non-load-bearing for the storage/commit milestones; it exists to keep the tree rooted in a
   recognisable product-level node for rendering and navigation.

2. Provide a durable substrate so semantic state can be anchored, reproduced, and evolved safely.

3. | The constraint schema stubs milestone introduces canonical persistence for constraints as first-class entities. This requires a stable refinement anchor in the storage spine so the dependency relationship is explicit in the tree and rendered views show correct prerequisites for snapshot diff and later evaluation work.
4. Snapshot diff and later evaluation work depend on being able to persist, reference, and transport constraints
   without prematurely locking the system into a single constraint family (e.g., ABB/SBB).

CORE requires that the snapshot manifest contains effective ABB constraints and resolved SBB selections,
but MEDIUM explicitly generalises constraints into multiple families. This seed introduces the minimal
persistence and manifest-envelope stubs needed so later work (diff, evaluation, resolution, bytecode)
can evolve without schema churn or ABI breakage.

This seed is intentionally non-load-bearing: it adds schema and manifest envelope capabilities only.

## WHAT (Description)

1. Maintain a minimal product framing so leaf Ettles can be rendered in context.
   This EP does not impose implementation requirements on Phase 1/2 milestones.

2. Establish the platform foundations as refined milestones under this EP:

- Storage spine (SQLite + CAS + seed import)
- Snapshot commit pipeline (manifest + ledger anchor)

3. | Structural anchor for the constraint schema stubs milestone under the storage spine. Represents the existence of: - constraints and ep_constraint_refs tables (additive migrations) - family-agnostic manifest constraint envelope - cas_blobs and provenance event wiring for constraint objects
   The normative implementation detail is defined in the constraint schema stubs seed (seed_constraint_schema_stubs_v3.yaml).
4. Implement the minimal canonical persistence and manifest-schema extensions needed before snapshot diff.

Success criteria (binding):

1. Constraint entities exist in canonical state, with EP-level attachment.
2. EP reference tables exist such that a closure (EPT) can enumerate which constraints were
   declared/attached along that closure.
3. Snapshot manifest includes a constraint envelope that is family-agnostic.
4. Nothing in canonical state, snapshot commit, or later seeds is constrained to ABB/SBB-only.
5. The implementation does not introduce evaluation, resolution, or enforcement logic.

Dependency assumptions (pre-existing):

- Storage spine exists (SQLite + CAS + migrations discipline).
- Snapshot commit exists (manifest written to CAS; ledger append).

---

Normative CORE spine constraint extensibility contract

The implementation MUST satisfy the frozen CORE invariants while remaining compatible with MEDIUM/FUTURE.

A) Frozen CORE requirements that MUST remain true (until v0.3):

A.1 Manifest required fields and additive-only evolution - "Snapshot manifest SHALL contain: ... effective ABB constraints ... resolved SBB selections ..."
and "Manifest schema may only be extended (additive changes allowed). Field removals are prohibited."

A.2 Constraint DSL complexity is explicitly experimental but MUST NOT alter frozen core invariants.

B) MEDIUM constraint-family generalisation that MUST be supported by CORE implementations:

B.1 "ABB→SBB is one family. MEDIUM introduces a generalized constraint taxonomy" including multiple
families beyond pattern constraints.

B.2 Each constraint has (minimum) identifiers and classification: constraint_id, family, kind, scope,
plus predicate/effects/evidence requirements.

C) FUTURE stability rules that MUST remain possible:

C.1 "No FUTURE feature may redefine CORE invariants. FUTURE features must be optional and must
degrade safely to MEDIUM behaviour under conservative policies/profiles."

C.2 FUTURE event sources include "constraint edits" and automation must be policy/profile governed.

D) Contract rules (binding; these are the guardrails that prevent ABB/SBB lock-in):

D.1 Family-agnostic canonical representation - Canonical state MUST represent constraints as first-class entities with a `family` field. - Canonical state MUST NOT hard-code a closed set of families. - Canonical state MUST treat ABB/SBB (if present) as one possible family, not the schema.

D.2 Family-agnostic manifest envelope - The snapshot manifest MUST contain a `constraints` envelope that can carry constraints from
any family. - The envelope MUST be structured such that unknown families can be preserved and diffed as
opaque payloads (by digest) without being interpreted.

D.3 CORE-required ABB/SBB fields as a _projection_ - To satisfy CORE manifest requirements, the manifest MUST include the CORE-required ABB/SBB
fields. - Those ABB/SBB fields MUST be derived from (or be aliases of) the family-agnostic envelope, not
be the sole representation. - If no ABB/SBB constraints exist, the ABB/SBB fields MUST still exist and MUST represent an empty
effective/resolved set.

D.4 Additive-only, schema-stable evolution - All new manifest fields introduced by this seed MUST be additive. - All new database tables/columns introduced by this seed MUST be additive. - Existing snapshot commit behaviour MUST remain valid, except for adding new fields that must
be populated deterministically.

D.5 No evaluation/resolution semantics in this seed - This seed MUST NOT introduce a constraint evaluation engine. - This seed MUST NOT implement ABB/SBB resolution logic. - This seed MUST NOT block snapshot commits due to constraint content.

D.6 Determinism requirements for later diff/commit - Any ordering applied to constraint lists in manifests MUST be deterministic. - Any digest computed over constraint payloads MUST be based on canonical serialization.

---

Required schema changes (minimum; additive):

1. constraints
   - constraint_id (string; stable; opaque)
   - family (string; open set)
   - kind (string; open set)
   - scope (string; open set)
   - payload_json (json/text; opaque family payload; may be minimal in CORE)
   - created_at, updated_at
   - deleted (tombstone)

2. ep_constraints (EP attachment)
   - ep_id
   - constraint_id
   - ordinal (int; deterministic ordering within an EP; immutable once assigned)
   - strength (string; optional stub; keep as text)
   - created_at
   - deleted (tombstone)

3. (optional but recommended stub) constraint_refs (snapshot-time reference index)
   - snapshot_id
   - constraint_id
   - role (declared/effective/resolved/evidence)

Required snapshot manifest fields (minimum; additive; deterministic):

- constraints:
  - schema_version (int)
  - declared: [ {constraint_id, family, kind, scope, payload_digest} ... ]
  - effective: [ {constraint_id, family, kind, scope, payload_digest} ... ]
  - resolved: [ {constraint_id, family, kind, scope, resolution_digest?} ... ]
  - evidence: [ {constraint_id, evidence_digest} ... ]

- effective_abb_constraints: []
- resolved_sbb_selections: []
- resolution_evidence: []

Notes (binding):

- In CORE, declared/effective/resolved may all be identical for now (no evaluation).
- payload_digest MUST be present even if payload_json is empty: the digest commits the payload bytes.
- Unknown families MUST be preserved in payload_json and represented in manifest.

Canonical schema extensions (SQLite; additive-only):

The store MUST introduce the following canonical tables (names are binding for v0.1/v0.2 unless already present):

    1) constraints
       - constraint_id TEXT PRIMARY KEY
       - family TEXT NOT NULL                        # anti-lock-in: required for all constraints
       - kind TEXT NOT NULL                          # 'abb' | 'sbb' | 'other' (stringly typed in stub phase)
       - payload_json TEXT NULL                      # opaque JSON (family-defined schema; not interpreted here)
       - payload_digest TEXT NULL                    # sha256 of canonical JSON bytes when payload_json present
       - created_at TEXT NOT NULL
       - updated_at TEXT NOT NULL
       - deleted_at TEXT NULL                        # tombstone (optional but recommended)

    2) ep_constraint_refs
       - ep_id TEXT NOT NULL
       - constraint_id TEXT NOT NULL
       - ordinal INTEGER NOT NULL DEFAULT 0          # deterministic ordering surface (0 when unused)
       - created_at TEXT NOT NULL
       PRIMARY KEY (ep_id, constraint_id)
       FOREIGN KEY(ep_id) REFERENCES eps(ep_id)
       FOREIGN KEY(constraint_id) REFERENCES constraints(constraint_id)

    3) constraint_sets (OPTIONAL in v0.1; RECOMMENDED as a stub anchor for MEDIUM)
       - constraint_set_id TEXT PRIMARY KEY
       - name TEXT NOT NULL
       - family_hint TEXT NULL
       - payload_json TEXT NULL
       - created_at TEXT NOT NULL
       - updated_at TEXT NOT NULL

    4) constraint_set_members (OPTIONAL if constraint_sets exists)
       - constraint_set_id TEXT NOT NULL
       - constraint_id TEXT NOT NULL
       - ordinal INTEGER NOT NULL DEFAULT 0
       PRIMARY KEY (constraint_set_id, constraint_id)

Notes: - No evaluator is implemented in this seed: payload_json is opaque. - Determinism is enforced by explicit `ordinal` fields or lexicographic ordering fallbacks. - These tables MUST NOT force ABB/SBB-specific columns beyond the generic (family, kind, payload).

API surface (Rust store/core; minimal):

- create_constraint(family, kind, payload_json?) -> constraint_id
- get_constraint(constraint_id) -> constraint record
- attach_constraint(ep_id, constraint_id, ordinal?) -> ()
- detach_constraint(ep_id, constraint_id) -> ()
- list_constraints_for_ep(ep_id) -> ordered list

## HOW (Implementation)

1. No scenarios. This EP is informational only.

2. Refinement only. Implementation scenarios live in child Ettles.

3. | No new implementation scenarios here. This EP exists to: - provide a stable parent refinement node for ettle:constraint_schema_stubs, - preserve the dependency relationship in the refinement tree, - and ensure rendered views show constraint stubs as a prerequisite sibling to snapshot commit before snapshot diff is implemented.
4. Scenarios (all MUST be implemented as tests; unit/integration; Gherkin is normative):

Feature: Constraint schema stubs and extensibility contract

Background:
Given a repository with SQLite + CAS store initialised
And snapshot commit is already implemented and passing tests

# --- Schema and persistence stubs ---

Scenario: Create a constraint entity with an arbitrary family without code changes
Given a constraint payload JSON with family "platform" and kind "runtime"
When I create a constraint with family "platform" and kind "runtime"
Then it is stored in canonical state with family "platform" preserved verbatim
And no validation rejects unknown families

Scenario: Attach multiple constraints to a single EP with stable ordering
Given an EP "ep:example:0" exists
And two constraints exist "c:1" and "c:2"
When I attach "c:1" with ordinal 0 and "c:2" with ordinal 1
Then listing EP constraints returns ["c:1", "c:2"] in ordinal order
And ordinals are immutable (attempt to reassign fails)

Scenario: Tombstoning a constraint does not delete historical attachment records
Given constraint "c:3" is attached to EP "ep:example:0"
When I tombstone constraint "c:3"
Then the constraint is excluded from active queries
But historical snapshots that referenced "c:3" remain readable

# --- Manifest envelope requirements ---

Scenario: Snapshot commit includes a family-agnostic constraints envelope
Given root_ettle_id "ettle:root" exists
And the EPT includes an EP with attached constraints of families "pattern" and "compliance"
When I call snapshot_commit for "ettle:root"
Then the manifest contains a top-level "constraints" envelope
And the envelope contains entries for both families
And each entry includes constraint_id, family, kind, scope, and payload_digest
And the ordering of constraints in each list is deterministic

Scenario: CORE-required ABB/SBB manifest fields remain present as projection
Given the current closure includes zero ABB/SBB-family constraints
When I call snapshot_commit
Then manifest fields effective_abb_constraints, resolved_sbb_selections, and resolution_evidence exist
And each of those fields is an empty list
And the family-agnostic constraints envelope still exists and is authoritative

Scenario: Unknown constraint families are preserved without interpretation
Given a constraint with family "vendor_ext" and payload containing unknown fields
When I call snapshot_commit
Then the manifest includes that constraint in the constraints envelope
And payload_digest matches a canonical digest of payload_json
And no code path attempts to interpret the payload structure

# --- Negative cases and invariants ---

Scenario: Attempting to constrain families to an enum is rejected by contract tests
Given code changes that introduce a closed enum of constraint families
When contract tests are run
Then they fail with a message "constraint family MUST be open set"

Scenario: Manifest field removal is prohibited
Given code changes that remove "effective_abb_constraints" or "constraints" from the manifest
When snapshot commit tests are run
Then they fail with a message "Manifest schema may only be extended"

Scenario: Non-deterministic constraint ordering is detected
Given code changes that iterate constraints using hash-map iteration order
When I call snapshot_commit twice with identical canonical state
Then semantic_manifest_digest differs
And the failure is reported as DeterminismViolation

Scenario: Snapshot commit does not fail due to constraint content
Given a constraint payload that is syntactically malformed for its family
When I attach it to an EP and call snapshot_commit
Then the commit succeeds
And the payload is preserved as opaque bytes with a payload_digest

# --- Boundary conditions ---

Scenario: Large constraint payloads are supported as opaque blobs
Given a constraint payload of size 1MB
When I store it and commit a snapshot
Then payload_digest is computed successfully
And the manifest size is within configured CAS limits

Scenario: Constraint attachment list can be empty
Given an EP with no attached constraints
When I call snapshot_commit
Then constraints.declared may be empty
And the manifest still contains the constraints envelope and ABB/SBB projection fields

Note: Link ownership - This seed MUST NOT declare a direct parent→child link to ettle:snapshot_diff because that intent is defined in a separate seed file. - The dependency is declared by the snapshot diff seed (which depends on the constraint schema stubs being present). - Importers MAY enforce referential integrity per-seed; avoiding cross-seed links prevents failed imports unless seeds are imported as a set.
