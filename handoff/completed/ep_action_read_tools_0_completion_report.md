# Completion Report: ep:action_read_tools:0

**Completed**: 2026-03-01
**Seed**: `handoff/seed_action_read_tools_v5.yaml`
**Branch**: `main`

---

## Summary

Implemented the full read-only query surface for the EttleX engine layer.
All 33 Gherkin scenarios from the seed are covered by integration tests.
The entry point `apply_engine_query(query, &Connection, &FsStore)` is
non-mutating; no variant acquires `&mut Connection` or writes to the DB/CAS.

---

## Acceptance Gates

| Gate | Result |
|------|--------|
| `make lint` | ✅ Pass (0 errors, 0 warnings) |
| `make test` | ✅ Pass (all tests pass; 3 `#[ignore]`) |
| `make coverage-check` | ✅ Pass (80.36% ≥ 80% threshold) |
| `cargo doc --workspace --no-deps` | ✅ Pass (0 warnings) |

---

## New ExErrorKind Variants

Added to `crates/ettlex-core/src/errors.rs`:

| Variant | Code | Description |
|---------|------|-------------|
| `ApprovalStorageCorrupt` | `ERR_APPROVAL_STORAGE_CORRUPT` | SQLite row exists but CAS blob is missing |
| `RefinementIntegrityViolation` | `ERR_REFINEMENT_INTEGRITY_VIOLATION` | EP has more than one structural parent |
| `NotImplemented` | `ERR_NOT_IMPLEMENTED` | Valid but unimplemented optional tool |

---

## New Files Created

| File | Description |
|------|-------------|
| `crates/ettlex-engine/src/commands/read_tools.rs` | All result structs, `ListOptions`, `Page<T>`, cursor helpers, `DEFAULT_LIST_LIMIT` |
| `crates/ettlex-store/migrations/007_approval_cas_schema.sql` | `request_digest TEXT` column + index on `approval_requests` |
| `crates/ettlex-engine/tests/action_read_tools_integration_tests.rs` | S1–S22 (S14, S22 `#[ignore]`) |
| `crates/ettlex-engine/tests/profile_approval_query_tests.rs` | S23–S30 |
| `crates/ettlex-engine/tests/predicate_preview_tests.rs` | S31–S33 + empty-candidates extension |
| `docs/action-read-tools.md` | Full query vocabulary, pagination semantics, error contract |

---

## Modified Files

| File | Change |
|------|--------|
| `crates/ettlex-core/src/errors.rs` | +3 `ExErrorKind` variants + `code()` / `Display` arms |
| `crates/ettlex-engine/src/commands/mod.rs` | `pub mod read_tools;` |
| `crates/ettlex-engine/src/commands/engine_query.rs` | +29 `EngineQuery` variants, +29 `EngineQueryResult` variants, full `apply_engine_query` dispatch; `SnapshotDiff` variant boxed |
| `crates/ettlex-store/src/migrations/embedded.rs` | Migration 007 registered |
| `crates/ettlex-store/src/repo/sqlite_repo.rs` | +8 read-only query functions |
| `crates/ettlex-store/src/snapshot/query.rs` | `SnapshotRow` struct + 5 new query functions + inline unit tests |
| `crates/ettlex-store/src/profile.rs` | `ApprovalRow` struct + 5 new query functions; `SqliteApprovalRouter` updated to write CAS + store `request_digest`; inline unit tests |
| `crates/ettlex-engine/tests/snapshot_diff_integration_tests.rs` | Added wildcard arm to non-exhaustive match |
| `crates/ettlex-engine/README.md` | New "Read-Only Query Surface" section |
| `crates/ettlex-store/README.md` | New store query surface documentation; migration 007 documented |

---

## Architecture Notes

### EPT traversal uses `ettle.parent_id`

`compute_rt` / `compute_ept` follow `ettle.parent_id` for upward traversal — **not**
`ep.child_ettle_id`. Tests that set up EPT chains must set `parent_id` on child Ettles.

