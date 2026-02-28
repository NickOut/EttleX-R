# Completion Report: ep:snapshot_commit_actions_refactor:0

**Date:** 2026-02-28
**Status:** DONE
**Coverage after:** 80.48% (threshold: 80%)

---

## Summary

Refactored snapshot commit to ensure `apply_engine_command` is the ONLY supported production ingress for `SnapshotCommit`. Prevents split-brain between CLI/engine and MCP/actions paths. Added structured error candidates, `RootEttleInvalid` error kind, `allow_dedup` flag (append-only default), and re-wired CLI `--root` through the canonical path.

---

## Traceability Table

| Scenario | Test File | Test Name | Production Module(s) |
|----------|-----------|-----------|----------------------|
| S1: Commit via apply_engine_command (leaf) | `snapshot_commit_by_leaf_tests.rs` | `test_snapshot_commit_succeeds_via_action_command` | `engine_command.rs`, `snapshot.rs` |
| S2: CLI `--root` delegates via apply_engine_command | `cli_snapshot_integration_tests.rs` | `test_cli_snapshot_commit_with_root_flag_legacy` | `ettlex-cli/commands/snapshot.rs` |
| S3: Store/engine fns internal-only | `snapshot_commit_legacy_resolution_tests.rs` | structural (pub(crate)) | `engine/commands/snapshot.rs` |
| S4: NotALeaf on non-leaf EP | `snapshot_commit_by_leaf_tests.rs` | `test_snapshot_commit_rejects_non_leaf_ep` | `snapshot.rs` |
| S5: NotFound on unknown EP | `snapshot_commit_by_leaf_tests.rs` | `test_snapshot_commit_rejects_unknown_ep` | `snapshot.rs` |
| S6: Legacy root resolves to one leaf | `snapshot_commit_legacy_resolution_tests.rs` | `test_legacy_root_resolves_when_exactly_one_leaf` | `snapshot.rs::resolve_root_to_leaf_ep` |
| S7: RootEttleAmbiguous includes candidate ids | `snapshot_commit_legacy_resolution_tests.rs` | `test_legacy_root_fails_when_multiple_leaves` | `errors.rs`, `snapshot.rs` |
| S8: No leaves → RootEttleInvalid | `snapshot_commit_legacy_resolution_tests.rs` | `test_legacy_root_fails_when_no_leaves` | `errors.rs`, `snapshot.rs` |
| S9: Determinism across paths | `snapshot_commit_determinism_tests.rs` | `test_snapshot_output_deterministic_across_paths` | `snapshot.rs` |
| S10: created_at non-determinism preserved | `snapshot_commit_determinism_tests.rs` | `test_created_at_non_determinism_preserved` | `snapshot.rs`, `persist.rs` |
| S11: No extra mutation during commit | `snapshot_commit_tests.rs` | `test_no_extra_mutations_during_commit` | `snapshot.rs` |
| S12: Policy gating in action layer | `snapshot_commit_policy_tests.rs` | `test_policy_denied_no_writes_no_routing` | `engine_command.rs` |
| S13: CAS IO error → Persistence typed error | `snapshot_commit_policy_tests.rs` | `test_snapshot_commit_cas_failure_surfaces_persistence_error` | `persist.rs` |
| S14: EptAmbiguous unreachable | `snapshot_commit_policy_tests.rs` | `test_ept_ambiguous_not_waivable` (`#[ignore]`) | `snapshot.rs` |
| S15a: Append-only default (allow_dedup=false) | `snapshot_commit_idempotency_tests.rs` | `test_snapshot_commit_append_only_default` | `persist.rs`, `snapshot.rs` |
| S15b: allow_dedup=true returns existing | `snapshot_commit_idempotency_tests.rs` | `test_snapshot_commit_allow_dedup_returns_existing` | `persist.rs` |
| S15c: allow_dedup records reuse event | `snapshot_commit_idempotency_tests.rs` | `test_snapshot_commit_allow_dedup_records_reuse_event` | `persist.rs` |
| S16: Large manifest performance | `snapshot_commit_idempotency_tests.rs` | `test_snapshot_commit_large_manifest` | `snapshot.rs`, `persist.rs` |

