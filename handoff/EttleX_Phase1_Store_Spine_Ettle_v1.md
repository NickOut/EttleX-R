# Ettle (Bootstrap Markdown) — Phase 1 Store Spine (SQLite + CAS + Seed Import)
**Product:** EttleX  
**Purpose:** Implement the frozen storage spine needed for Phase 1: SQLite schema + migrations discipline, filesystem CAS with atomic writes, `cas_blobs` index maintenance, and a disposable Seed Format v0 importer that can populate canonical Ettle/EP state without “documents as canonical”. This milestone must also enable the next milestone (“Snapshot commit pipeline”) to be authored as a seed, imported, and then rendered (via Phase 0.5 deterministic rendering) into a handoff markdown for the coding agent.  
**Tier:** CORE (Bootstrapping)  
**Status:** Draft (bootstrap spec; generator-ready)  
**Created:** 2026-02-21  
**Ettle-ID:** ettle/bootstrap-phase1-store-spine (bootstrap)  
**EPs:** EP0 only (single-leaf implementable)

> This is a **leaf implementable** Ettle used to bootstrap Phase 1. It is intended to be handed to an external AI code generator with human oversight to produce the Tests/Code/Docs triad.
>
> **Semantics note:** This Ettle uses the project’s WHY/WHAT/HOW definitions:
> - **WHY** = purpose/rationale
> - **WHAT** = normative condition/feature that must be met
> - **HOW** = method/process to achieve it (**Gherkin scenarios live here**)
>
> **Continuity note (Phase 0.5 dependency):** Phase 0.5 delivered:
> - the in-memory canonical Ettle/EP model and CRUD invariants,
> - deterministic RT/EPT traversal,
> - deterministic rendering functions (including a “bootstrap markdown render”).
>
> Phase 1 MUST persist and reload the same canonical model without changing its semantics, and MUST preserve determinism across round-trips.

---

## EP0 — Frozen Store Spine (SQLite schema + CAS) + Seed Import v0

### WHY (purpose / rationale)

Phase 1 makes the semantic kernel durable and self-hosting by introducing a frozen persistence substrate and a minimal bootstrap ingestion path. Without a durable store we cannot:

1) commit snapshots and form semantic anchors in an immutable ledger,  
2) store manifests and computed projections in a stable, hash-addressed way,  
3) diff snapshots reliably,  
4) run a CLI loop that is robust to restarts and collaborative workflows,  
5) bootstrap EttleX “using itself” without reverting to document-driven canonical state.

The bootstrap paradox is explicit:

- canonical is data (DB + CAS), not prose documents,  
- yet we need an initial canonical state before the higher-level tooling exists.

Seed import resolves this paradox by providing a **minimal, mechanically ingestible, disposable** representation that can create canonical objects and links via the engine APIs. This is analogous to migrations/fixtures in conventional systems. It is not a return to “documents as canonical”.

This milestone is also deliberately structured to change how future work is specified:

- After this milestone, new features SHOULD be authored as seed YAML, imported into canonical state, and rendered using Phase 0.5 deterministic rendering into an implementation handoff markdown for the coding agent.
- Therefore, the store spine must not only write/read data, but must make the “import → canonical state → render” loop reliable and deterministic.

### WHAT (normative conditions / features)

#### 1. Phase 1 Scope Boundary (Non-negotiable)

This milestone MUST implement:

A) SQLite schema (all design tables required for Phase 1) including `cas_blobs` (extended with `kind`, and optionally `verified_at`/`status`), plus an internal `schema_version` table for migration discipline.  
B) Migration discipline from day one (`migrations/` folder, `schema_version` table, ordered application, idempotent apply detection).  
C) Filesystem CAS store using `cas/<shard>/<digest>.<ext>` layout with atomic temp→rename writes and read APIs.  
D) `cas_blobs` index: best-effort upsert within the same SQLite transaction boundary as ledger append operations. Index is **non-load-bearing** for correctness, but MUST be populated when the system is healthy.  
E) Seed import: Seed Format v0 (YAML) + importer module + internal helper binary (`ettlex seed import <seed.yaml|dir>`), not a stable long-term CLI surface.  
F) Store-backed canonical state reload: ability to read Ettles/EPs/links from SQLite into the Phase 0.5 in-memory model **without semantic drift**, and then render deterministically using the Phase 0.5 renderer. (This is required so Phase 1 can enable the next milestone to be specified as a seed rather than a bootstrap markdown Ettle.)

This milestone MUST NOT implement:

