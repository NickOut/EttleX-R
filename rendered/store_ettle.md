# Storage Spine (SQLite + CAS + Seed Import)

## EP 0

**Normative**: Yes

**WHY**: Snapshot commit is only meaningful if canonical state is durable and content-addressed.
The storage spine is therefore the immediate prerequisite for snapshot commit and later diff/GC work.

**WHAT**: This Ettle is a structural anchor for the already-implemented Phase 1 Store Spine milestone.
It represents the existence of:

- SQLite schema + migrations discipline (including facet_snapshots/provenance_events stubs)
- Filesystem CAS with atomic writes
- cas_blobs index population (non-load-bearing)
- Seed Format v0 importer

The normative implementation detail for this milestone is defined in the bootstrap markdown Ettle
“Phase 1 Store Spine (SQLite + CAS + Seed Import)”.

**HOW**: No new implementation scenarios here. The milestone is already delivered.
This EP exists to:

- provide a stable parent refinement node for snapshot commit,
- preserve the dependency relationship in the refinement tree,
- and ensure rendered views show correct prerequisites.

**Child**: Snapshot Commit Pipeline (End-to-End)

## EP 1

**Normative**: Yes

**WHY**: | The constraint schema stubs milestone introduces canonical persistence for constraints as first-class entities. This requires a stable refinement anchor in the storage spine so the dependency relationship is explicit in the tree and rendered views show correct prerequisites for snapshot diff and later evaluation work.

**WHAT**: | Structural anchor for the constraint schema stubs milestone under the storage spine. Represents the existence of: - constraints and ep_constraint_refs tables (additive migrations) - family-agnostic manifest constraint envelope - cas_blobs and provenance event wiring for constraint objects
The normative implementation detail is defined in the constraint schema stubs seed (seed_constraint_schema_stubs_v3.yaml).

**HOW**: | No new implementation scenarios here. This EP exists to: - provide a stable parent refinement node for ettle:constraint_schema_stubs, - preserve the dependency relationship in the refinement tree, - and ensure rendered views show constraint stubs as a prerequisite sibling to snapshot commit before snapshot diff is implemented.

**Child**: Constraint Schema Stubs (CORE spine extensibility contract)

## EP 2

**Normative**: Yes

**WHY**: Governance artefacts — decisions, rationale, evidence — are first-class canonical
entities that require durable storage alongside semantic artefacts. Without a stable
refinement anchor, decision schema stubs and later governance work (policy gates,
evidence workflows, audit trails) have no coherent position in the tree.

**WHAT**: Structural anchor for governance artefact storage under the storage spine. Represents
the existence of:

- decisions, decision_evidence_items, and decision_links tables (additive migrations)
- portable evidence capture and deterministic query surfaces
- action:commands and action:queries for decision lifecycle management

The normative implementation detail is defined in the decision schema stubs seed
(seed_decision_schema_stubs_v1.yaml).

**HOW**: No new implementation scenarios here. This EP exists to:

- provide a stable parent refinement node for ettle:decision_schema_stubs,
- separate governance artefact storage from semantic canonical storage (ep:store:1),
- and ensure rendered views correctly reflect the two-track storage architecture
  (semantic vs governance) under the store spine.
