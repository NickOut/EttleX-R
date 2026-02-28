# Completion Report: EP `content_digest` Persistence (seed_store_v6 delta)

**Date:** 2026-02-28
**Classification:** B (Behavioural Extension)
**Status:** DONE

---

## Summary

Implemented `content_digest` field on `Ep` model and wired it into SQLite persistence.
The `eps.content_digest` column was already present in migration 001 but was always `NULL`; now it is populated with a SHA-256 hex digest computed from canonical WHY+WHAT+HOW JSON.

---

## Changes

### `crates/ettlex-core/src/model/ep.rs`

- Added `use sha2::{Digest, Sha256};`
- Added `pub content_digest: String` field after `how`, before `created_at`
- Added private `fn compute_content_digest(why, what, how) -> String`:
  - Builds `BTreeMap<&str, &str>` with keys `"how"`, `"what"`, `"why"` (alphabetical)
  - Serialises via `serde_json::to_string` → canonical JSON `{"how":"…","what":"…","why":"…"}`
  - SHA-256 hex encodes the result
- `Ep::new()`: assigns `content_digest` from `compute_content_digest`
- **`Ep::new()` signature unchanged** — all call sites compile without modification
- Added 3 inline unit tests:
  - `test_ep_content_digest_is_64_chars`
  - `test_ep_content_digest_is_deterministic`
  - `test_ep_content_digest_changes_with_content`

### `crates/ettlex-store/src/repo/sqlite_repo.rs`

- `persist_ep` (line ~98): `None::<String>` → `Some(ep.content_digest.clone())`
- `persist_ep_tx` (line ~133): `None::<String>` → `Some(ep.content_digest.clone())`

### `crates/ettlex-store/tests/ep_content_digest_tests.rs` (new)

4 integration tests written BEFORE production code (TDD RED gate confirmed):
- `test_seed_import_content_digest_non_null` — digest is non-null 64-char hex
- `test_seed_import_content_digest_correct_value` — stored digest matches expected SHA-256
- `test_seed_import_persists_why_and_what_bodies` — WHY/WHAT present, digest non-null (regression guard)
- `test_seed_import_content_digest_stable_on_reload` — stored digest equals `ep.content_digest` after hydration

---

## No Other Changes

- No migrations (column already exists in migration 001)
- No hydration changes (hydration calls `Ep::new()` with loaded values; digest recomputed automatically)
- No call-site changes to `Ep::new()`

---

## Acceptance Gates

| Gate | Result |
|------|--------|
| `make lint` | ✅ PASS |
| `make test` | ✅ PASS |
| `make coverage-check` | ✅ 81.07% ≥ 80% |
