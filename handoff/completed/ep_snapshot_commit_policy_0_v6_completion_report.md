# Completion Report: ep:snapshot_commit_policy:0 — v6 dry_run Constraint Resolution Semantics

**Date**: 2026-02-28
**Protocol**: code_generator_prompt_v1_2.md (TDD-First, Triad-Complete, Traceable)
**Plan**: Re-implementation Plan v6 dry_run Constraint Resolution Semantics

---

## Classification

**B — Behavioural Extension**: adds new scenarios to an existing facet (`ep:snapshot_commit_policy:0`).

---

## Preamble: Revert Evidence

Files reverted to HEAD (`69ba82e`) before any new work:

```bash
git restore crates/ettlex-core/src/candidate_resolver.rs
git restore crates/ettlex-engine/src/commands/snapshot.rs
git restore crates/ettlex-engine/tests/snapshot_commit_policy_tests.rs
rm handoff/completed/ep_snapshot_commit_policy_0_v6_completion_report.md
```

Post-revert build: `cargo build -p ettlex-engine` — success.
Post-revert test count: **21 passed, 1 ignored** (22 scenarios total).

---

## RED Evidence (Step 4 Gate)

After writing tests but before any implementation, running:

```
cargo test -p ettlex-engine --test snapshot_commit_policy_tests 2>&1 | head -30
```

Produced compile errors:

```
error[E0432]: unresolved import `ettlex_core::candidate_resolver::DryRunConstraintStatus`
 --> crates/ettlex-engine/tests/snapshot_commit_policy_tests.rs:6:5
  |
6 | use ettlex_core::candidate_resolver::DryRunConstraintStatus;
  |     ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^ no `DryRunConstraintStatus` in `candidate_resolver`

error[E0609]: no field `constraint_resolution` on type `ettlex_engine::commands::snapshot::SnapshotCommitResult`
   --> crates/ettlex-engine/tests/snapshot_commit_policy_tests.rs:742:10
    |
742 |         .constraint_resolution
    |          ^^^^^^^^^^^^^^^^^^^^^ unknown field
    |
    = note: available fields are: `snapshot_id`, `manifest_digest`, `semantic_manifest_digest`, `was_duplicate`, `head_after`

error[E0609]: no field `constraint_resolution` on type `ettlex_engine::commands::snapshot::SnapshotCommitResult`
   --> crates/ettlex-engine/tests/snapshot_commit_policy_tests.rs:793:10
    |
793 |         .constraint_resolution
    |          ^^^^^^^^^^^^^^^^^^^^^ unknown field

error[E0609]: no field `constraint_resolution` on type `ettlex_engine::commands::snapshot::SnapshotCommitResult`
    --> crates/ettlex-engine/tests/snapshot_commit_policy_tests.rs:1056:10
     |
1056 |         .constraint_resolution
     |          ^^^^^^^^^^^^^^^^^^^^^ unknown field
```

RED confirmed. Implementation did NOT proceed until this failure was observed.

---

## GREEN Evidence (Step 6)

After implementing all three files:

```
running 24 tests
test result: ok. 23 passed; 0 failed; 1 ignored; 0 measured; 0 filtered out; finished in 0.16s
```

23 passing + 1 ignored (Phase 1 DeterminismViolation guard — unchanged).

---

## Scenario → Artefact Mapping

| Scenario | Test name | File | Status |
|----------|-----------|------|--------|
| Δ17 | `test_dry_run_no_writes` (extended) | `snapshot_commit_policy_tests.rs` | PASS |
| Δ18 / S24 | `test_dry_run_no_routing` (extended) | `snapshot_commit_policy_tests.rs` | PASS |
| S23 | `test_dry_run_computes_resolved_constraint_resolution` | `snapshot_commit_policy_tests.rs` | PASS |
| S25 | `test_dry_run_yields_uncomputed_when_predicate_evaluation_disabled` | `snapshot_commit_policy_tests.rs` | PASS |

---

## Files Changed

| File | Change |
|------|--------|
| `crates/ettlex-core/src/candidate_resolver.rs` | Added `DryRunConstraintStatus`, `DryRunConstraintResolution`, `compute_dry_run_resolution()` |
| `crates/ettlex-engine/src/commands/snapshot.rs` | Added `ResolvedProfile`, replaced `resolve_ambiguity_policy` with `resolve_profile`, added `constraint_resolution` field to `SnapshotCommitResult`, restructured dry_run steps 6-7 |
| `crates/ettlex-engine/tests/snapshot_commit_policy_tests.rs` | Added `DryRunConstraintStatus` import, `seed_profile_with_disabled_evaluation` helper, extended tests Δ17 & Δ18, added S23 and S25 |
| `crates/ettlex-engine/README.md` | Added `dry_run mode` section documenting `constraint_resolution` semantics and `Uncomputed` status |

---

## Invariants Verified

- `constraint_resolution` is `Some(...)` in all dry_run `Committed` results ✓
- `constraint_resolution` is `None` in all non-dry-run `Committed` results ✓
- `approval_token` never appears in dry_run results ✓
- No snapshot rows written in dry_run (`snapshot_count = 0`) ✓
- No approval_request rows written in dry_run (`approval_count = 0`) ✓

---

## Acceptance Gate Outputs

### `make lint` — zero warnings

```
✓ Banned pattern checks passed
cargo fmt --all -- --check
cargo clippy --workspace -- -D warnings
    Finished `dev` profile [unoptimized + debuginfo] target(s) in 2.80s
```

### `make test` — full workspace clean

All suites passed; zero failures across entire workspace.
`snapshot_commit_policy_tests`: 23 passed, 1 ignored.

### `make coverage-check` — ≥80% threshold met

```
80.43% coverage, 2795/3475 lines covered
✅ Coverage 80% meets minimum threshold 80%
```

### `make coverage-html` — HTML report generated

```
✅ HTML coverage report generated: coverage/tarpaulin-report.html
```

---

## TDD Protocol Confirmation

- Reverted pre-existing out-of-order implementation before any new work ✓
- RED gate confirmed (compile failure observed before touching implementation) ✓
- GREEN gate confirmed (23 passing tests after implementation) ✓
- No new behavioural code written without a driving test ✓
- Docs triad complete: rustdocs on new types + README.md updated ✓
- Repo placement rules respected (core types in `ettlex-core`, engine logic in `ettlex-engine`) ✓
