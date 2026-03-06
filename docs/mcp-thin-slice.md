# MCP Thin-Slice Server

## Overview

`ettlex-mcp` exposes the EttleX engine as a **Model Context Protocol (MCP)**
server. It is a thin transport adapter — no business logic lives here. Every
tool call is delegated directly to `ettlex-engine` read or write commands.

This document covers the authoring workflow, available tools, and error contract
for AI agents interacting with an EttleX repository via MCP.

---

## Authoring workflow

A typical agent workflow for committing an EP snapshot:

1. **Read current state** — call `ettle.list` or `ettle.get` to locate the
   target ettle and its leaf EP ID.
2. **Check policies** — call `policy.list` and `policy.get` to understand
   applicable policies; optionally `policy.project_for_handoff` to obtain
   structured obligations.
3. **Create constraints** (if needed) — `ettlex.apply ConstraintCreate` then
   `ettlex.apply ConstraintAttachToEp`.
4. **Commit snapshot** — `ettlex.apply SnapshotCommit{ leaf_ep_id, policy_ref }`.
   - On success: `result.tag == "SnapshotCommit"` with `snapshot_id` and
     `manifest_digest`.
   - On routed approval: `result.tag == "RoutedForApproval"` with
     `approval_token`.
5. **Verify** — call `snapshot.get_head` or `snapshot.get_manifest` to confirm.

---

## Tool Reference

### `ettlex.apply`

The single write endpoint. All mutations go through this tool.

**Params:**

```json
{
  "command": { "tag": "...", ...fields },
  "expected_state_version": 41
}
```

`expected_state_version` is optional; omit for unconditional writes. When
provided, the server checks the current state version and returns `HeadMismatch`
if it differs.

**Response:**

```json
{
  "new_state_version": 42,
  "result": { "tag": "...", ...fields }
}
```

#### Commands

| Tag                    | Required fields                                            | Optional fields                                  |
| ---------------------- | ---------------------------------------------------------- | ------------------------------------------------ |
| `SnapshotCommit`       | `leaf_ep_id`, `policy_ref`                                 | `profile_ref`, `dry_run`, `expected_head`        |
| `EttleCreate`          | `title`                                                    | —                                                |
| `EpCreate`             | `ettle_id`, `ordinal`                                      | `normative` (default true), `why`, `what`, `how` |
| `ConstraintCreate`     | `constraint_id`, `family`, `kind`, `scope`, `payload_json` | —                                                |
| `ConstraintAttachToEp` | `ep_id`, `constraint_id`, `ordinal`                        | —                                                |
| `ProfileCreate`        | `profile_ref`, `payload_json`                              | `source`                                         |
| `ProfileSetDefault`    | `profile_ref`                                              | —                                                |

---

### `ettle.get`

Get a single ettle and its EP IDs.

**Params:** `{ "ettle_id": "ettle:..." }`

**Response:** `{ "ettle_id", "title", "parent_id", "ep_ids": [...], "created_at" }`

---

### `ettle.list`

List ettles with pagination.

**Params:** `{ "limit": 100, "cursor": "<opaque>" }`

**Response:** `{ "items": [...], "cursor": "<opaque>" }`

---

### `ettle.list_eps`

List EPs belonging to an ettle.

**Params:** `{ "ettle_id": "ettle:..." }`

**Response:** `{ "items": [{ "id", "ettle_id", "ordinal", "normative" }] }`

---

### `ep.get`

Get a single EP.

**Params:** `{ "ep_id": "ep:..." }`

**Response:** `{ "id", "ettle_id", "ordinal", "normative", "why", "what", "how", "child_ettle_id" }`

---

### `snapshot.get_head`

Get the manifest digest of the most recently committed snapshot for an ettle.

**Params:** `{ "ettle_id": "ettle:..." }`

**Response:** `{ "manifest_digest": "<sha256>" }`

---

### `snapshot.get_manifest`

Get raw manifest bytes for a snapshot.

**Params:** `{ "snapshot_id": "snapshot:..." }`

**Response:** `{ "snapshot_id", "manifest_digest", "manifest_bytes": "<json-string>" }`

---

### `snapshot.diff`

Compute a structured diff between two snapshots.

**Params:** `{ "a_snapshot_id": "snapshot:...", "b_snapshot_id": "snapshot:..." }`

**Response:** `{ "identity": { "a_manifest_digest", "b_manifest_digest" }, "human_summary": "<markdown>" }`

---

### `policy.get`

Read a policy document.

**Params:** `{ "policy_ref": "policy/name@version" }`

**Response:** `{ "policy_ref", "text": "<full-document>" }`

---

### `policy.project_for_handoff`

Produce a deterministic byte projection of a policy's HANDOFF obligations,
suitable for passing to a code-generator prompt.

**Params:** `{ "policy_ref": "policy/name@version", "profile_ref": null }`

**Response:** `{ "policy_ref", "profile_ref", "projection": "<text>" }`

---

### `profile.get` / `profile.list` / `profile.get_default`

Query profile configuration.

`profile.get` params: `{ "profile_ref": "profile/name@version" }`

`profile.get_default` params: `{}`

Both respond with: `{ "profile_ref", "profile_digest", "payload": {...} }`

---

### `approval.get`

Get an approval request.

**Params:** `{ "approval_token": "approval:..." }`

---

### `constraint_predicates.preview`

Preview constraint predicate resolution without any side-effects.

**Params:** `{ "profile_ref": "...", "context": {}, "candidates": ["c:1", "c:2"] }`

**Response:** `{ "status": "Selected|NoMatch|Ambiguous|RoutedForApproval", "selected": "c:1", "candidates": [...] }`

---

## Error Contract

| Code                      | HTTP analogue | Description                     |
| ------------------------- | ------------- | ------------------------------- |
| `AuthRequired`            | 401           | Missing or invalid bearer token |
| `ToolNotFound`            | 404           | Unknown tool name               |
| `RequestTooLarge`         | 413           | Payload exceeds 1 MB            |
| `InvalidCursor`           | 400           | Malformed pagination cursor     |
| `InvalidCommand`          | 400           | Unknown command tag             |
| `InvalidInput`            | 400           | Missing required fields         |
| `HeadMismatch`            | 409           | OCC state_version mismatch      |
| `NotFound`                | 404           | Entity not found                |
| `NotALeaf`                | 422           | SnapshotCommit on a non-leaf EP |
| `PolicyDenied`            | 403           | Policy rejected the operation   |
| `PolicyNotFound`          | 404           | Unknown policy reference        |
| `ProfileNotFound`         | 404           | Unknown profile reference       |
| `ProfileConflict`         | 409           | ProfileCreate content mismatch  |
| `ApprovalNotFound`        | 404           | Unknown approval token          |
| `InvalidConstraintFamily` | 400           | Empty constraint family         |
| `DuplicateAttachment`     | 409           | Constraint already attached     |
| `MissingBlob`             | 500           | CAS blob not found              |
| `ResponseTooLarge`        | 413           | Response exceeds size limit     |

---

## Invariants

- **MCP never injects business logic.** Query results are delegated byte-for-byte
  from the engine.
- **All writes go through `ettlex.apply`.** The MCP layer has no direct store
  access.
- **Query tools are read-only.** They use `&Connection` and never write to the
  DB, CAS, or ledger.
- **`profile_ref` is not defaulted by MCP.** If absent in `SnapshotCommit`, it
  is forwarded as `None` to the engine; defaulting is the engine's responsibility.
- **Canonical JSON.** All responses use sorted key ordering for determinism.
