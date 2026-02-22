# EttleX Product

## EP 0

**Normative**: No

**WHY**: Establish the product-level framing for EttleX as a semantic evolution engine.
This EP is non-load-bearing for the storage/commit milestones; it exists to keep the tree rooted in a
recognisable product-level node for rendering and navigation.

**WHAT**: Maintain a minimal product framing so leaf Ettles can be rendered in context.
This EP does not impose implementation requirements on Phase 1/2 milestones.

**HOW**: No scenarios. This EP is informational only.

## EP 1

**Normative**: Yes

**WHY**: Provide a durable substrate so semantic state can be anchored, reproduced, and evolved safely.

**WHAT**: Establish the platform foundations as refined milestones under this EP:

- Storage spine (SQLite + CAS + seed import)
- Snapshot commit pipeline (manifest + ledger anchor)

**HOW**: Refinement only. Implementation scenarios live in child Ettles.

**Child**: Storage Spine (SQLite + CAS + Seed Import)
