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

**Migrations** (14 total as of Slice 02):

- `001_initial_schema.sql` - Ettles, EPs, provenance_events
- `002_snapshot_ledger.sql` - Snapshots ledger for committed manifests
- `003_constraints_schema.sql` - Constraints and EP-constraint attachment tables *(dropped in 014)*
- `004_decisions_schema.sql` - Decisions and decision links
- `005_profiles_schema.sql` - Profiles table
- `006_approval_requests_schema.sql` - Approval requests
- `007_approval_cas_schema.sql` - `request_digest` on approval_requests (CAS-backed)
- `008_mcp_command_log.sql` - OCC log *(renamed to `command_log` in 014)*
- `009_parent_ep_id.sql` - `parent_ep_id TEXT` on ettles *(legacy, superseded by EP model in future slice)*
- `010_backfill_parent_ep_id.sql` - Backfill `parent_ep_id`
- `011_eps_title.sql` - Nullable `title TEXT` on `eps`
- `012_ettle_v2_schema.sql` - Add `why`, `what`, `how`, `reasoning_link_id`, `reasoning_link_type`, `tombstoned_at` to `ettles`
- `013_ettle_timestamps_iso8601.sql` - Migrate `ettles.created_at`/`updated_at` from INTEGER to TEXT ISO-8601
- `014_slice02_schema.sql` - Rename `mcp_command_log`→`command_log`; rename `provenance_events.timestamp`→`occurred_at` (ISO-8601); drop `constraints`/`ep_constraint_refs` and related tables; add `relation_type_registry`, `relations`, `groups`, `group_members`; seed 4 built-in relation types

**Public API**:

- `apply_migrations(conn)` - Apply all pending migrations
- `get_schema_version(conn)` - Get current schema version

**Migration discipline**:

- Additive-only changes
- Never remove fields
- Always include rollback plan (manual)

### `repo` - Repository Layer

Bridges between domain models (`ettlex_core::model`) and SQLite persistence.

**Ettle/EP functions** (all on `&Connection` unless noted):

- `insert_ettle(conn, record)` — Insert new Ettle
- `get_ettle(conn, id)` — Load `EttleRecord` by ID (returns `Option`)
- `list_ettles(conn, opts)` — Paginated `EttleListPage`
- `update_ettle(conn, id, patch)` — Update Ettle fields
- `tombstone_ettle(conn, id)` — Set `tombstoned_at`
- `insert_ep(conn, record)` — Insert new EP
- `get_ep(conn, id)` — Load EP by ID

**Relation functions**:

- `insert_relation(conn, record)` — Insert new `RelationRecord`
- `get_relation(conn, id)` — Load relation by ID
- `update_relation_properties(conn, id, props)` — Update `properties_json`
- `tombstone_relation(conn, id)` — Set `tombstoned_at`
- `list_relations(conn, filter)` — Filter by `source_ettle_id` and/or `relation_type`
- `count_active_constraint_relations(conn, ettle_id)` — Used by `EttleTombstone` guard

**Group functions**:

- `insert_group(conn, record)` — Insert new `GroupRecord`
- `get_group(conn, id)` — Load group by ID
- `list_groups(conn)` — All active groups
- `tombstone_group(conn, id)` — Set `tombstoned_at` (blocked if active members exist)
- `insert_group_member(conn, record)` — Add `GroupMemberRecord`
- `tombstone_group_member(conn, group_id, ettle_id)` — Tombstone membership
- `list_group_members(conn, group_id, include_tombstoned)` — Members of a group
- `count_active_group_members(conn, group_id)` — Used by `GroupTombstone` guard

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

Current as of migration 014 (Slice 02). Total: 17 tables.

### `ettles` Table

