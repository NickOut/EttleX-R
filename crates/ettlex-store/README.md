# ettlex-store

**EttleX Store - Persistence Layer with SQLite, CAS, and Seed Import**

Persistence and storage layer providing durable semantic state management for EttleX.

## Overview

EttleX Store implements the storage spine that bridges between the pure domain models in `ettlex-core` and persistent storage. It provides:

- **SQLite Repository**: ACID-compliant persistence with migration framework
- **Content-Addressable Storage (CAS)**: Immutable blob storage for EP content and snapshots
- **Seed Import**: Parse and import seed YAML files with cross-seed reference support
- **Snapshot Commit**: Atomic manifest persistence with ledger anchoring
- **Event Ledger**: Append-only provenance tracking for all operations

## Architecture

The store layer is organized into focused modules:

```
ettlex-store/
├── cas/           # Content-addressable storage (filesystem-based)
├── ledger/        # Event sourcing and append-only ledger (stub)
├── migrations/    # SQLite schema migrations
├── repo/          # Repository pattern for Ettle/EP persistence
├── schema/        # Database schema management (stub)
├── seed/          # Seed format parser and importer
└── snapshot/      # Snapshot commit and manifest persistence
```

## Features

### ✅ SQLite Repository

- **Schema migrations**: Versioned SQL migrations with automatic application
- **Transactional operations**: Full ACID guarantees with transaction support
- **Round-trip integrity**: Persist and hydrate domain models without data loss
- **Soft deletion**: Tombstone pattern preserves referential integrity

**Migration files**: `src/migrations/*.sql`

**Key types**:

- `SqliteRepo` - Repository layer for Ettle/EP persistence
- `apply_migrations()` - Apply all pending migrations

### ✅ Content-Addressable Storage (CAS)

- **Immutable blobs**: SHA-256 addressed content storage
- **Atomic writes**: Collision detection and idempotent operations
- **Digest verification**: Content integrity guarantees
- **Efficient retrieval**: Direct digest-based lookup

**Storage layout**: `.ettlex/cas/{first_2_hex}/{digest}.{ext}`

**Key types**:

- `FsStore` - Filesystem-based CAS implementation
- `write()` - Write blob and return digest
- `read()` - Read blob by digest
- `exists()` - Check if blob exists

### ✅ Seed Import

- **YAML parsing**: Seed Format v0 with schema validation
- **Cross-seed references**: Reference Ettles/EPs from previously imported seeds
- **Invariant enforcement**: Uses core `refinement_ops` for EP uniqueness
- **Transaction safety**: Atomic import with automatic rollback on failure
- **Provenance tracking**: Events recorded for started/applied/completed

**Seed format**: See `handoff/seed_*.yaml` for examples

**Key functions**:

- `parse_seed_file()` - Parse and validate seed YAML
- `parse_seed_file_with_db()` - Parse with database-aware validation
- `import_seed()` - Atomic import with provenance tracking
- `compute_seed_digest()` - Deterministic seed content digest

**Supported operations**:

- Define Ettles with EPs (WHY/WHAT/HOW content)
- Create refinement links (parent EP → child Ettle)
- Cross-seed references (validated against database)

### ✅ Snapshot Commit

- **Manifest generation**: Deterministic snapshot manifest from EPT
- **Dual persistence**: CAS blob + SQLite ledger entry in single transaction
- **Digest computation**: manifest_digest (full) + semantic_manifest_digest (excludes timestamp)
- **Idempotency**: Re-committing identical semantic state returns existing snapshot
- **Optimistic concurrency**: expected_head validation for safe updates

**Manifest schema**: JSON with EPT, constraints, coverage, metadata

**Key types**:

- `SnapshotManifest` - Complete snapshot representation
- `SnapshotCommitResult` - Result including snapshot_id and digests

**Key functions**:

- `persist_manifest_to_cas()` - Write manifest to CAS
- `create_snapshot_ledger_entry()` - Append to snapshots table
- `commit_snapshot()` - Atomic dual-write operation

## Module Documentation

### `cas` - Content-Addressable Storage

Filesystem-based CAS with SHA-256 addressing. Provides immutable blob storage with collision detection.

**Public API**:

- `FsStore::new(root_path)` - Create CAS store at path
- `write(content, extension)` - Write blob, return digest
- `read(digest)` - Read blob by digest
- `exists(digest)` - Check existence

**Properties**:

- Idempotent writes (same content → same digest → no-op)
- Collision detection (different content → same digest → error)
- Thread-safe (atomic file operations)

### `migrations` - Schema Versioning

SQL migration framework with automatic version tracking and application.

**Migrations**:

- `001_initial_schema.sql` - Ettles, EPs, provenance_events, facet_snapshots stub
- `002_snapshots_ledger.sql` - Snapshots ledger for committed manifests
- `003_constraints_schema.sql` - Constraints and EP-constraint attachment tables

