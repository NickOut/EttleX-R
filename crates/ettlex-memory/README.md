# ettlex-memory

**EttleX Memory Manager — Ettle Context Assembly**

Aggregation layer that assembles structured context from `ettlex-store` and exposes a
command entry point for MCP without a direct `ettlex-engine` dependency.

## Purpose

`ettlex-memory` sits between `ettlex-mcp` and `ettlex-engine`:

```
ettlex-mcp
    ↓
ettlex-memory      ← this crate
    ↓
ettlex-engine / ettlex-store
```

This keeps `ettlex-mcp` free of a direct `ettlex-engine` link, enforcing the thin-adapter
principle and enabling future substitution of the storage backend.

## Public API

### `MemoryManager`

```rust
use ettlex_memory::memory_manager::MemoryManager;
use ettlex_store::cas::FsStore;
use rusqlite::Connection;

let mm = MemoryManager::new(conn, cas, policy_provider);

// Apply a write command (delegates to ettlex-engine apply_command)
let (result, new_state_version) = mm.apply_command(cmd, expected_state_version)?;

// Assemble a structured context for an Ettle (read-only)
let ctx = mm.assemble_ettle_context(ettle_id)?;
```

### `assemble_ettle_context`

Returns an `EttleContext` containing:

- Ettle metadata (id, title, why/what/how)
- All active EPs (ordered by ordinal)
- All active group memberships for the Ettle

Used by agents to assemble a prompt-ready description of an Ettle's current state.

## Error Handling

All public APIs return `Result<T, ExError>`. See `ettlex-errors` for the canonical
error taxonomy.

## Dependencies

- `ettlex-engine` — write command dispatch
- `ettlex-store` — read queries for context assembly
- `ettlex-core` — domain types
- `ettlex-errors` — error types