---

## Files Modified

| File | Change |
|------|--------|
| `crates/ettlex-core/src/errors.rs` | Added `RootEttleInvalid` kind + `ERR_ROOT_ETTLE_INVALID` code; added `candidates: Option<Vec<String>>` field to `ExError`; added `with_candidates()` builder and `candidates()` accessor |
| `crates/ettlex-store/src/snapshot/persist.rs` | Added `allow_dedup: bool` to `SnapshotOptions`; gated dedup block on `allow_dedup`; emit `tracing::info!` reuse event when `allow_dedup=true` and duplicate found |
| `crates/ettlex-engine/src/commands/snapshot.rs` | Added `allow_dedup: bool` to `SnapshotOptions`; changed `snapshot_commit_by_leaf` and `snapshot_commit_by_root_legacy` to `pub(crate)`; fixed 0-leaf case to `RootEttleInvalid`; added `.with_candidates()` on multi-leaf error; added `pub fn resolve_root_to_leaf_ep` wrapper; added module-level visibility doc comment |
| `crates/ettlex-cli/src/commands/snapshot.rs` | Removed `snapshot_commit_by_root_legacy` import; added `resolve_root_to_leaf_ep` import; re-wired `--root` branch through `resolve_root_to_leaf_ep` + `apply_engine_command` |

## Test Files Modified/Created

| File | Change |
|------|--------|
| `crates/ettlex-engine/tests/snapshot_commit_idempotency_tests.rs` | NEW — S15 (append-only + allow_dedup) + S16 (large manifest) |
| `crates/ettlex-engine/tests/snapshot_commit_legacy_resolution_tests.rs` | Rewrote to use `resolve_root_to_leaf_ep` (pub(crate) broke old imports); S8 asserts `RootEttleInvalid`; S7 asserts `candidates()` contains both EP ids |
| `crates/ettlex-engine/tests/snapshot_commit_determinism_tests.rs` | S10: two append-only commits → 2 rows, different snapshot_ids, same semantic digest |
| `crates/ettlex-engine/tests/snapshot_commit_tests.rs` | Added `allow_dedup: false` to all `SnapshotOptions` literals; `test_snapshot_commit_idempotent_across_calls` uses `allow_dedup: true` |
| `crates/ettlex-engine/tests/snapshot_commit_policy_tests.rs` | Added `allow_dedup: false` to all `SnapshotOptions` literals; Scenario 19 second call uses `allow_dedup: true`; added S13 IO error test |
| `crates/ettlex-engine/tests/snapshot_commit_by_leaf_tests.rs` | Added `allow_dedup: false` to all `SnapshotOptions` literals |
| `crates/ettlex-store/tests/snapshot_persist_tests.rs` | Added `allow_dedup: false/true` to all `SnapshotOptions` literals as appropriate |

---

## Acceptance Gates

- [x] `make lint` — no warnings/errors
- [x] `make test` — all tests pass (including `test_snapshot_commit_cas_failure_surfaces_persistence_error`)
- [x] `make coverage-check` — 80.48% ≥ 80% threshold

---

## Key Behavioural Changes

1. **Append-only default**: `allow_dedup: false` is now the default. Two identical commits create two rows. Callers opt into dedup with `allow_dedup: true`.
2. **`RootEttleInvalid`** replaces `NotFound` when a root ettle has no leaf EPs.
3. **Structured candidates**: `ExError::candidates()` returns `Option<&[String]>` containing candidate leaf EP ids for `RootEttleAmbiguous` errors.
4. **CLI `--root` canonical path**: CLI resolves root→leaf via `resolve_root_to_leaf_ep`, then dispatches through `apply_engine_command` — identical to the `--leaf` path.
5. **Visibility enforcement**: `snapshot_commit_by_leaf` and `snapshot_commit_by_root_legacy` are `pub(crate)` — not accessible from outside `ettlex-engine`.
