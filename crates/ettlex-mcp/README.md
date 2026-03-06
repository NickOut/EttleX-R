# ettlex-mcp

MCP (Model Context Protocol) thin-slice server for EttleX.

This crate is a **transport-only adapter** over `ettlex-engine`. It contains
no business logic — every tool call is delegated to the engine query or command
surface unchanged.

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
| `ettle_list_eps`                | List EPs for an ettle                                     |
| `ep_get`                        | Get an EP by ID                                           |
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

---

## Commands (via `ettlex_apply`)

All write operations go through `ettlex_apply` with a typed `command` payload:

```json
{ "command": { "tag": "EttleCreate", "title": "My Ettle" } }
```

| Tag                    | Fields                                                                  | Description                                   |
| ---------------------- | ----------------------------------------------------------------------- | --------------------------------------------- |
| `SnapshotCommit`       | `leaf_ep_id`, `policy_ref`, `profile_ref?`, `dry_run`, `expected_head?` | Commit a snapshot                             |
| `EttleCreate`          | `title`                                                                 | Create an ettle                               |
| `EpCreate`             | `ettle_id`, `ordinal`, `normative`, `why`, `what`, `how`                | Create an EP                                  |
| `ConstraintCreate`     | `constraint_id`, `family`, `kind`, `scope`, `payload_json`              | Create a constraint                           |
| `ConstraintAttachToEp` | `ep_id`, `constraint_id`, `ordinal`                                     | Attach a constraint to an EP                  |
| `ProfileCreate`        | `profile_ref`, `payload_json`, `source?`                                | Create a profile (idempotent on same content) |
| `ProfileSetDefault`    | `profile_ref`                                                           | Set the repository default profile            |

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
| `InvalidConstraintFamily` | Empty constraint family                           |
| `DuplicateAttachment`     | Constraint already attached to EP                 |
| `MissingBlob`             | CAS blob not found                                |
| `ResponseTooLarge`        | Response would exceed size limit                  |

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
