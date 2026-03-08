# Completion Report: ep:store ordinal 3 — Identity Contract + PolicyCreate

**EP:** `ep:store:3`
**Date completed:** 2026-03-08
**Classification:** A (New Behaviour: PolicyCreate) + C (Behavioural Modification: identity enforcement)

---

## Summary

Implemented two related concerns in the store ordinal 3 EP:
1. **Identity contract** — server-generated IDs enforced at the action layer; caller-supplied IDs rejected.
2. **PolicyCreate command** — new McpCommand variant + FilePolicyProvider implementation.

---

## Production Changes

### ExErrorKind::PolicyConflict
- Added `PolicyConflict` variant to `ExErrorKind` in `ettlex-core/src/errors.rs`
- Code string: `"PolicyConflict"`

### Identity Contract (EttleCreate / EpCreate)
- `McpCommand::EttleCreate` gained `ettle_id: Option<String>` field
- `McpCommand::EpCreate` gained `ep_id: Option<String>` field
- `dispatch_mcp_command` rejects `Some(ettle_id)` with `InvalidInput` before any store interaction
- `dispatch_mcp_command` rejects `Some(ep_id)` with `InvalidInput` before any store interaction
- Generated IDs use ULIDv7; format validated via regex in tests

### PolicyCreate Command
- `PolicyProvider` trait gained `fn policy_create(&self, policy_ref: &str, text: &str) -> Result<()>`
  - Default impl returns `Err(ExError::new(ExErrorKind::NotImplemented))`
- `FilePolicyProvider` implements `policy_create`:
  - Validates non-empty `policy_ref` containing `@` separator
  - Validates non-empty `text`
  - Returns `PolicyConflict` if file already exists
  - Writes via temp file + atomic rename (no partial writes)
- `McpCommand::PolicyCreate { policy_ref: String, text: String }` added
- `McpCommandResult::PolicyCreate { policy_ref: String }` added
- `dispatch_mcp_command` dispatches to `policy_provider.policy_create()` and records to `mcp_command_log`

### Acknowledged Limitation
Cross-system atomicity gap: if the file write succeeds but the SQLite log insert fails, the policy
file exists without a corresponding `state_version` increment. This is idempotent — a retry returns
`PolicyConflict` rather than duplicating the file. Recorded here as an acknowledged design limitation.

---

## Scenarios Implemented

### Identity Contract (`ettlex-engine/tests/identity_contract_tests.rs`)

| ID | Scenario |
|---|---|
| S-ID-1 | EttleCreate with no ettle_id generates ULID and returns it |
| S-ID-2 | EpCreate with no ep_id generates ULID and returns it |
| S-ID-3 | EttleCreate rejects supplied ettle_id → InvalidInput |
| S-ID-4 | EpCreate rejects supplied ep_id → InvalidInput |
| S-ID-5 | EttleCreate with empty title fails → InvalidInput |
| S-ID-6 | EpCreate referencing missing ettle fails → NotFound |
| S-ID-7 | EttleCreate with max-length title succeeds |
| S-ID-8 | EpCreate with ordinal conflict fails → OrdinalConflict |
| S-ID-9 | Generated ettle_id matches ULID format |
| S-ID-10 | Generated ep_id matches ULID format |
| S-ID-11 | Two successive EttleCreate calls produce distinct ettle_ids |
| S-ID-12 | Repeated EttleCreate with identical title produces distinct Ettles |
| S-ID-13 | EttleCreate followed by ettle.get returns consistent state |
| S-ID-14 | EttleCreate MUST NOT silently discard supplied ettle_id |
| S-ID-15 | EpCreate MUST NOT silently discard supplied ep_id |

### PolicyCreate (`ettlex-engine/tests/policy_create_tests.rs`)

| ID | Scenario |
|---|---|
| S-PC-1 | PolicyCreate with valid policy_ref and text succeeds, state_version+1 |
| S-PC-2 | PolicyCreate rejects duplicate policy_ref → PolicyConflict |
| S-PC-3 | PolicyCreate rejects empty text → InvalidInput |
| S-PC-4 | PolicyCreate rejects empty policy_ref → InvalidInput |
| S-PC-5 | PolicyCreate rejects malformed policy_ref (no `@`) → InvalidInput |
| S-PC-6 | PolicyCreate on write failure → neither policy persisted nor state_version changed |
| S-PC-7 | PolicyCreate with max-length policy_ref succeeds |
| S-PC-8 | PolicyCreate with large text body (100 KB) succeeds |
| S-PC-9 | policy_ref is stable retrieval key — policy.get returns exactly that policy |
| S-PC-10 | PolicyCreate NOT idempotent — second identical call → PolicyConflict |
| S-PC-11 | After PolicyCreate, policy.list includes new policy |
| S-PC-12 | After PolicyCreate, SnapshotCommit can reference new policy_ref |
| S-PC-13 | PolicyCreate success reflected in state.get_version (V+1) |
| S-PC-14 | Existing file-backed policies remain retrievable after PolicyCreate |
| S-PC-15 | PolicyCreate MUST NOT overwrite existing policy |

---

## Formally Deferred Constraints

| Constraint | Rationale | Location |
|---|---|---|
| S-ID concurrent: Concurrent EttleCreate producing distinct ULIDs | Requires thread-parallel test harness; deferred to load test phase | `// DEFERRED:` comment in `identity_contract_tests.rs` |
| S-ID-15 (seed import): Seed importer rejects caller-supplied ettle_id | Seed importer is a separate subsystem; scoped separately | `// DEFERRED:` comment in `identity_contract_tests.rs` |
| S-PC concurrent: Concurrent PolicyCreate race | File-system atomic rename + SQLite uniqueness each guarantee single-winner; cross-system race is OS-level | `// DEFERRED:` comment in `policy_create_tests.rs` |

---

## Acceptance Gates

- `make lint` — PASS
- `make test` — 841/841 PASS, 3 skipped
- `make coverage-check` — 87% ≥ 80% threshold PASS
- `make doc` — PASS