- snapshot commit pipeline (manifest building + ledger anchoring) beyond stubs/hooks,  
- snapshot diff logic,  
- full public CLI surface (other than the internal seed helper),  
- garbage collection or integrity scan logic (but schema may be “scan-ready” via optional columns).

#### 2. Storage semantics are canonical (DB + CAS), not prose

- The canonical truth for Ettles/EPs/links MUST live in SQLite.  
- Large text bodies (EP content) SHOULD be stored in CAS (digest-addressed) when CAS is available. For Phase 1 bootstrap simplicity, EP content MAY be stored inline in SQLite, but the system MUST still compute and persist stable digests and MUST keep the storage shape migration-ready for CAS-backed payloads without semantic change.  
- Prose/markdown outputs are derived views produced by deterministic rendering; they are not canonical.

#### 3. Determinism requirements

Determinism is a first-class requirement because semantic anchors (digests, EPT ordering, manifests) depend on stable traversal.

The store spine MUST preserve determinism under:

- repeated imports of the same seed (idempotency checks may reject duplicates or create identical state; in either case the canonical digest results MUST be stable),  
- DB round-trip: load → render MUST be byte-for-byte stable for a given canonical state,  
- CAS addressing: the same canonical object MUST yield the same digest and therefore the same CAS path.

Where ordering is required, implementations MUST use deterministic ordering primitives (e.g., `BTreeMap`, sorted vectors) and MUST NOT rely on hash map iteration order.

#### 4. SQLite schema (five tables) + invariants

This milestone implements six tables in total: the five Phase 1 design tables (including `facet_snapshots` and `provenance_events`) plus an internal `schema_version` migration control table.

The schema MUST be implemented as specified in the design doc for Phase 1.

At minimum the following tables MUST exist (Phase 1 design tables plus internal migration control):

1) `schema_version` (internal migration control; not counted as part of the “design tables”)
- Tracks applied migrations.
- MUST prevent skipping migrations.
- MUST record: migration id/name, applied timestamp, and an optional checksum of the migration file contents.

2) `ettles`
- Stores Ettle identity and non-EP metadata.
- MUST include a stable `ettle_id` primary key.
- MUST include fields sufficient to reconstruct the Phase 0.5 in-memory Ettle header/meta.

3) `eps`
- Stores EP identity and association to `ettles`.
- MUST include: `ep_id` primary key, `ettle_id` foreign key, `ordinal` (integer), `normative` (boolean), and references to EP content (prefer CAS digest reference, not inline).
- MUST enforce uniqueness: `(ettle_id, ordinal)` unique.
- MUST enforce stable linkage: each EP belongs to exactly one Ettle.
- Linkage to child Ettles MUST follow the design doc: if refinement membership is anchored to a parent EP, the anchor EP MUST carry `child_ettle_id` (nullable) to represent the single anchored child for that EP.

4) `facet_snapshots` (schema stub included now; pipeline deferred)
- Represents immutable snapshot ledger entries and their facet anchor references.
- Even though snapshot commit is out of scope for this milestone, the table schema MUST be present now to avoid early schema churn, consistent with the “include cas_blobs now even though it is not load-bearing” rationale.
- No rows are required to be written in Phase 1 except those created by optional test scaffolding (if any).

5) `provenance_events` (seed import provenance)
- Stores append-only provenance events for canonical state mutations.
- Seed import MUST emit provenance events (at minimum: “seed import started”, “seed import applied”, and “seed import completed” with seed digest).
- Provenance schema MUST be present now to avoid early migrations and to support auditability from the first import.

6) `cas_blobs`
- Index of CAS digests to paths and metadata.
- MUST include `digest` primary key.
- MUST include `relpath` and `size_bytes`.
- MUST include `kind` (e.g., `ep_body_md`, `seed_yaml`, `manifest_json`, `tes_json`).
- MAY include: `verified_at` (timestamp) and `status` (e.g., `present|missing|corrupt`) for integrity scan readiness. If included, they MUST be nullable and default to NULL.

Important: even if `cas_blobs` exists, correctness MUST NOT depend on its presence. Reads MUST be able to derive the path from digest (`cas/<shard>/<digest>.<ext>`) without consulting the index.

Design alignment note (links):
- Earlier drafts referred to a separate `ep_links` table. For Phase 1, the implementation MUST align with the design doc approach of representing anchored child membership via `eps.child_ettle_id` (nullable) and SHOULD avoid introducing a separate link table in this milestone. If a separate link view is introduced later, it MUST be derived without changing canonical semantics.
#### 5. Migration discipline (mandatory from day one)

