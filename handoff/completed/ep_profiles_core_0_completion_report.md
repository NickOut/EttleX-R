# Completion Report: ettle:profiles_core (ep:profiles_core:0)

## Summary

**EP**: `ep:profiles_core:0`
**Ettle**: `ettle:profiles_core`
**Completed**: 2026-02-27
**Acceptance gates**: lint ✅ test ✅ coverage ✅ (80.28% ≥ 80%)

---

## Scope

Immutable versioned profile persistence with content-addressable storage, canonical JSON serialization, CAS-based conflict detection, and deterministic reference resolution.

---

## New Files Created

| File | Purpose |
|---|---|
| `crates/ettlex-core/src/model/profile.rs` | `ProfileRef`, `ProfilePayload`, `Profile` domain types |
| `crates/ettlex-core/src/queries/profile_queries.rs` | `profile_get`, `profile_list`, `profile_resolve_ref`, `profile_get_default` |
| `crates/ettlex-store/migrations/005_profiles_schema.sql` | `profiles` + `profile_settings` tables |

---

## Files Modified

| File | Change |
|---|---|
| `crates/ettlex-core/src/errors.rs` | Added 8 new `ExErrorKind` variants + `code()` arms |
| `crates/ettlex-core/src/model/mod.rs` | Added `pub mod profile;` |
| `crates/ettlex-core/src/lib.rs` | Added `pub mod predicate;` (shared with constraint_predicates) |
| `crates/ettlex-core/src/queries/mod.rs` | Added `pub mod profile_queries;` |
| `crates/ettlex-store/src/migrations/embedded.rs` | Registered migration 005 |
| `crates/ettlex-store/src/repo/sqlite_repo.rs` | Added `persist_profile`, `get_profile`, `list_profiles`, `get_default_profile_ref`, `set_default_profile_ref` |

---

## Schema

```sql
CREATE TABLE IF NOT EXISTS profiles (
    profile_id   TEXT    NOT NULL,
    version      INTEGER NOT NULL,
    payload_digest TEXT  NOT NULL,
    payload_json TEXT    NOT NULL,
    created_at   INTEGER NOT NULL,
    PRIMARY KEY (profile_id, version)
) STRICT;

CREATE TABLE IF NOT EXISTS profile_settings (
    singleton    INTEGER PRIMARY KEY CHECK (singleton = 1) DEFAULT 1,
    default_profile_ref TEXT
) STRICT;
INSERT OR IGNORE INTO profile_settings (singleton) VALUES (1);

CREATE INDEX IF NOT EXISTS idx_profiles_digest ON profiles(payload_digest);
```

---

## Domain Model

### ProfileRef
- Parses `profile/<id>@<version>` format
- Validates: id non-empty alphanumeric/hyphen, version ≥ 0
- `parse()` → `Result<ProfileRef, ExError>` (`ProfileRefInvalid` on failure)

### ProfilePayload
- `BTreeMap<String, serde_json::Value>` — guarantees deterministic key ordering for SHA-256 digest
- `digest()` → `String` (hex SHA-256 of canonical JSON)

### Profile
- Composite of `profile_id: String`, `version: u32`, `payload_digest: String`, `payload_json: String`, `created_at_ms: i64`
- `profile_ref()` → `ProfileRef` reconstruction
- Implements `Display`: `Profile(profile/<id>@<ver>, digest=<first8>...)`

---

## Query Functions

| Function | Behaviour |
|---|---|
| `profile_get(conn, profile_ref)` | Fetch specific version; `ProfileNotFound` if absent |
| `profile_list(conn)` | All profiles ordered by (profile_id, version) |
| `profile_resolve_ref(conn, ref_str, config_default)` | Explicit ref → config default → `profile/default@0` → `ProfileDefaultMissing` |
| `profile_get_default(conn)` | Returns current default ref string; `ProfileDefaultMissing` if none set |

---

## Error Kinds Added

| ExErrorKind | Stable Code |
|---|---|
| `ProfileRefInvalid` | `ERR_PROFILE_REF_INVALID` |
| `ProfileNotFound` | `ERR_PROFILE_NOT_FOUND` |
| `ProfileDefaultMissing` | `ERR_PROFILE_DEFAULT_MISSING` |
| `ProfileConflict` | `ERR_PROFILE_CONFLICT` |
| `ProfileImmutable` | `ERR_PROFILE_IMMUTABLE` |
| `ProfileCorrupt` | `ERR_PROFILE_CORRUPT` |
| `ProfileStorageCorrupt` | `ERR_PROFILE_STORAGE_CORRUPT` |
| `ProfileInvalid` | `ERR_PROFILE_INVALID` |

---

## Immutability Invariant

The `persist_profile` function enforces:
- Same (profile_id, version) + same digest → idempotent (no error)
- Same (profile_id, version) + different digest → `ProfileConflict`

---

## Test Coverage

All scenarios covered by inline unit tests in `model/profile.rs`, `queries/profile_queries.rs`, and `sqlite_repo.rs`. Specific coverage includes:
- `ProfileRef::parse` — valid, invalid id, invalid version, missing separator
- `ProfilePayload` — empty, multi-key determinism (BTreeMap ordering)
- `persist_profile` — insert, idempotent re-insert, conflict detection
- `get_profile` — found, not found
- `list_profiles` — empty, populated
- `get_default_profile_ref` / `set_default_profile_ref`
- `profile_resolve_ref` — explicit ref, config default fallback, `profile/default@0` fallback, missing
