# ettlex-engine

**EttleX Engine - Orchestration Layer**

High-level command orchestration coordinating between core domain logic and persistence layer.

## Overview

EttleX Engine provides the orchestration layer that sits between user-facing surfaces (CLI, MCP, Tauri) and the underlying domain/persistence layers. It handles:

- **Command orchestration**: Coordinating multi-step operations across layers
- **Transaction boundaries**: Managing atomic operations with proper rollback
- **Error contextualization**: Converting low-level errors to user-friendly messages
- **Lifecycle logging**: Operation start/end/error events with timing

## Architecture

The engine follows a command pattern with clear separation of concerns:

```
User Surface (CLI/MCP/Tauri)
    ↓
Engine Commands (orchestration + logging)
    ↓
Store Layer (persistence + CAS)
    ↓
Core Domain (pure logic)
```

## Module Structure

```
ettlex-engine/
└── src/
    └── commands/
        └── mod.rs          # Command registry
```

Planned command modules:

- `snapshot.rs` - Snapshot commit orchestration
- `preview.rs` - Preview generation
- `validation.rs` - Tree validation workflows
- `export.rs` - Multi-format export orchestration

## Command Pattern

Each engine command follows this pattern:

```rust
pub fn command_name(
    // Domain inputs
    arg1: &str,
    arg2: SomeType,
    // Infrastructure
    repo: &Repository,
) -> Result<CommandResult> {
    use ettlex_core::logging_facility::*;

    // Lifecycle logging: START
    log_op_start!("command_name", arg1 = arg1);

    let start = std::time::Instant::now();

    // Delegate to implementation
    let result = command_name_impl(arg1, arg2, repo)
        .map_err(|e| {
            let duration_ms = start.elapsed().as_millis() as u64;
            // Lifecycle logging: ERROR
            log_op_error!("command_name", e, duration_ms = duration_ms);
            e
        })?;

    let duration_ms = start.elapsed().as_millis() as u64;

    // Lifecycle logging: END
    log_op_end!("command_name", duration_ms = duration_ms, result_id = &result.id);

    Ok(result)
}
```

## Logging Ownership

The engine layer **owns lifecycle logging** for all high-level operations:

- **Engine**: `log_op_start!`, `log_op_end!`, `log_op_error!` (lifecycle)
- **Store**: `tracing::debug!()` only (implementation detail)
- **Core**: No logging (pure functions)

This ensures:

- Single source of truth for operation timing
- Consistent correlation ID propagation
- Clean separation between lifecycle and implementation logging

## Error Handling

The engine converts low-level errors to contextualized errors:

```rust
use ettlex_core::errors::{ExError, ExErrorKind};

// Core/Store error → Engine error with context
.map_err(|e| {
    ExError::new(ExErrorKind::Persistence)
        .with_op("snapshot_commit")
        .with_message("Failed to persist manifest to CAS")
        .with_source(e)
})?
```

Error kinds used by engine commands:

- `InvalidInput` - User-provided data is invalid
- `NotFound` - Referenced entity doesn't exist
- `AmbiguousSelection` - Multiple candidates found (requires disambiguation)
- `ConstraintViolation` - Operation violates domain constraints
- `Concurrency` - Optimistic lock failure
- `Persistence` - Storage operation failed
- `Invariant` - Domain invariant violated

## Snapshot Commit via Action Commands

The canonical way to commit a snapshot is via `EngineCommand::SnapshotCommit`:

```rust
use ettlex_engine::commands::engine_command::{EngineCommand, apply_engine_command};
use ettlex_engine::commands::snapshot::SnapshotOptions;

let cmd = EngineCommand::SnapshotCommit {
    leaf_ep_id: "ep:my-leaf:0".to_string(),
    policy_ref: "policy/default@0".to_string(),
    profile_ref: "profile/default@0".to_string(),
    options: SnapshotOptions {
        expected_head: None,
        dry_run: false,
    },
};

let result = apply_engine_command(cmd, &mut conn, &cas)?;
```

### Leaf-scoped semantics

- A **leaf EP** is an EP with no `child_ettle_id` (no refinement edge)
- Determined structurally, not by ordinal position
- Validation enforced at entry point (returns `ConstraintViolation` if EP has child)

### dry_run mode

Setting `dry_run: true` on `SnapshotOptions` performs a non-mutating simulation of the full
policy pipeline. The engine computes the EPT and manifest but performs no database writes and
no approval-request routing.

