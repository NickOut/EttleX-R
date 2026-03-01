# Completion Report: ep:snapshot_diff:0 — Snapshot Diff Engine

**Date**: 2026-03-01
**Status**: DONE

---

## Summary

Implemented the full snapshot diff engine per `handoff/seed_snapshot_diff_v10.yaml`.

---

## New Files

| File | Purpose |
|---|---|
| `crates/ettlex-core/src/diff/mod.rs` | Module root with re-exports and module docs |
| `crates/ettlex-core/src/diff/model.rs` | All diff output types (`SnapshotDiff`, `DiffSeverity`, etc.) |
| `crates/ettlex-core/src/diff/engine.rs` | `compute_diff(a_bytes, b_bytes) -> Result<SnapshotDiff>` |
| `crates/ettlex-core/src/diff/human_summary.rs` | `render_human_summary(diff) -> String` |
| `crates/ettlex-store/src/snapshot/query.rs` | `fetch_snapshot_manifest_digest`, `fetch_manifest_bytes_by_digest` |
| `crates/ettlex-engine/src/commands/engine_query.rs` | `apply_engine_query`, `SnapshotRef`, `EngineQuery`, `EngineQueryResult` |
| `crates/ettlex-core/tests/snapshot_diff_tests.rs` | 26 pure diff unit tests |
| `crates/ettlex-engine/tests/snapshot_diff_integration_tests.rs` | 5 integration tests |

## Modified Files

| File | Change |
|---|---|
| `crates/ettlex-core/src/errors.rs` | +4 ExErrorKind variants: `InvalidManifest`, `MissingField`, `MissingBlob`, `InvariantViolation` |
| `crates/ettlex-core/src/lib.rs` | `pub mod diff;` declaration |
| `crates/ettlex-store/src/snapshot/mod.rs` | `pub mod query;` + re-export |
| `crates/ettlex-engine/src/commands/mod.rs` | `pub mod engine_query;` |

---

## New ExErrorKind Variants

| Variant | Code |
|---|---|
| `InvalidManifest` | `ERR_INVALID_MANIFEST` |
| `MissingField` | `ERR_MISSING_FIELD` |
| `MissingBlob` | `ERR_MISSING_BLOB` |
| `InvariantViolation` | `ERR_INVARIANT_VIOLATION` |

---

## Key Design Notes

- `compute_diff` is pure (no I/O). Takes raw `&[u8]` manifest bytes.
- Fast-path: byte-identical → `Identical` / `DiffSeverity::None` (no detailed computation)
- Semantic-identity path: same `semantic_manifest_digest` → `NoSemanticChange`
- Invariant violations (constraints_digest mismatch) are **non-fatal** — diff still returns
- All collections use `BTreeMap`/sorted `Vec` for deterministic serialisation
- Determinism guard: output round-trips through JSON without mutation
- `apply_engine_query` accepts `&Connection` (read-only, no `&mut`)
- CAS key vs manifest-internal `manifest_digest`: the DB stores the CAS key (SHA256 of pretty-printed JSON); the manifest's own `manifest_digest` field is SHA256 of compact JSON (different values — do not compare directly)

---

## Acceptance Gates

```
make lint          # ✅ no warnings, no errors
make test          # ✅ all tests pass (31 new + all existing)
make coverage-check # ✅ 82.17% ≥ 80% threshold
```

---

## Test Count

- Pure diff tests (`snapshot_diff_tests.rs`): **26**
- Integration tests (`snapshot_diff_integration_tests.rs`): **5**
- `human_summary.rs` inline tests: **13**
- **Total new tests: 44**
