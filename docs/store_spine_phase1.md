# Phase 1 Store Spine - Documentation

## Overview

Phase 1 implements the persistent storage layer for EttleX, enabling:

- Durable SQLite-backed storage for Ettles and EPs
- Content-addressable storage (CAS) for EP content
- Seed Format v0 for bootstrapping canonical state
- Migration-driven schema evolution
- Provenance tracking for all imports

**Status**: ✅ Complete (57 tests passing)

## Architecture

```
┌─────────────────────────────────────────────────────────────┐
│                    ettlex-cli                                │
│                 (seed import command)                        │
└────────────────────────┬────────────────────────────────────┘
                         │
┌────────────────────────▼────────────────────────────────────┐
│                  ettlex-store                                │
│  ┌──────────────┐  ┌───────────┐  ┌─────────────────────┐  │
│  │ Seed Parser  │→ │ Importer  │→ │ Repository Layer    │  │
│  │ (YAML→Model) │  │ (Phase0.5)│  │ (Model→SQLite/CAS)  │  │
│  └──────────────┘  └───────────┘  └─────────────────────┘  │
│                         ↓                     ↓              │
│              ┌──────────────────┐   ┌────────────────┐      │
│              │  SQLite Schema   │   │  CAS (files)   │      │
│              │  (6 tables)      │   │  (sharded)     │      │
│              └──────────────────┘   └────────────────┘      │
└─────────────────────────────────────────────────────────────┘
                         ↑
┌────────────────────────┴────────────────────────────────────┐
│                   ettlex-core                                │
│        (Phase 0.5: in-memory ops, rendering)                │
└─────────────────────────────────────────────────────────────┘
```

## SQLite Schema

### Tables

**1. schema_version** - Migration tracking

```sql
CREATE TABLE schema_version (
    id INTEGER PRIMARY KEY,
    migration_id TEXT NOT NULL UNIQUE,
    applied_at INTEGER NOT NULL,
    checksum TEXT
);
```

**2. ettles** - Ettle entities

```sql
CREATE TABLE ettles (
    id TEXT PRIMARY KEY,
    title TEXT NOT NULL,
    parent_id TEXT,
    deleted INTEGER NOT NULL DEFAULT 0,
    created_at INTEGER NOT NULL,
    updated_at INTEGER NOT NULL,
    metadata TEXT,  -- JSON
    FOREIGN KEY (parent_id) REFERENCES ettles(id)
);

CREATE INDEX idx_ettles_parent_id ON ettles(parent_id);
```

**3. eps** - Ettle Partitions

```sql
CREATE TABLE eps (
    id TEXT PRIMARY KEY,
    ettle_id TEXT NOT NULL,
    ordinal INTEGER NOT NULL,
    normative INTEGER NOT NULL,
    child_ettle_id TEXT,
    content_digest TEXT,  -- SHA256 if CAS-backed
    content_inline TEXT,  -- If not CAS-backed
    deleted INTEGER NOT NULL DEFAULT 0,
    created_at INTEGER NOT NULL,
    updated_at INTEGER NOT NULL,
    UNIQUE (ettle_id, ordinal),
    FOREIGN KEY (ettle_id) REFERENCES ettles(id),
    FOREIGN KEY (child_ettle_id) REFERENCES ettles(id)
);

CREATE INDEX idx_eps_ettle_id ON eps(ettle_id);
CREATE INDEX idx_eps_ordinal ON eps(ettle_id, ordinal);
```

**4. facet_snapshots** - Snapshot facets (Phase 2)

```sql
CREATE TABLE facet_snapshots (
    id INTEGER PRIMARY KEY,
    snapshot_id TEXT NOT NULL,
    facet_kind TEXT NOT NULL,
    entity_id TEXT NOT NULL,
    digest TEXT NOT NULL,
    created_at INTEGER NOT NULL,
    UNIQUE (snapshot_id, facet_kind, entity_id)
);

CREATE INDEX idx_facet_snapshot ON facet_snapshots(snapshot_id);
```

**5. provenance_events** - Import tracking

```sql
CREATE TABLE provenance_events (
    id INTEGER PRIMARY KEY,
    kind TEXT NOT NULL,
    correlation_id TEXT NOT NULL,
    timestamp INTEGER NOT NULL,
    metadata TEXT  -- JSON
);

CREATE INDEX idx_provenance_correlation ON provenance_events(correlation_id);
```

