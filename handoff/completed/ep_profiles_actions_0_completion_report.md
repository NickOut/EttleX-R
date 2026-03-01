# Completion Report: ettle:profiles_actions (ep:profiles_actions:0)

## Summary

**EP**: `ep:profiles_actions:0`
**Ettle**: `ettle:profiles_actions`
**Completed**: 2026-02-27
**Acceptance gates**: lint ✅ test ✅ coverage ✅ (80.28% ≥ 80%)

---

## Scope

Profile authoring commands (`ProfileCreate`, `ProfileSetDefault`) implemented at the engine layer with CAS + SQLite persistence. Durable `ApprovalRequest` model with dual-digest scheme (`approval_token` / `semantic_request_digest`) and `ApprovalRouter` trait for pluggable routing backends.

---

## New Files Created

| File | Purpose |
|---|---|
| `crates/ettlex-core/src/model/approval.rs` | `ApprovalRequest`, `ApprovalKind`, `ApprovalStatus` domain types |
| `crates/ettlex-core/src/approval_router.rs` | `ApprovalRouter` trait (pure, no rusqlite) |
| `crates/ettlex-engine/src/approval/mod.rs` | `NullApprovalRouter` implementation |
| `crates/ettlex-engine/src/commands/profile.rs` | `apply_profile_create`, `apply_profile_set_default` |
| `crates/ettlex-store/migrations/006_approval_requests_schema.sql` | `approval_requests` table |

---

## Files Modified

| File | Change |
|---|---|
| `crates/ettlex-core/src/errors.rs` | Added 5 approval `ExErrorKind` variants + `code()` arms |
| `crates/ettlex-core/src/model/mod.rs` | Added `pub mod approval;` |
| `crates/ettlex-store/src/migrations/embedded.rs` | Registered migration 006 |
| `crates/ettlex-store/src/repo/sqlite_repo.rs` | Added `persist_approval_request`, `get_approval_request`, `list_approval_requests` |
| `crates/ettlex-engine/src/commands/engine_command.rs` | Added `EngineCommand::ProfileCreate`, `EngineCommand::ProfileSetDefault` variants and results |
| `crates/ettlex-engine/src/lib.rs` | Added `pub mod approval;` |

---

## Schema

```sql
CREATE TABLE IF NOT EXISTS approval_requests (
    approval_token          TEXT    PRIMARY KEY NOT NULL,
    semantic_request_digest TEXT    NOT NULL,
    kind                    TEXT    NOT NULL,
    status                  TEXT    NOT NULL DEFAULT 'pending',
    profile_ref             TEXT    NOT NULL,
    policy_ref              TEXT    NOT NULL DEFAULT '',
    leaf_ep_id              TEXT    NOT NULL DEFAULT '',
    reason_code             TEXT    NOT NULL DEFAULT '',
    candidate_set_json      TEXT    NOT NULL DEFAULT '[]',
    created_at              INTEGER NOT NULL,
    resolved_at             INTEGER
) STRICT;

CREATE INDEX IF NOT EXISTS idx_approval_requests_created
    ON approval_requests(created_at, approval_token);
CREATE INDEX IF NOT EXISTS idx_approval_requests_semantic
    ON approval_requests(semantic_request_digest);
```

---

## Domain Model

### ApprovalRequest
- `approval_token`: SHA-256 of canonical JSON **including** `created_at_ms`
- `semantic_request_digest`: SHA-256 of canonical JSON **excluding** `created_at_ms`
- `candidate_set`: lexicographically sorted before digest computation
- Canonical JSON uses `BTreeMap` to guarantee deterministic key ordering
- `status`: `Pending` | `Resolved` | `Rejected`

### ApprovalKind
- `ProfileSelection` | `CandidateSelection`
- `as_str()` / `parse()` for storage round-trip

### ApprovalStatus
- `Pending` | `Resolved` | `Rejected`
- `as_str()` / `parse()` for storage round-trip

---

## ApprovalRouter Trait

```rust
pub trait ApprovalRouter: Send + Sync {
    fn route_approval_request(&self, req: &ApprovalRequest)
        -> std::result::Result<String, ExError>;
}
```

- Lives in `ettlex-core` — no `rusqlite` dependency
- Implementations manage their own connection/state
- `NullApprovalRouter` (in `ettlex-engine`) always returns `ApprovalRoutingUnavailable`

---

## Engine Commands

### ProfileCreate
1. Parse and validate `profile_ref` string
2. Compute payload digest (SHA-256 of canonical BTreeMap JSON)
3. Persist via `persist_profile` (idempotent or `ProfileConflict`)
4. CAS write with `kind: profile_json`
5. Emit provenance event

### ProfileSetDefault
1. Parse and validate `profile_ref` string
2. Verify profile exists in `profiles` table
3. Update `profile_settings.default_profile_ref`
4. Emit provenance event

---

## Error Kinds Added

| ExErrorKind | Stable Code |
|---|---|
| `ApprovalRequestInvalid` | `ERR_APPROVAL_REQUEST_INVALID` |
| `ApprovalNotFound` | `ERR_APPROVAL_NOT_FOUND` |
| `ApprovalStorageCorrupt` | `ERR_APPROVAL_STORAGE_CORRUPT` |
| `ApprovalConflict` | `ERR_APPROVAL_CONFLICT` |
| `ApprovalRoutingUnavailable` | `ERR_APPROVAL_ROUTING_UNAVAILABLE` |

---

## Key Design Decisions

1. **ApprovalRouter trait in ettlex-core**: Keeps `ettlex-core` free of `rusqlite`. Implementations (NullApprovalRouter, future real routers) live in higher layers.
2. **Dual digest scheme**: `semantic_request_digest` enables deduplication across timestamps; `approval_token` is globally unique per request instance.
3. **BTreeMap for canonical JSON**: Prevents non-determinism from HashMap key ordering. The `candidate_set` is also sorted before hashing.
4. **NullApprovalRouter**: Safe default; always returns `ApprovalRoutingUnavailable`. Prevents accidental no-op routing in production.

---

## Test Coverage

Inline tests cover:
- `ApprovalRequest::new` — token computation, semantic digest stability across timestamps
- `candidate_set` sorting
- `ApprovalKind` / `ApprovalStatus` parse/display round-trips
- `persist_approval_request` — insert, idempotent re-insert, conflict
- `get_approval_request` — found, not found
- `list_approval_requests`
- `NullApprovalRouter` — returns `ApprovalRoutingUnavailable`
- `ProfileCreate` / `ProfileSetDefault` engine command dispatch