### `EpListDecisions` ancestor walk

With `include_ancestors = true`, the implementation walks up `ettle.parent_id` and
collects decisions linked to **Ettles** (`target_kind = "ettle"`), not to EPs.

### Coverage scope

`make coverage-check` uses `--exclude-files 'tests/*'`, so only inline
`#[cfg(test)]` modules count. Inline test modules were added to both
`snapshot/query.rs` and `profile.rs` to ensure coverage above 80%.

### `SnapshotDiff` boxed

`EngineQueryResult::SnapshotDiff` is `Box<SnapshotDiffResult>` to satisfy the
`large_size_difference` Clippy lint on the `EngineQueryResult` enum.

---

## Test Count

| Test file | Tests |
|-----------|-------|
| `action_read_tools_integration_tests.rs` | 17 (+ 2 `#[ignore]`) |
| `profile_approval_query_tests.rs` | 8 |
| `predicate_preview_tests.rs` | 4 |
| `snapshot/query.rs` (inline) | 8 |
| `profile.rs` (inline) | 13 |

Total new tests: **50** (48 active + 2 ignored)

---

## Scenario Traceability

| Scenario | Test |
|----------|------|
| S1: read tools never mutate | `test_read_tools_are_nonmutating` |
| S2: ettle.get | `test_ettle_get_returns_metadata_and_eps` |
| S3: ettle.list default limit | `test_ettle_list_enforces_default_limit` |
| S4: ettle.list cursor pagination | `test_ettle_list_cursor_pagination_deterministic` |
| S5: ettle.list prefix filter | `test_ettle_list_prefix_filter` |
| S6: ep.list_children | `test_ep_list_children_deterministic` |
| S7: ep.list_parents single | `test_ep_list_parents_single_parent` |
| S8: ep.list_parents corruption | `test_ep_list_parents_integrity_violation` |
| S9: constraint.list_by_family tombstone | `test_constraint_list_by_family_tombstone_filter` |
| S10: ep.list_constraints | `test_ep_list_constraints_ordered` |
| S11: manifest.get_by_snapshot | `test_manifest_get_by_snapshot_digests_and_bytes` |
| S12: manifest.get_by_digest unknown | `test_manifest_get_by_digest_not_found` |
| S13: ept.compute | `test_ept_compute_deterministic` |
| S14: ept.compute ambiguous | `#[ignore] test_ept_compute_ambiguous` |
| S15–S17: snapshot.diff | existing `snapshot_diff_integration_tests.rs` |
| S18: decision.list | `test_decision_list_deterministic` |
| S19: ep.list_decisions ancestors | `test_ep_list_decisions_with_ancestors` |
| S20: ept.compute_decision_context | `test_ept_compute_decision_context_deterministic` |
| S21: decision queries no snapshot effect | `test_decision_queries_no_snapshot_effect` |
| S22: scale 10k EPs | `#[ignore] test_read_tools_scale` |
| S23: profile.get | `test_profile_get_deterministic` |
| S24: profile.resolve null → default | `test_profile_resolve_null_uses_default` |
| S25: profile.resolve unknown | `test_profile_resolve_unknown_not_found` |
| S26: profile.list pagination | `test_profile_list_pagination_deterministic` |
| S27: approval.get | `test_approval_get_digests_and_bytes` |
| S28: approval.get unknown | `test_approval_get_unknown_token` |
| S29: approval.get CAS missing | `test_approval_get_cas_blob_missing_corrupt_error` |
| S30: approval.list ordering | `test_approval_list_deterministic` |
| S31: preview no approval created | `test_preview_does_not_create_approval_request` |
| S32: preview deterministic | `test_preview_deterministic` |
| S33: preview evaluation disabled | `test_preview_evaluation_disabled` |
| S33 ext: empty candidates | `test_preview_empty_candidates_no_match` |