- A `migrations/` folder MUST exist.  
- Each migration MUST have a monotonically increasing id prefix (e.g., `0001_...sql`, `0002_...sql`).  
- On startup (or on explicit open), the store MUST apply migrations in order.  
- If a migration is missing (gap), the store MUST fail fast.  
- Migrations MUST be applied within a transaction.  
- The `schema_version` table MUST record each applied migration.

#### 6. CAS filesystem store

CAS path layout MUST be:

- Root: `cas/` (relative to repo root or configured project root)  
- Shard: first two hex chars of digest, i.e. `cas/ab/`  
- Filename: `<digest>.<ext>` where `<ext>` is determined by `kind` (minimum: `.json` and `.md` supported)

Writes MUST be atomic:

- Write to a temp file in the same directory (`.<digest>.<ext>.tmp.<nonce>`), fsync as appropriate, then rename to final path.  
- If final exists and content matches, write MUST be treated as success (idempotent).  
- If final exists and content differs, this is a **CAS corruption** error and MUST fail.

Reads MUST:

- Given a digest and expected extension, return bytes or typed parse (where relevant).  
- Validate file presence; if missing, error classification MUST distinguish “missing blob” vs other IO.

#### 7. Seed Format v0 (YAML) — disposable bootstrap fixture

Seed import MUST use a small declarative Seed Format v0 that maps 1:1 to the canonical model and avoids “documents as canonical”.

Seed Format v0 MUST:

- be YAML,  
- include a top-level `schema_version: 0`,  
- include a list of `ettles`, each with `why/what/how` data in EPs,  
- include explicit `links` (parent/child), with optional `parent_ep` anchoring.

Seed Format v0 MUST include WHY/WHAT/HOW at EP level (full structure retained), because the imported tree will be used immediately to render handoff markdown for coding agents via the Phase 0.5 renderer.

A minimal v0 structure (normative keys only):

```yaml
schema_version: 0
project:
  name: ettlex
ettles:
  - id: ettle:root
    title: "EttleX Product"
    eps:
      - id: ep:root:0
        ordinal: 0
        normative: false
        why: "..."
        what:
          - type: narrative
            content: "..."
        how: "..."
links:
  - parent: ettle:root
    parent_ep: ep:root:0
    child: ettle:cli
```

Rules:

- `id` fields are opaque strings; the importer MUST NOT enforce a URI scheme beyond non-empty.  
- `ordinal` MUST be explicit and MUST be unique per ettle.  
- `what` MAY be either:
  - a string (treated as a single narrative block), or
  - a list of typed blocks `{type, content}`.  
  Importer MUST normalize both to the canonical in-memory representation.

- `links` MUST reference existing ettle ids.  
- If `parent_ep` is specified, it MUST reference an EP belonging to the parent ettle.

The importer MUST compute a **seed digest** for reproducibility:

- The seed digest MUST be computed over a canonical serialization of the normalized parsed seed structure, not raw YAML bytes.  
- Canonicalization MUST sort:
  - ettles by `id`,
  - EPs by `(ordinal, id)`,
  - links by `(parent, parent_ep, child)` (treat missing `parent_ep` as empty string for ordering).
- Canonical serialization MUST be stable JSON (or equivalent) with deterministic key ordering.

#### 8. Seed importer behaviour (engine-driven; no direct DB poking)

Seed import MUST:

- parse YAML strictly (reject duplicate keys; reject unknown top-level keys unless explicitly allowed; reject schema_version != 0),  
- validate invariants (uniqueness, referential integrity, ordinals, parent_ep ownership),  
- call engine APIs to create Ettles, EPs, and links (not raw SQL from importer module),  
- insert canonical objects into SQLite and write EP payloads to CAS (preferred),  
- upsert `cas_blobs` entries for written CAS objects (best-effort, but in normal operation must succeed).

Seed import MUST be available as:

- a Rust module used by tests and internal tooling, and  
- an internal helper binary command: `ettlex seed import <path>`  
  - where `<path>` may be a file or a directory containing one or more seed YAML files (directory traversal MUST be deterministic and MUST sort file names).

Seed import MUST NOT be a stable long-term CLI command at this stage. It is explicitly a bootstrap convenience.

Optional: “fire initial snapshot commit”
- The seed importer MAY support a `--commit` flag.  
- If snapshot commit pipeline is not yet implemented, `--commit` MUST fail with a clear NotImplemented error (not a silent no-op).  
- If implemented later, `--commit` MUST:
  - import seed objects,
  - then invoke snapshot commit for a specified target leaf (or whole tree) in one deterministic flow.

