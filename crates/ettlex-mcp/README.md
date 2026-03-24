# ettlex-mcp

MCP (Model Context Protocol) thin-slice server for EttleX.

This crate is a **transport-only adapter** over `ettlex-memory`. It contains
no business logic — every tool call is delegated to the engine query or command
surface unchanged via `MemoryManager`.

> **Architecture note (Slice 02):** `ettlex-mcp` no longer depends directly on
> `ettlex-engine`. All write commands route through `ettlex-memory::MemoryManager`.
> Read queries are dispatched via `apply_engine_query` from `ettlex-engine` (accessed
> through `ettlex-memory`'s re-export).

---

## Tool Surface

### Write tools

| Tool           | Description                                       |
| -------------- | ------------------------------------------------- |
| `ettlex_apply` | Apply a write command (see [Commands](#commands)) |

### Read tools

| Tool                            | Description                                               |
| ------------------------------- | --------------------------------------------------------- |
| `ettle_get`                     | Get an ettle by ID                                        |
| `ettle_list`                    | List ettles (paginated)                                   |
| `snapshot_get`                  | Get a snapshot ledger row                                 |
| `snapshot_list`                 | List snapshots (paginated)                                |
| `snapshot_get_head`             | Get manifest digest of the most recent committed snapshot |
| `snapshot_get_manifest`         | Get raw manifest bytes for a snapshot                     |
| `snapshot_diff`                 | Compute a structured diff between two snapshots           |
| `policy_get`                    | Get a policy document by reference                        |
| `policy_list`                   | List available policies (paginated)                       |
| `policy_project_for_handoff`    | Project a policy for code-generator handoff               |
| `profile_get`                   | Get a profile by reference                                |
| `profile_list`                  | List profiles (paginated)                                 |
| `profile_get_default`           | Get the default profile                                   |
| `approval_get`                  | Get an approval request by token                          |
| `constraint_predicates_preview` | Preview constraint predicate resolution (read-only)       |
| `relation_get`                  | Get a relation by ID (returns even if tombstoned)         |
| `relation_list`                 | List relations by source/target ettle (paginated)         |
| `group_get`                     | Get a group by ID (returns even if tombstoned)            |
| `group_list`                    | List groups (paginated)                                   |
| `group_member_list`             | List group memberships by group/ettle (paginated)         |

---

## Commands (via `ettlex_apply`)

All write operations go through `ettlex_apply` with a typed `command` payload:

```json
{ "command": { "tag": "EttleCreate", "title": "My Ettle", "why": "...", "what": "...", "how": "..." } }
```

### Ettle commands

| Tag              | Fields                                                       | Description                    |
| ---------------- | ------------------------------------------------------------ | ------------------------------ |
| `EttleCreate`    | `title`, `why?`, `what?`, `how?`, `reasoning_link_id?`, `reasoning_link_type?` | Create an Ettle |
| `EttleUpdate`    | `ettle_id`, `title?`, `why?`, `what?`, `how?`, `reasoning_link_id?`, `reasoning_link_type?` | Update an Ettle |
| `EttleTombstone` | `ettle_id`                                                   | Tombstone an Ettle             |

### Relation commands

| Tag                 | Fields                                                                    | Description              |
| ------------------- | ------------------------------------------------------------------------- | ------------------------ |
| `RelationCreate`    | `relation_type`, `source_ettle_id`, `target_ettle_id`, `properties_json?` | Create a relation        |
| `RelationUpdate`    | `relation_id`, `properties_json`                                          | Update relation properties |
| `RelationTombstone` | `relation_id`                                                             | Tombstone a relation     |

### Group commands

| Tag               | Fields                           | Description                |
| ----------------- | -------------------------------- | -------------------------- |
| `GroupCreate`     | `name`                           | Create a group             |
| `GroupTombstone`  | `group_id`                       | Tombstone a group          |
| `GroupMemberAdd`  | `group_id`, `ettle_id`           | Add an Ettle to a group    |
| `GroupMemberRemove` | `group_id`, `ettle_id`         | Remove an Ettle from a group |

### Snapshot and governance commands

| Tag                 | Fields                                                                  | Description                                   |
| ------------------- | ----------------------------------------------------------------------- | --------------------------------------------- |
| `SnapshotCommit`    | `leaf_ep_id`, `policy_ref?`, `profile_ref?`, `dry_run`, `expected_head?` | Commit a snapshot                            |
| `ProfileCreate`     | `profile_ref`, `payload_json`, `source?`                                | Create a profile (idempotent on same content) |
| `ProfileSetDefault` | `profile_ref`                                                           | Set the repository default profile            |
| `PolicyCreate`      | `policy_ref`, `text`                                                    | Create a policy document                      |

### Response shape

```json
{
  "new_state_version": 42,
  "result": { "tag": "EttleCreate", "ettle_id": "ettle:..." }
}
```

### Optimistic Concurrency Control (OCC)

Pass `expected_state_version` to guard against concurrent mutations:

```json
{
  "command": { "tag": "EttleCreate", "title": "Safe" },
  "expected_state_version": 41
}
```

Returns `HeadMismatch` if the current `state_version` differs.

---

## Error Contract

All errors have the shape `{ error_code: String, message: String }`.

| Code                      | Meaning                                           |
| ------------------------- | ------------------------------------------------- |
| `AuthRequired`            | Missing or invalid bearer token                   |
| `ToolNotFound`            | Unknown tool name                                 |
| `InvalidCursor`           | Malformed pagination cursor                       |
| `InvalidCommand`          | Unknown command tag                               |
| `InvalidInput`            | Missing required fields or bad values             |
| `RequestTooLarge`         | Payload exceeds size limit                        |
| `HeadMismatch`            | OCC version mismatch                              |
| `NotFound`                | Entity not found                                  |
| `NotALeaf`                | SnapshotCommit on a non-leaf EP                   |
| `PolicyDenied`            | Policy rejected the snapshot                      |
| `PolicyNotFound`          | Unknown policy reference                          |
| `ProfileNotFound`         | Unknown profile reference                         |
| `ProfileConflict`         | ProfileCreate with different content for same ref |
| `ApprovalNotFound`        | Unknown approval token                            |
| `MissingBlob`             | CAS blob not found                                |
| `ResponseTooLarge`        | Response would exceed size limit                  |
| `PolicyConflict`          | PolicyCreate attempted on existing policy_ref     |
| `SelfReferentialLink`     | RelationCreate source and target are the same     |
| `AlreadyTombstoned`       | Entity is already tombstoned                      |
| `HasActiveDependants`     | Tombstone blocked by active dependants            |

---

## Authentication

Configure via `AuthConfig`:

```rust
// Require a token on every request
AuthConfig::with_token("t:dev")

// Disable auth (development only)
AuthConfig::disabled()
```

The token is passed as `auth_token` on `McpToolCall`. Missing or incorrect
tokens return `AuthRequired` before any tool routing occurs.

---

## Pagination

List tools support `limit` and `cursor` params:

```json
{ "limit": 50, "cursor": "<opaque-base64-string>" }
```

The default limit is 100. Responses include a `cursor` field when more items exist.
