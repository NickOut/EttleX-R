# Completion Report: ep:snapshot_commit_actions_refactor ordinal 1 — policy_ref Optional

**EP:** `ep:snapshot_commit_actions_refactor:1`
**Date completed:** 2026-03-08
**Classification:** C — Behavioural Modification (policy_ref: String → Option<String> on SnapshotCommit)

---

## Summary

Made `policy_ref` optional on `SnapshotCommit`. Absent policy_ref triggers default policy resolution
in the action layer; if no default is configured, the commit proceeds with permissive pass-through
(empty string stored as manifest policy_ref, no policy check applied).

---

## Production Changes

### `ettlex-engine/src/commands/engine_command.rs`
- `EngineCommand::SnapshotCommit.policy_ref`: `String` → `Option<String>`

### `ettlex-engine/src/commands/mcp_command.rs`
- `McpCommand::SnapshotCommit.policy_ref`: `String` → `Option<String>`

### `ettlex-engine/src/commands/snapshot.rs` — `snapshot_commit_by_leaf`
- Signature: `policy_ref: Option<&str>` (was `&str`)
- Removed Step 1a `PolicyRefMissing` guard entirely
- Resolution logic (action layer):
  - `Some(ref)` → use directly
  - `None` → call `policy_provider.get_default_policy_ref()`:
    - `Some(default)` → use default ref
    - `None` → permissive pass-through (`resolved_policy_ref = ""`)
- Manifest `policy_ref` field = `resolved_policy_ref` (empty string for permissive pass-through)

### `ettlex-core/src/policy_provider.rs`
- Added `fn get_default_policy_ref(&self) -> Option<String>` to `PolicyProvider` trait
- Default impl: `None`
- `FilePolicyProvider`, `NoopPolicyProvider`, `DenyAllPolicyProvider`: all return `None`
  (configurable default policy is a future EP)

### Callsite Updates
All test files and CLI code updated from `policy_ref: "...".to_string()` to `policy_ref: Some("...".to_string())`:
- `ettlex-engine/tests/policy_provider_tests.rs` — commit_cmd helper + S13 test rewritten
- `ettlex-engine/tests/snapshot_commit_by_leaf_tests.rs`
- `ettlex-engine/tests/snapshot_commit_legacy_resolution_tests.rs`
- `ettlex-engine/tests/snapshot_commit_idempotency_tests.rs`
- `ettlex-engine/tests/snapshot_commit_determinism_tests.rs`
- `ettlex-engine/tests/snapshot_commit_policy_tests.rs`
- `ettlex-engine/tests/policy_create_tests.rs`
- `ettlex-cli/src/commands/snapshot.rs`

### S13 Test Behaviour Change
`test_s13_empty_policy_ref_returns_policy_ref_missing` was rewritten to
`test_s13_absent_policy_ref_permissive_pass_through` — now asserts that absent `policy_ref`
results in a successful commit (permissive pass-through), not `PolicyRefMissing`.

---

## Scenarios Implemented (`ettlex-engine/tests/policy_ref_optional_tests.rs`)

| ID | Scenario |
|---|---|
| S-PR-1 | SnapshotCommit succeeds with policy_ref absent, no default → permissive pass-through |
| S-PR-2 | SnapshotCommit succeeds with explicit policy_ref |
| S-PR-4 | SnapshotCommit with explicit policy_ref that doesn't exist → PolicyNotFound |
| S-PR-6 | SnapshotCommit with None policy_ref behaves identically to absent |
| S-PR-7 | Manifest always records policy_ref field (empty string if permissive) |
| S-PR-8 | Explicit policy_ref takes precedence over default |
| S-PR-10 | SnapshotCommit with absent policy_ref transitions to committed state |
| S-PR-11 | Result tag is SnapshotCommitted with permissive pass-through |
| S-PR-12 | Existing calls with explicit policy_ref continue to work unchanged |
| S-PR-14 | Manifest bytes byte-stable for identical state + absent policy_ref |

---

## Formally Deferred Constraints

| Constraint | Rationale | Location |
|---|---|---|
| S-PR-3: `get_default_policy_ref` returning non-None | Phase 1: always returns None; configurable default policy is a future EP | `// DEFERRED:` comment in `policy_ref_optional_tests.rs` |
| S-PR-5: absent policy_ref + default policy that denies → PolicyDenied | Requires get_default_policy_ref returning non-None; same deferral | `// DEFERRED:` comment in `policy_ref_optional_tests.rs` |
| S-PR-9: Absent policy_ref defaulting is deterministic | Covered structurally by S-PR-14; formal multi-run test deferred | `// DEFERRED:` comment in `policy_ref_optional_tests.rs` |
| S-PR-13: MCP does not inject policy_ref | Verified structurally (Option<String> type); formal MCP transport test is low priority | `// DEFERRED:` comment in `policy_ref_optional_tests.rs` |

---

## Acceptance Gates

- `make lint` — PASS
- `make test` — 841/841 PASS, 3 skipped
- `make coverage-check` — 87% ≥ 80% threshold PASS
- `make doc` — PASS