The result always contains a populated `constraint_resolution` field (`Option<DryRunConstraintResolution>`)
describing what the constraint resolution _would have_ produced:

| Profile state                                   | Result status                                                             |
| ----------------------------------------------- | ------------------------------------------------------------------------- |
| No constraints                                  | `Resolved`, `selected_profile_ref = None`, `candidates = []`              |
| 1 constraint                                    | `Resolved`, `selected_profile_ref = Some(...)`, `candidates = [id]`       |
| N constraints, `ChooseDeterministic`            | `Resolved`, `selected_profile_ref = Some(lex-first)`, `candidates` sorted |
| N constraints, `RouteForApproval` or `FailFast` | `RoutedForApproval`, `selected_profile_ref = None`, `candidates` sorted   |
| `predicate_evaluation_enabled = false`          | `Uncomputed`, no selection, empty candidates                              |

`constraint_resolution` is always `None` in non-dry-run (`Committed`) results.
`approval_token` is never present in dry-run results.

### Legacy root resolution

For backward compatibility, use `snapshot_commit_by_root_legacy()`:

```rust
use ettlex_engine::commands::snapshot::snapshot_commit_by_root_legacy;

let result = snapshot_commit_by_root_legacy(
    "ettle:root",
    "policy/default@0",
    "profile/default@0",
    SnapshotOptions { expected_head: None, dry_run: false },
    &mut conn,
    &cas,
)?;
```

**Resolution rules:**

- Resolves root Ettle to exactly one leaf EP
- Fails with `AmbiguousSelection` if multiple leaves exist
- Fails with `NotFound` if no leaves exist

## Repository Pattern

Engine commands receive a `Repository` struct containing all infrastructure:

```rust
pub struct Repository {
    pub conn: rusqlite::Connection,
    pub cas: ettlex_store::cas::FsStore,
    pub store: ettlex_core::ops::store::Store,
}
```

This allows commands to:

- Execute database transactions
- Read/write to CAS
- Access in-memory domain models

## Testing

Engine tests focus on orchestration and error handling:

```bash
cargo test -p ettlex-engine
```

Test structure:

- **Integration tests**: Full command execution with temp DB + CAS
- **Error path tests**: Verify error conversion and logging
- **Logging tests**: Verify lifecycle events emitted correctly

Example test:

```rust
use ettlex_core::logging_facility::test_capture::init_test_capture;

#[test]
fn test_command_logs_lifecycle() {
    let capture = init_test_capture();

    let result = some_command(...);

    let events = capture.events();
    capture.assert_event_exists("command_name", "start");
    capture.assert_event_exists("command_name", "end");

    assert!(result.is_ok());
}
```

## Transaction Management

Engine commands manage transaction boundaries:

```rust
pub fn atomic_operation(...) -> Result<()> {
    let tx = conn.transaction()?;

    // Multiple store operations
    store::operation_1(&tx, ...)?;
    store::operation_2(&tx, ...)?;

    // Commit or rollback
    tx.commit()?;

    Ok(())
}
```

**Transaction discipline**:

