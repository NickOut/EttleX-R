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