**6. cas_blobs** - CAS index (non-load-bearing)

```sql
CREATE TABLE cas_blobs (
    digest TEXT PRIMARY KEY,
    relpath TEXT NOT NULL,
    size_bytes INTEGER NOT NULL,
    kind TEXT NOT NULL,
    verified_at INTEGER,
    status TEXT
);
```

### Key Constraints

- **Foreign Keys**: Enabled via `PRAGMA foreign_keys = ON`
- **Unique Constraints**: `(ettle_id, ordinal)` for EPs
- **Timestamps**: Unix epoch (i64)
- **Boolean Fields**: Stored as INTEGER (0/1)

## Content-Addressable Storage (CAS)

### Layout

```
.ettlex/cas/
├── ab/
│   └── abc123def456...sha256.txt
├── cd/
│   └── cdef789...sha256.json
└── ...
```

- **Sharding**: First 2 hex chars of SHA256 digest
- **Naming**: `<full-digest>.<extension>`
- **Atomicity**: Temp file → rename pattern
- **Collision Detection**: Byte-comparison on duplicate digest

### Operations

```rust
let cas = FsStore::new(".ettlex/cas");

// Write (atomic, collision-detecting)
let digest = cas.write(b"content", "txt")?;

// Read (extension-agnostic)
let content = cas.read(&digest)?;
```

## Seed Format v0

### Schema

```yaml
schema_version: 0 # Must be 0
project:
  name: 'project-name'

ettles:
  - id: 'ettle:identifier'
    title: 'Human Title'
    eps:
      - id: 'ep:identifier:ordinal'
        ordinal: 0 # Unique within ettle
        normative: true # or false
        why: 'Rationale'
        what: 'Description' # or { text: "Description" }
        how: 'Implementation'

links:
  - parent: 'ettle:parent-id'
    parent_ep: 'ep:parent-id:ordinal'
    child: 'ettle:child-id'
```

### Validation Rules

1. **Schema Version**: Must be `0`
2. **Ordinal Uniqueness**: No duplicate ordinals within an Ettle
3. **Referential Integrity**: All link references must exist
4. **EP Ownership**: `parent_ep` must belong to `parent` Ettle
5. **WHAT Polymorphism**: String or `{ text: "..." }` normalized to string

### Examples

**Minimal Seed**:

```yaml
schema_version: 0
project:
  name: bootstrap

ettles:
  - id: ettle:root
    title: 'Root Ettle'
    eps:
      - id: ep:root:0
        ordinal: 0
        normative: false
        why: 'Bootstrap'
        what: 'Initial state'
        how: 'Seed import'

links: []
```

**With Links**:

```yaml
schema_version: 0
project:
  name: example

ettles:
  - id: ettle:parent
    title: 'Parent'
    eps:
      - id: ep:parent:0
        ordinal: 0
        normative: true
        why: 'Define structure'
        what: 'Parent concept'
        how: 'Contains child'

  - id: ettle:child
    title: 'Child'
    eps:
      - id: ep:child:0
        ordinal: 0
        normative: true
        why: 'Refine parent'
        what: 'Child concept'
        how: 'Implements parent'

links:
  - parent: ettle:parent
    parent_ep: ep:parent:0
    child: ettle:child
```

## Migration Discipline

### Framework

- **Location**: `crates/ettlex-store/migrations/`
- **Naming**: `NNN_description.sql` (e.g., `001_initial_schema.sql`)
- **Checksums**: SHA256 stored in `schema_version` table
- **Gap Detection**: Sequential migration IDs required
- **Idempotency**: Re-running migrations is safe

### Adding Migrations

1. Create `migrations/NNN_description.sql`
2. Add SQL DDL statements
3. Checksums computed automatically on first run
4. Never modify existing migrations (add new ones instead)

### Running Migrations

```rust
use ettlex_store::migrations;

let mut conn = Connection::open("store.db")?;
migrations::apply_migrations(&mut conn)?;
```

## Invariants

### Determinism

1. **Reload Ordering**: SQLite queries use `ORDER BY` for deterministic results
2. **Seed Digest**: Canonical JSON representation (sorted keys)
3. **Render Stability**: Import → reload → render is byte-for-byte identical

### Immutability

1. **Ordinals**: Never change after EP creation
2. **IDs**: Seed IDs are stable across imports
3. **Migrations**: Existing migrations never modified