- One transaction per command (avoid nested transactions)
- Rollback on any error (automatic via `?` operator)
- CAS writes OUTSIDE transaction (idempotent, can't rollback)
- Ledger writes INSIDE transaction (must be atomic)

## Command Result Types

Commands return domain-specific result types:

```rust
#[derive(Debug, Clone)]
pub struct SnapshotCommitResult {
    pub snapshot_id: String,
    pub manifest_digest: String,
    pub semantic_manifest_digest: String,
    pub is_idempotent: bool,
}
```

Results include:

- Operation outcome (ID, digest, etc.)
- Metadata (timestamps, flags)
- No implementation details (no internal errors)

## Dependencies

Key dependencies:

- `ettlex-core` - Domain models, operations, logging facility
- `ettlex-store` - Persistence, CAS, repository layer
- `rusqlite` - Database transactions
- `tracing` - Implementation logging (debug level)

## Read-Only Query Surface (`engine_query.rs`)

The `apply_engine_query(query, &Connection, &FsStore)` function dispatches all read-only queries.
It never acquires `&mut Connection` and never writes to the DB or CAS.

### Query Variants

| Variant                                                               | Description                                                             |
| --------------------------------------------------------------------- | ----------------------------------------------------------------------- |
| `StateGetVersion`                                                     | Returns current state version and semantic head digest                  |
| `EttleGet { ettle_id }`                                               | Metadata + EP IDs for an Ettle                                          |
| `EttleList(opts)`                                                     | Paginated list of all Ettles                                            |
| `EttleListEps { ettle_id }`                                           | All EPs for an Ettle (ordered by ordinal)                               |
| `EpGet { ep_id }`                                                     | Single EP by ID                                                         |
| `EpListChildren { ep_id }`                                            | EPs in the child Ettle of an EP (via `child_ettle_id`)                  |
| `EpListParents { ep_id }`                                             | EPs whose Ettle is the parent of this EP's child Ettle                  |
| `EpListConstraints { ep_id }`                                         | Constraints attached to an EP (ordered by `ep_constraint_refs.ordinal`) |
| `EpListDecisions { ep_id, include_ancestors }`                        | Decisions for an EP; optionally walk parent Ettles                      |
| `ConstraintGet { constraint_id }`                                     | Single constraint by ID                                                 |
| `ConstraintListByFamily { family, include_tombstoned }`               | All constraints in a family                                             |
| `DecisionGet { decision_id }`                                         | Single decision by ID                                                   |
| `DecisionList(opts)`                                                  | Paginated list of all decisions                                         |
| `DecisionListByTarget { target_kind, target_id, include_tombstoned }` | Decisions for a target                                                  |
| `EttleListDecisions { ettle_id, include_eps, include_ancestors }`     | Decisions for an Ettle and its EPs                                      |
| `EptComputeDecisionContext { leaf_ep_id }`                            | Full decision context for an EPT chain                                  |
| `SnapshotGet { snapshot_id }`                                         | Single snapshot row                                                     |
| `SnapshotList { ettle_id }`                                           | All snapshots, optionally filtered by root Ettle                        |
| `ManifestGetBySnapshot { snapshot_id }`                               | Manifest bytes + digests for a snapshot                                 |
| `ManifestGetByDigest { manifest_digest }`                             | Manifest bytes from CAS directly                                        |
| `EptCompute { leaf_ep_id }`                                           | Compute the EPT for a leaf EP                                           |
| `ProfileGet { profile_ref }`                                          | Profile payload + digest                                                |
| `ProfileResolve { profile_ref }`                                      | Resolve profile (defaults if `None`)                                    |
| `ProfileGetDefault`                                                   | Explicit default-profile lookup                                         |
| `ProfileList(opts)`                                                   | Paginated profile listing                                               |
| `ApprovalGet { approval_token }`                                      | Approval payload + digests from CAS                                     |
| `ApprovalList(opts)`                                                  | Paginated approval listing                                              |
| `ApprovalListByKind { kind, options }`                                | Returns `NotImplemented` (Phase 1 deferred)                             |
| `ConstraintPredicatesPreview { … }`                                   | Non-mutating dry-run constraint predicate preview                       |
| `SnapshotDiff { a_ref, b_ref }`                                       | Diff two snapshots                                                      |

### Pagination

All list queries accept `ListOptions`:

```rust
pub struct ListOptions {
    pub limit: Option<usize>,         // default: 100
    pub cursor: Option<String>,       // opaque base64-encoded sort key
    pub prefix_filter: Option<String>,
    pub title_contains: Option<String>,
}
```

Results return `Page<T>` with `cursor: Option<String>` and `has_more: bool`.

### Error Contract

| Error kind                     | Meaning                                                  |
| ------------------------------ | -------------------------------------------------------- |
| `NotFound`                     | Generic entity missing                                   |
| `ProfileNotFound`              | Profile ref not found                                    |
| `ApprovalNotFound`             | Approval token not found                                 |
| `ApprovalStorageCorrupt`       | SQLite row exists but CAS blob is missing                |
| `RefinementIntegrityViolation` | EP has more than one structural parent                   |
| `MissingBlob`                  | CAS blob not found for a snapshot manifest digest        |
| `NotImplemented`               | Query is valid but deferred (e.g., `ApprovalListByKind`) |

## Future Work

Planned engine commands:

- [ ] `preview::generate()` - Generate previews for Ettles
- [ ] `validation::validate_tree()` - Full tree validation with reporting
- [ ] `export::to_json()` - Export to JSON projection
- [ ] `export::to_archimate()` - Export to ArchiMate projection
- [ ] `gc::collect_unreferenced()` - Garbage collect CAS blobs