```sql
CREATE TABLE ettles (
    id           TEXT PRIMARY KEY,
    title        TEXT NOT NULL,
    why          TEXT NOT NULL DEFAULT '',
    what         TEXT NOT NULL DEFAULT '',
    how          TEXT NOT NULL DEFAULT '',
    reasoning_link_id   TEXT REFERENCES ettles(id),
    reasoning_link_type TEXT,
    tombstoned_at TEXT,          -- ISO-8601 or NULL
    created_at   TEXT NOT NULL,  -- ISO-8601
    updated_at   TEXT NOT NULL   -- ISO-8601
    -- legacy columns (parent_id, deleted, metadata) retained for migration compatibility
);
```

### `eps` Table

```sql
CREATE TABLE eps (
    id           TEXT PRIMARY KEY,
    ettle_id     TEXT NOT NULL REFERENCES ettles(id),
    ordinal      INTEGER NOT NULL,
    normative    INTEGER NOT NULL DEFAULT 1,
    title        TEXT,            -- nullable, added in migration 011
    why          TEXT,
    what         TEXT,
    how          TEXT,
    created_at   TEXT NOT NULL,
    updated_at   TEXT NOT NULL,
    deleted_at   TEXT,
    child_ettle_id TEXT REFERENCES ettles(id)
);
```

### `relation_type_registry` Table (Migration 014)

Canonical registry of allowed relation types. Seeded with 4 built-in entries.

```sql
CREATE TABLE relation_type_registry (
    relation_type     TEXT PRIMARY KEY,
    label             TEXT NOT NULL,
    properties_schema TEXT,        -- JSON Schema for properties_json validation (nullable)
    cycle_check       INTEGER NOT NULL DEFAULT 0,
    tombstoned_at     TEXT         -- ISO-8601 or NULL
);
```

Built-in types (seeded): `constraint`, `realises`, `semantic_peer`, `depends_on`.

### `relations` Table (Migration 014)

```sql
CREATE TABLE relations (
    relation_id      TEXT PRIMARY KEY,
    relation_type    TEXT NOT NULL REFERENCES relation_type_registry(relation_type),
    source_ettle_id  TEXT NOT NULL REFERENCES ettles(id),
    target_ettle_id  TEXT NOT NULL REFERENCES ettles(id),
    properties_json  TEXT,
    tombstoned_at    TEXT,
    created_at       TEXT NOT NULL,
    updated_at       TEXT NOT NULL
);
```

### `groups` Table (Migration 014)

```sql
CREATE TABLE groups (
    group_id      TEXT PRIMARY KEY,
    name          TEXT NOT NULL,
    tombstoned_at TEXT,
    created_at    TEXT NOT NULL,
    updated_at    TEXT NOT NULL
);
```

### `group_members` Table (Migration 014)

```sql
CREATE TABLE group_members (
    group_id      TEXT NOT NULL REFERENCES groups(group_id),
    ettle_id      TEXT NOT NULL REFERENCES ettles(id),
    tombstoned_at TEXT,
    created_at    TEXT NOT NULL,
    PRIMARY KEY (group_id, ettle_id)
);
```

### `provenance_events` Table

Append-only event log. `occurred_at` is ISO-8601 (renamed from `timestamp` in migration 014).

```sql
CREATE TABLE provenance_events (
    id             INTEGER PRIMARY KEY AUTOINCREMENT,
    kind           TEXT NOT NULL,
    correlation_id TEXT NOT NULL,
    occurred_at    TEXT NOT NULL,  -- ISO-8601
    metadata       TEXT            -- JSON
);
```

### `command_log` Table (renamed from `mcp_command_log` in Migration 014)

OCC counter. One row per successful write command.

