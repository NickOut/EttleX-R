# Completion Report: ep:store_ep_update ordinal 7 supplement

**EP:** `ep:store_ep_update:0` — ordinal 7 supplement (12 missing scenarios)
**Date completed:** 2026-03-08
**Classification:** B — Behavioural Extension (adds scenarios to existing EpUpdate)

---

## Summary

Filled 12 missing test scenarios for the EpUpdate triad (store/action/MCP layers). No production
code changes were required; the behaviour was already correct and the scenarios were missing
coverage only.

---

## Scenarios Added

| ID | Scenario | Test Location |
|---|---|---|
| S-SU-null | EpUpdate with all fields null rejected as EmptyUpdate | `ettlex-engine/tests/ep_update_engine_tests.rs` |
| S-SU-large | EpUpdate with large content (50 KB) succeeds | `ettlex-engine/tests/ep_update_engine_tests.rs` |
| S-SU-inv | EpUpdate does not change ordinal/ettle_id/child_ettle_id | `ettlex-engine/tests/ep_update_engine_tests.rs` |
| S-SU-idem | EpUpdate NOT idempotent — two updates increment state_version twice | `ettlex-engine/tests/ep_update_engine_tests.rs` |
| S-SU-det | EpUpdate result deterministic — same input → same stored state | `ettlex-engine/tests/ep_update_engine_tests.rs` |
| S-SU-conc1 | Concurrent EpUpdate with expected_state_version → exactly one success | `ettlex-engine/tests/ep_update_engine_tests.rs` |
| S-SU-conc2 | Sequential EpUpdate without expected_state_version → both succeed (V+2) | `ettlex-engine/tests/ep_update_engine_tests.rs` |
| S-SU-obs | EpUpdate success reflected in state.get_version() | `ettlex-engine/tests/ep_update_engine_tests.rs` |
| S-SU-mig1 | eps schema migration adds title column if absent | `ettlex-store/tests/migrations_test.rs` |
| S-SU-mig2 | EpUpdate on EP created before title column succeeds | `ettlex-engine/tests/ep_update_engine_tests.rs` |
| S-SU-proh | EpUpdate MUST NOT create new EP (list_eps count unchanged) | `ettlex-engine/tests/ep_update_engine_tests.rs` |
| S-SU-byte | ep.get after EpUpdate returns byte-identical results | `ettlex-engine/tests/ep_update_engine_tests.rs` |

---

## Production Changes

None. All scenarios test existing correct behaviour.

---

## Formally Deferred Constraints

None for this supplement.

---

## Acceptance Gates

- `make lint` — PASS
- `make test` — 841/841 PASS, 3 skipped
- `make coverage-check` — 87% ≥ 80% threshold PASS
- `make doc` — PASS (pre-existing warning in ettlex-cli unrelated to this EP)
