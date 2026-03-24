# Relations, Groups, and Relation Type Registry

*Added in Slice 02 (2026-03-22)*

---

## Overview

EttleX supports two mechanisms for structuring Ettles beyond the parent/child refinement tree:

- **Relations** — typed, directional links between Ettles
- **Groups** — named collections of Ettles with tombstonable membership

Both are governed by the **Relation Type Registry**, which defines what relation types
are valid and whether cycle detection applies.

---

## Relation Type Registry

The registry is seeded at migration time with four built-in types:

| Type           | Label             | Cycle check | Notes |
|---------------|-------------------|-------------|-------|
| `constraint`  | Constraint        | Yes         | Blocks EttleTombstone if active outgoing constraint relations exist |
| `realises`    | Realises          | Yes         | A→B means A realises B |
| `semantic_peer` | Semantic peer   | No          | Equivalence — no cycle concern |
| `depends_on`  | Depends on        | Yes         | Dependency ordering |

To query the registry via MCP:

```json
{ "tool_name": "ettle_list", ... }
```

> The registry is currently read-only. Future slices will expose `RelationTypeCreate` and
> `RelationTypeTombstone` commands.

---

## Creating a Relation

```json
{
  "command": {
    "tag": "RelationCreate",
    "relation_type": "constraint",
    "source_ettle_id": "ettle:my-service",
    "target_ettle_id": "ettle:my-rule",
    "properties_json": "{\"severity\": \"error\"}"
  }
}
```

Relations are identified by a `rel:…` prefixed UUID assigned by the engine.

### Invariants

- `source_ettle_id` and `target_ettle_id` must both exist and not be tombstoned
- Self-referential relations are rejected (`SelfReferentialLink`)
- For types with `cycle_check = true`, both direct and transitive cycles are rejected
- The caller may not supply `relation_id` (engine assigns it)

---

## Updating a Relation

Only `properties_json` can be updated. Source, target, and type are immutable.

```json
{
  "command": {
    "tag": "RelationUpdate",
    "relation_id": "rel:...",
    "properties_json": "{\"severity\": \"warning\"}"
  }
}
```

---

## Tombstoning a Relation

```json
{
  "command": {
    "tag": "RelationTombstone",
    "relation_id": "rel:..."
  }
}
```

Tombstoned relations are excluded from all list queries by default. Pass
`include_tombstoned: true` to include them.

---

## EttleTombstone and Constraint Relations

An `EttleTombstone` is **blocked** if the Ettle has any active outgoing relations of
type `constraint`. Tombstone the constraint relations first.

---

## Groups

Groups are named collections of Ettles. Membership is tombstonable (a member can be
re-added after removal).

### Create a group

```json
{ "command": { "tag": "GroupCreate", "name": "Frontend Services" } }
```

Groups are identified by a `grp:…` prefixed UUID.

### Add a member

```json
{
  "command": {
    "tag": "GroupMemberAdd",
    "group_id": "grp:...",
    "ettle_id": "ettle:my-service"
  }
}
```

Invariants:
- The group must exist and not be tombstoned
- The Ettle must exist and not be tombstoned
- Adding a member that already has an active membership is rejected (`AlreadyExists`)
- Re-adding after tombstoning is allowed (creates a new membership record)

### Remove a member

```json
{
  "command": {
    "tag": "GroupMemberRemove",
    "group_id": "grp:...",
    "ettle_id": "ettle:my-service"
  }
}
```

### Tombstone a group

A group can only be tombstoned if it has no active members. Remove all members first.

```json
{ "command": { "tag": "GroupTombstone", "group_id": "grp:..." } }
```

---

## MCP Read Tools (Slice 02b)

Five read tools are available directly via MCP (bypass `ettlex_apply`, do not increment `state_version`):

### `relation_get`

```json
{ "relation_id": "rel:..." }
```

Returns: `{ relation_id, source_ettle_id, target_ettle_id, relation_type, properties_json, created_at, tombstoned_at }`.
Returns the record even if tombstoned. Returns `NotFound` if not found.

### `relation_list`

```json
{ "source_ettle_id": "ettle:...", "target_ettle_id": "ettle:...", "include_tombstoned": false, "limit": 100, "cursor": "..." }
```

At least one of `source_ettle_id` or `target_ettle_id` must be supplied (returns `InvalidInput` otherwise).
Returns `{ items: [...], cursor? }` with cursor-based pagination.

### `group_get`

```json
{ "group_id": "grp:..." }
```

Returns: `{ group_id, name, created_at, tombstoned_at }`.
Returns the record even if tombstoned. Returns `NotFound` if not found.

### `group_list`

```json
{ "include_tombstoned": false, "limit": 100, "cursor": "..." }
```

Returns `{ items: [{ group_id, name, created_at, tombstoned_at }], cursor? }`.

### `group_member_list`

```json
{ "group_id": "grp:...", "ettle_id": "ettle:...", "include_tombstoned": false, "limit": 100, "cursor": "..." }
```

At least one of `group_id` or `ettle_id` must be supplied (returns `InvalidInput` otherwise).
Returns `{ items: [{ id, group_id, ettle_id, created_at, tombstoned_at }], cursor? }`.

## Read Queries (Engine layer)

The following engine handler functions are available for relations and groups:

| Function | Description |
|----------|-------------|
| `handle_relation_get(conn, relation_id)` | Fetch a single relation |
| `handle_relation_list(conn, source?, target?, relation_type?, include_tombstoned)` | List relations (at least one filter required) |
| `handle_group_get(conn, group_id)` | Fetch a single group |
| `handle_group_list(conn, include_tombstoned)` | List all groups |
| `handle_group_member_list(conn, group_id, include_tombstoned)` | List members of a group by group_id |

---

## Error Codes

| Code | Trigger |
|------|---------|
| `InvalidInput` | Unknown `relation_type`, empty group name, missing required fields |
| `NotFound` | Source/target Ettle or relation/group not found |
| `AlreadyTombstoned` | Operating on an already-tombstoned entity |
| `SelfReferentialLink` | `source_ettle_id == target_ettle_id` |
| `HasActiveDependants` | `GroupTombstone` while active members exist |
| `CycleDetected` | `RelationCreate` would introduce a directed cycle |