```sql
CREATE TABLE command_log (
    id         INTEGER PRIMARY KEY AUTOINCREMENT,
    command    TEXT NOT NULL,
    applied_at TEXT NOT NULL  -- ISO-8601
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

### `snapshot/query.rs` — Read-Only Snapshot Queries

Pure read functions for the snapshots ledger (never write):

- `fetch_snapshot_manifest_digest(conn, snapshot_id)` — fetch `manifest_digest` column
- `fetch_snapshot_digests(conn, snapshot_id)` — fetch both `(manifest_digest, semantic_manifest_digest)`
- `fetch_snapshot_row(conn, snapshot_id)` — full `SnapshotRow` struct
- `list_snapshot_rows(conn, ettle_id: Option)` — all rows, optionally filtered by root Ettle
- `fetch_head_snapshot(conn)` — most recent committed snapshot (`ORDER BY created_at DESC`)
- `fetch_manifest_bytes_by_digest(cas, digest)` — read manifest blob from CAS

### `profile.rs` — Profile and Approval Queries

Read functions for profiles and approval requests (all `&Connection`):

- `load_profile_full(conn, profile_ref)` — full profile payload + digest
- `load_default_profile(conn)` — profile with `is_default = 1`
- `list_profiles_paginated(conn, after_ref, limit)` — cursor-paginated profile list
- `fetch_approval_row(conn, approval_token)` — full `ApprovalRow` struct
- `list_approval_rows_paginated(conn, after_key, limit)` — cursor-paginated approval list (ascending `created_at`)
- `query_approval_rows_no_digest(conn, after_key, limit)` — fallback for rows without `request_digest`

### Migration 007

`007_approval_cas_schema.sql` adds `request_digest TEXT` column to `approval_requests`
and an index on it. The `SqliteApprovalRouter` now writes full approval payload JSON
to CAS and stores the digest in this column (enabling `ApprovalGet` to retrieve the blob).

### Database Schema (additions)

#### `profiles` Table (Migration 005)

```sql
CREATE TABLE profiles (
    profile_ref TEXT PRIMARY KEY NOT NULL,
    payload_json TEXT NOT NULL,
    is_default INTEGER NOT NULL DEFAULT 0,
    created_at INTEGER NOT NULL
) STRICT;
```

#### `approval_requests` Table (Migrations 006 + 007)

```sql
CREATE TABLE approval_requests (
    approval_token TEXT PRIMARY KEY NOT NULL,
    reason_code TEXT NOT NULL,
    candidate_set_json TEXT NOT NULL,
    semantic_request_digest TEXT NOT NULL,
    status TEXT NOT NULL DEFAULT 'pending',
    created_at INTEGER NOT NULL,
    request_digest TEXT  -- added by migration 007
) STRICT;
```

## `FilePolicyProvider`

Implements `PolicyProvider` backed by Markdown files on the local filesystem.

```rust
use ettlex_store::file_policy_provider::FilePolicyProvider;
use ettlex_core::policy_provider::PolicyProvider;

let provider = FilePolicyProvider::new("policies");
// Override the default 1 MiB export size limit:
let provider = FilePolicyProvider::new("policies").with_max_bytes(512_000);

let list = provider.policy_list().unwrap();
let text = provider.policy_read("codegen_handoff_policy_v1").unwrap();
let export = provider.policy_export("codegen_handoff_policy_v1", "codegen_handoff").unwrap();
```

**Behaviour**:

- `policy_ref` maps to `{policies_dir}/{policy_ref}.md`
- `policy_list()` returns all `.md` files sorted by stem, each with `version = "0"`
- `policy_read(ref)` returns full UTF-8 text; invalid UTF-8 → `PolicyParseError`
- `policy_export(ref, "codegen_handoff")` extracts `<!-- HANDOFF: START -->…<!-- HANDOFF: END -->` blocks
- `policy_check(ref, ...)` returns `Ok(())` if the file exists, else `PolicyNotFound`
- Default max export: **1 MiB** (`DEFAULT_MAX_EXPORT_BYTES`)

See [`docs/policy-system.md`](../../docs/policy-system.md) for the full HANDOFF marker specification and error contract.

## Future Work

- [x] Migrations 001–014 complete
- [ ] Slice 03: Remove EP concept entirely (drop legacy EP and parent_ep_id columns)
- [ ] Slice 04: Remove seed import capability
- [ ] Read-optimized views (materialized EPT)
- [ ] Multi-repository support