### Integrity

1. **Foreign Keys**: Enforced at database level
2. **Unique Constraints**: `(ettle_id, ordinal)` enforced
3. **Transactions**: All imports wrapped in SQLite transaction

## Bootstrap Workflow

### Initial Setup

```bash
# Clone repository
git clone <repo> && cd EttleX-Project/Rust

# Build
cargo build

# Create seed file
cat > my_seed.yaml <<EOF
schema_version: 0
project:
  name: my-project

ettles:
  - id: ettle:root
    title: "My Root"
    eps:
      - id: ep:root:0
        ordinal: 0
        normative: false
        why: "Start here"
        what: "First ettle"
        how: "Bootstrap"

links: []
EOF

# Import seed
cargo run -p ettlex-cli -- seed import my_seed.yaml
```

### Verification

```bash
# Check database
sqlite3 .ettlex/store.db "SELECT * FROM ettles;"

# Check provenance
sqlite3 .ettlex/store.db "SELECT * FROM provenance_events;"

# Round-trip test
cargo test -p ettlex-store --test round_trip_test
```

## CLI Usage

### Commands

```bash
# Import single seed
ettlex seed import path/to/seed.yaml

# Import directory (sorted)
ettlex seed import path/to/seeds/

# Future: --commit flag (Phase 2)
ettlex seed import seed.yaml --commit  # NotImplemented error
```

### Database Location

Default: `.ettlex/store.db` (relative to CWD)

## Testing

### Test Coverage

- **Unit Tests**: 35 (migrations, CAS, repo, seed parsing)
- **Integration Tests**: 22 (hydration, seed import, round-trip)
- **Total**: 57 tests passing

### Running Tests

```bash
# All tests
cargo test -p ettlex-store

# Specific suite
cargo test -p ettlex-store --test round_trip_test

# With coverage
cargo tarpaulin -p ettlex-store --out Html
```

### Key Test Scenarios

1. **Migrations**: Schema creation, idempotency, checksum validation
2. **CAS**: Atomic writes, collision detection, no partial writes
3. **Seed Import**: Valid/invalid seeds, rollback on failure
4. **Round-Trip**: Import → reload → render stability (ACCEPTANCE GATE)
5. **Determinism**: Reload ordering, digest stability

## Phase 1 Deliverables

✅ **Completed**:

- SQLite schema with 6 tables
- Migration framework with checksums
- CAS filesystem store (sharded, atomic)
- Seed Format v0 parser and validator
- Seed digest canonicalization
- Seed importer (Phase 0.5 integration)
- Repository layer (persist + hydrate)
- CLI seed import command
- Provenance tracking
- 57 tests passing
- Round-trip determinism verified

⏭️ **Deferred to Phase 2**:

- CAS index (cas_blobs upsert) - non-load-bearing
- Snapshot commit pipeline
- `--commit` flag implementation
- Git mirror synchronization

## Next Steps

1. **Phase 2 Planning**: Create seed for Phase 2 (snapshot commits)
2. **Self-Hosting**: Specify Phase 2 as seed → import → render → handoff markdown
3. **Automation**: CI/CD for test suite
4. **Performance**: Benchmark import/reload times

## Troubleshooting

### Common Issues

**Foreign Key Constraint Failure**:

- Ensure parent entities created before children
- Check link references point to existing Ettles/EPs

**Duplicate Ordinal Error**:

- Ordinals must be unique within an Ettle
- Check seed YAML for duplicate ordinal values

**Migration Checksum Mismatch**:

- Migration SQL file was modified after initial run
- Create new migration instead of modifying existing

**CAS Collision**:

- Two different contents produced same SHA256 (extremely unlikely)
- Check for file corruption

### Debug Mode

```bash
# Enable SQL logging
RUST_LOG=sqlx=debug cargo test -p ettlex-store

# Verbose test output
cargo test -p ettlex-store -- --nocapture
```

## References

- **Plan**: `/handoff/EttleX_Phase1_Store_Spine_Ettle_v1.md`
- **Crate**: `crates/ettlex-store/`
- **Tests**: `crates/ettlex-store/tests/`
- **Fixtures**: `crates/ettlex-store/tests/fixtures/`
- **Migrations**: `crates/ettlex-store/migrations/`

---

**Version**: Phase 1 Complete
**Date**: 2026-02-21
**Status**: 🎉 All acceptance gates passed