#### 9. Store-backed reload + deterministic render (enables next milestone workflow)

To enable “next milestone authored as seed rather than bootstrap markdown”, the store spine MUST provide:

A) A repository read API to load:
- a full tree (all ettles/eps/links), and  
- a subset rooted at a given ettle id (for leaf-focused rendering/handoff).

B) Deterministic load ordering:
- Ettle enumeration MUST be stable (sorted by id or by deterministic traversal),
- EP enumeration per ettle MUST be stable (sorted by ordinal then id),
- Link enumeration MUST be stable (sorted by parent, parent_ep, child).

C) Round-trip identity:
- Loading from DB and then rendering via Phase 0.5 render MUST be stable given identical canonical content.

The minimal success case for this requirement is:

- Import a seed describing a tree with at least 2 Ettles and at least 2 EPs,  
- Reload from SQLite,  
- Render using Phase 0.5 deterministic renderer,  
- Ensure the produced output is stable across two runs.

#### 10. Error taxonomy (minimum)

Implementation MUST distinguish and surface:

- Migration errors (missing migration, apply failure, checksum mismatch).  
- CAS errors (missing blob, write collision with differing bytes, invalid digest, IO).  
- Seed parse errors (YAML parse failure, schema mismatch, unknown fields if strict).  
- Seed validation errors (missing references, duplicate ids, ordinal conflicts, invalid parent_ep).  
- Store constraint errors (foreign key violations, uniqueness violations, transaction failures).

Errors MUST be structured and testable (not string-matched).

#### 11. Artefact expectations

This Ettle MUST result in:

- Code:
  - `implementation/src/store/...` (SQLite + migrations + repo APIs)
  - `implementation/src/cas/...` (filesystem CAS)
  - `implementation/src/seed/...` (seed v0 parser + importer)
  - `implementation/src/bin/ettlex.rs` (internal seed helper command only, minimal clap wiring)

- Tests:
  - unit tests for migration application ordering and schema_version handling,
  - unit tests for CAS atomic write/idempotency/collision detection,
  - unit tests for seed parsing + canonicalization + seed digest stability,
  - integration tests that import seed → reload → render (Phase 0.5 renderer) → compare outputs.

- Docs:
  - `implementation/docs/store_spine_phase1.md` describing schema, CAS layout, seed format v0, and invariants.
  - A short “How to bootstrap” section: `ettlex seed import seed.yaml` for Phase 1 developers.

### HOW (method / process / scenarios)

All scenarios below MUST be implemented as tests (unit/integration). Gherkin is normative.

#### A. Migrations + schema version discipline

Scenario: Open store applies migrations in order on empty DB  
Given an empty SQLite database file  
And a `migrations/` folder with migrations `0001_init.sql`, `0002_add_cas_blobs_kind.sql`  
When the store is opened  
Then migrations are applied in ascending id order  
And `schema_version` records both migrations as applied  
And the schema contains all five tables defined for Phase 1

Scenario: Open store fails if a migration id gap exists  
Given a `migrations/` folder containing `0001_init.sql` and `0003_third.sql`  
When the store is opened  
Then the store fails with a MigrationGap error  
And no partial schema changes are committed

Scenario: Migration re-application is idempotent and does not re-run applied migrations  
Given a database with migrations `0001` and `0002` already applied in `schema_version`  
When the store is opened again  
Then migrations are not re-applied  
And the open succeeds

Scenario: Migration checksum mismatch is detected (if checksums are recorded)  
Given a database with migration `0001` recorded with checksum X  
And the migration file `0001_init.sql` now has checksum Y where Y != X  
When the store is opened  
Then the store fails with a MigrationChecksumMismatch error

#### B. CAS write/read semantics

Scenario: CAS write is atomic and idempotent  
Given an empty CAS directory  
When I write bytes B with digest D and extension `.json`  
Then the file `cas/<shard(D)>/<D>.json` exists  
And reading digest D returns bytes B  
When I write bytes B again to the same digest D  
Then the write succeeds without modifying content

Scenario: CAS write detects collision with different bytes  
Given a CAS blob already stored at digest D with bytes B1  
When I attempt to write bytes B2 to digest D where B2 != B1  
Then the write fails with CasCollision error  
And the existing blob remains unchanged

Scenario: CAS read reports missing blob  
Given no file exists for digest D  
When I read digest D  
Then the operation fails with CasMissing error

#### C. cas_blobs index behaviour