**Public API**:

- `apply_migrations(conn)` - Apply all pending migrations
- `get_schema_version(conn)` - Get current schema version

**Migration discipline**:

- Additive-only changes
- Never remove fields
- Always include rollback plan (manual)

### `repo` - Repository Layer

Bridges between domain models (`ettlex_core::model`) and SQLite persistence.

**Public API**:

- `SqliteRepo::persist_ettle()` - Insert/update Ettle
- `SqliteRepo::persist_ep()` - Insert/update EP
- `SqliteRepo::persist_constraint()` - Insert/update Constraint
- `SqliteRepo::persist_ep_constraint_ref()` - Insert EP-constraint attachment
- `SqliteRepo::get_ettle()` - Load Ettle by ID
- `SqliteRepo::get_ep()` - Load EP by ID
- `SqliteRepo::get_constraint()` - Load Constraint by ID
- `SqliteRepo::list_constraints()` - Load all Constraints
- `SqliteRepo::list_ep_constraint_refs()` - Load attachments for EP
- `hydration::load_tree()` - Hydrate full tree into Store (includes constraints)

**Transaction support**:

- `persist_ettle_tx(tx, ettle)` - Within transaction
- `persist_ep_tx(tx, ep)` - Within transaction
- `persist_constraint_tx(tx, constraint)` - Within transaction

### `seed` - Seed Import

Parses Seed Format v0 YAML and imports into canonical state with provenance tracking.

**Modules**:

- `parser.rs` - YAML parsing and validation
- `digest.rs` - Deterministic seed content hashing
- `importer.rs` - Transaction-based import orchestration
- `provenance.rs` - Event emission for import lifecycle

**Cross-seed support**:

- References to non-existent Ettles/EPs checked against database
- Parent Ettle loaded with all EPs for link operations
- Uses `refinement_ops::link_child()` to enforce EP uniqueness

**Validation**:

- Unique ordinals within Ettle
- Valid parent/child references
- EP0 exists and is normative
- No duplicate child mappings (enforced by core)

### `snapshot` - Snapshot Commit

Atomic snapshot commit pipeline: EPT → manifest → CAS + ledger.

**Modules**:

- `manifest.rs` - Snapshot manifest generation and schema
- `digest.rs` - EPT digest, manifest digest computation
- `persist.rs` - CAS write + ledger append transaction

**Manifest fields**:

- `manifest_schema_version` - Schema version (currently 1)
- `created_at` - RFC3339 timestamp
- `policy_ref`, `profile_ref` - Governance references
- `ept` - Ordered EP list with digests
- `ept_digest` - EPT content digest
- `manifest_digest` - Full manifest digest (includes timestamp)
- `semantic_manifest_digest` - Semantic digest (excludes timestamp)
- `root_ettle_id` - Snapshot root
- `store_schema_version` - Current migration head

**Transaction boundary**:

1. Validate expected_head (if provided)
2. Check semantic digest (idempotency)
3. Write manifest to CAS
4. Insert ledger entry
5. Commit transaction

## Database Schema

### `ettles` Table

Stores Ettle entities (architectural concepts/decisions).

```sql
CREATE TABLE ettles (
    id TEXT PRIMARY KEY,
    title TEXT NOT NULL,
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL,
    deleted_at TEXT,
    parent_id TEXT,
    FOREIGN KEY (parent_id) REFERENCES ettles(id)
);
```

### `eps` Table

Stores EP entities (refinement points with WHY/WHAT/HOW).

```sql
CREATE TABLE eps (
    id TEXT PRIMARY KEY,
    ettle_id TEXT NOT NULL,
    ordinal INTEGER NOT NULL,
    normative INTEGER NOT NULL DEFAULT 1,
    why TEXT NOT NULL,
    what TEXT NOT NULL,
    how TEXT NOT NULL,
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL,
    deleted_at TEXT,
    child_ettle_id TEXT,
    FOREIGN KEY (ettle_id) REFERENCES ettles(id),
    FOREIGN KEY (child_ettle_id) REFERENCES ettles(id)
);
```

### `constraints` Table

Stores constraint entities with family-agnostic design (Migration 003).

```sql
CREATE TABLE constraints (
    constraint_id TEXT PRIMARY KEY NOT NULL,
    family TEXT NOT NULL,
    kind TEXT NOT NULL,
    scope TEXT NOT NULL,
    payload_json TEXT NOT NULL,
    payload_digest TEXT NOT NULL,
    created_at INTEGER NOT NULL,
    updated_at INTEGER NOT NULL,
    deleted_at INTEGER
) STRICT;
```

**Fields**:

