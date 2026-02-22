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
- `Concurrency` - Optimistic lock failure
- `Persistence` - Storage operation failed
- `Invariant` - Domain invariant violated

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

## Future Work

Planned engine commands:

- [ ] `snapshot::commit()` - Snapshot commit with manifest generation
- [ ] `snapshot::diff()` - Diff two snapshots
- [ ] `preview::generate()` - Generate previews for Ettles
- [ ] `validation::validate_tree()` - Full tree validation with reporting
- [ ] `export::to_json()` - Export to JSON projection
- [ ] `export::to_archimate()` - Export to ArchiMate projection
- [ ] `gc::collect_unreferenced()` - Garbage collect CAS blobs