Scenario: cas_blobs is upserted for successful CAS writes  
Given an empty database and empty CAS directory  
When I store an EP body blob to CAS with digest D and kind `ep_body_md`  
Then a `cas_blobs` row exists for digest D  
And the row contains relpath `cas/<shard(D)>/<D>.md`  
And the row contains size_bytes equal to the stored blob size  
And the row contains kind `ep_body_md`

Scenario: cas_blobs is non-load-bearing for reads  
Given a CAS blob exists on disk for digest D  
And no `cas_blobs` row exists for digest D  
When I read digest D via the CAS API  
Then the read succeeds  
And does not require the index table

Scenario: cas_blobs upsert is best-effort but normally consistent with ledger transactions  
Given a healthy SQLite DB and CAS directory  
When a store operation writes a CAS blob and appends a ledger-linked record in the same transaction boundary  
Then the transaction commits successfully  
And the cas_blobs row exists  
And the ledger-linked record exists  
And both refer to the same digest

(Note: in Phase 1, “ledger-linked record” may be the canonical inserts; snapshot ledger append is Phase 2. This scenario exists to enforce transaction wiring discipline early.)

#### D. Seed Format v0 parsing, normalization, and digest

Scenario: Seed v0 parses and imports a minimal tree  
Given a seed YAML file with schema_version 0  
And it contains ettles A (root) and B (child)  
And it contains EPs with explicit ordinals and WHY/WHAT/HOW content  
And it contains a link A→B anchored to a parent EP  
When I run `ettlex seed import seed.yaml`  
Then ettles A and B exist in SQLite  
And EPs exist for each ettle with their ordinals  
And the link A→B exists  
And EP payloads are stored in CAS and referenced by digest in SQLite (if CAS-backed payloads are used)

Scenario: Seed WHAT polymorphism normalizes to stable EP digests  
Given a seed file where an EP uses `what` as a single string  
And another seed file where the same EP uses `what` as a single-element typed block list with identical semantic content  
When I import each seed into a fresh store  
And I reload the resulting canonical state  
Then the normalized in-memory representation of `what` is identical  
And the computed EP digest is identical

Scenario: Seed digest is stable across equivalent YAML encodings  
Given two seed files with identical semantic content but different YAML formatting/order  
When I compute their seed digests  
Then the digests are identical

Scenario: Seed import rejects invalid references  
Given a seed with a link referencing a non-existent child ettle  
When I import it  
Then import fails with SeedValidation error  
And the database remains unchanged

Scenario: Seed import rejects duplicate ordinals within an ettle  
Given a seed where an ettle has two EPs with ordinal 0  
When I import it  
Then import fails with SeedValidation error

#### D2. Provenance events emitted during seed import

Scenario: Seed import emits provenance events  
Given an empty store  
When I import a valid seed file  
Then a provenance_events row exists with kind "seed_import_started"  
And a provenance_events row exists with kind "seed_import_applied" for each imported ettle   
And a provenance_events row exists with kind "seed_import_completed" containing the seed digest   
And all three events share a common import correlation id  

#### E. Store reload + deterministic render (Phase 0.5 renderer)

Scenario: Import → reload → render is stable  
Given a valid seed describing a small tree  
When I import the seed  
And I reload canonical state from SQLite into the Phase 0.5 in-memory model  
And I render that model to bootstrap markdown using the Phase 0.5 deterministic renderer  
And I repeat the reload and render a second time  
Then the two rendered outputs are byte-for-byte identical

Scenario: Reload ordering is deterministic independent of insertion order  
Given a seed describing multiple ettles  
And the importer inserts them in any order (implementation-dependent)  
When I reload the canonical state  
Then enumeration order is deterministic  
And rendering is stable

#### F. Optional seed-triggered commit flag behaviour (guarded)

Scenario: Seed import --commit fails clearly when snapshot commit not implemented  
Given snapshot commit pipeline is not implemented  
When I run `ettlex seed import seed.yaml --commit`  
Then the command fails with NotImplemented  
And the error message explicitly says snapshot commit is not available yet

---

## Notes to the implementation agent (non-normative guidance)

- Keep the Seed Format v0 parser intentionally small and strict. It is disposable, but determinism and validation are non-negotiable.
- Prefer to store EP payload bodies in CAS and keep SQLite rows small (digest references). If you do store inline text as a temporary measure, still compute the digest and record it consistently so later migration to CAS-backed payloads is straightforward.
- Use a single “store transaction” API to ensure future snapshot commit can reuse it unchanged.
- Ensure the repo read API returns Phase 0.5 types directly or via an explicit mapping module; do not fork a new parallel model.