- `family` - Constraint family (e.g., "ABB", "observability", "custom-org")
- `kind` - Constraint kind (e.g., "Rule", "Metric", "Policy")
- `scope` - Constraint scope (e.g., "EP", "Ettle", "Global")
- `payload_json` - JSON payload with constraint data
- `payload_digest` - SHA-256 digest of payload (for content-addressable deduplication)

**Design**:

- No closed enums - arbitrary family/kind/scope strings for open extension
- Tombstone pattern with `deleted_at` for historical preservation
- Content-addressable via `payload_digest`

### `ep_constraint_refs` Table

Many-to-many relationship between EPs and constraints (Migration 003).

```sql
CREATE TABLE ep_constraint_refs (
    ep_id TEXT NOT NULL,
    constraint_id TEXT NOT NULL,
    ordinal INTEGER NOT NULL,
    created_at INTEGER NOT NULL,
    PRIMARY KEY (ep_id, constraint_id),
    FOREIGN KEY (ep_id) REFERENCES eps(id) ON DELETE CASCADE,
    FOREIGN KEY (constraint_id) REFERENCES constraints(constraint_id) ON DELETE CASCADE
) STRICT;
```

**Fields**:

- `ordinal` - Deterministic ordering within EP (for manifest serialization)

**Invariants**:

- Composite primary key prevents duplicate attachments
- Cascade deletes maintain referential integrity

### `provenance_events` Table

Append-only event log for seed import and operations.

```sql
CREATE TABLE provenance_events (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    event_type TEXT NOT NULL,
    event_payload TEXT NOT NULL,
    created_at TEXT NOT NULL
);
```

### `snapshots` Table

Snapshot ledger for committed manifests (planned migration 002).

```sql
CREATE TABLE snapshots (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    snapshot_id TEXT NOT NULL UNIQUE,
    root_ettle_id TEXT NOT NULL,
    manifest_digest TEXT NOT NULL,
    semantic_manifest_digest TEXT NOT NULL,
    created_at INTEGER NOT NULL,
    parent_snapshot_id TEXT,
    policy_ref TEXT NOT NULL,
    profile_ref TEXT NOT NULL,
    status TEXT NOT NULL DEFAULT 'committed',
    FOREIGN KEY (parent_snapshot_id) REFERENCES snapshots(snapshot_id)
);
```

## Usage Examples

### Import a Seed File

```rust
use ettlex_store::seed::importer::import_seed;
use rusqlite::Connection;
use std::path::Path;

let mut conn = Connection::open(".ettlex/store.db")?;
let seed_path = Path::new("handoff/seed_example.yaml");

let seed_digest = import_seed(&seed_path, &mut conn)?;
println!("Imported seed: {}", seed_digest);
```

### Write Content to CAS

```rust
use ettlex_store::cas::FsStore;

let store = FsStore::new(".ettlex/cas");
let content = b"Hello, World!";
let digest = store.write(content, "txt")?;

println!("Stored at digest: {}", digest);

// Read back
let retrieved = store.read(&digest)?;
assert_eq!(retrieved, content);
```

### Load Tree from Database

```rust
use ettlex_store::repo::hydration::load_tree;
use rusqlite::Connection;

let conn = Connection::open(".ettlex/store.db")?;
let store = load_tree(&conn)?;

// Access domain models
let root = store.get_ettle("ettle:root")?;
println!("Root ettle: {}", root.title);
```

## Testing

Run store tests:

```bash
cargo test -p ettlex-store
```

Key test files:

- `tests/cas_test.rs` - CAS operations
- `tests/migrations_test.rs` - Schema migrations
- `tests/round_trip_test.rs` - Persist/hydrate integrity
- `tests/seed_parse_test.rs` - Seed parsing
- `src/seed/importer.rs` - Import scenarios (unit tests)
- `tests/snapshot_persist_tests.rs` - Snapshot commit

## Error Handling

Uses `thiserror` for typed errors:

```rust
pub type Result<T> = std::result::Result<T, ExError>;
```

Error conversion utilities:

- `from_rusqlite(e: rusqlite::Error) -> ExError`
- `from_io(e: std::io::Error) -> ExError`
- `from_serde(e: serde_yaml::Error) -> ExError`

## Dependencies

Key dependencies:

- `rusqlite` (0.29) - SQLite driver with bundled library
- `ettlex-core` - Domain models and operations
- `serde_yaml` (0.9) - YAML parsing for seeds
- `sha2`, `hex` - Digest computation
- `uuid` (v7) - Time-ordered IDs
- `chrono` - Timestamp handling

## Future Work

Planned enhancements:

- [x] Migration 002: Snapshot ledger schema (completed)
- [x] Migration 003: Constraint persistence tables (completed)
- [ ] Event sourcing for all CRUD operations
- [ ] Read-optimized views (materialized EPT)
- [ ] Multi-repository support
