# ettlex-agent-api

**EttleX Agent API — Public API surface for agent consumers of EttleX.**

This crate provides the full public CRUD API for Ettle, Relation, and Group operations,
intended for use by AI agents or other consumers that should not depend directly on
`ettlex-engine` or `ettlex-store`.

## Architecture

```
agent implementations / callers
    ↓
ettlex-agent-api       ← this crate
    ↓
ettlex-memory          ← only workspace dependency (re-exports engine + store types)
    ↓
ettlex-engine / ettlex-store / ettlex-core
```

By depending only on `ettlex-memory`, agent code never directly references
`ettlex-engine`, `ettlex-store`, or `ettlex-core`.

## Modules

- `operations::ettle` — Ettle read/write operations (`agent_ettle_get`, `agent_ettle_create`, etc.)
- `operations::relation` — Relation read/write operations (`agent_relation_get`, `agent_relation_create`, etc.)
- `operations::group` — Group and group-member read/write operations
- `boundary::mapping` — Single designated boundary for ExError → display mapping

## Key Conventions

### Routing invariant
All write operations route through `ettlex_memory::apply_command`. Read operations call
`SqliteRepo` directly via `ettlex_memory::SqliteRepo`.

### Lifecycle logging
Each public function emits exactly one `start` event and one `end`/`end_error` event.
WHY/WHAT/HOW content MUST NOT appear as log field values.

### OCC
Write functions accept `expected_state_version: Option<u64>`. Pass `None` to skip OCC,
or `Some(v)` to assert that the current state version equals `v` before executing.

### Cursor encoding
`agent_ettle_list` accepts an opaque base64 URL-safe-no-pad cursor string. The cursor is
decoded and forwarded to `SqliteRepo::list_ettles`. Encoding is handled by the store layer.

### Filter requirement
- `agent_relation_list` requires at least one of `source_ettle_id`, `target_ettle_id`, or `relation_type`.
- `agent_group_member_list` requires at least one of `group_id` or `ettle_id`.

## Dependencies

This crate depends only on `ettlex-memory` (and standard utilities: `base64`, `serde_json`,
`tracing`). It MUST NOT depend on `ettlex-engine`, `ettlex-store`, or `ettlex-core` directly.
