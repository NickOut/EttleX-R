# Completion Report: ep:constraint_engine_slice0:0

**Date:** 2026-02-28
**Status:** DONE
**Coverage:** 81.00% (threshold: 80%)

---

## Summary

Implemented the stable constraint engine boundary for EttleX Phase 1. All 16 Gherkin scenarios pass. The formal engine boundary module is present, error kinds are correct, manifest format uses plain constraint IDs, and all families report `UNCOMPUTED` status in Phase 1.

---

## Changes Made

### New Files

| File | Description |
|---|---|
| `crates/ettlex-core/src/constraint_engine/mod.rs` | **NEW** — constraint engine boundary: `evaluate()`, `ConstraintEvalCtx`, `DeclaredConstraintRef`, `ConstraintFamilyStatus`, `FamilyEvaluation`, `ConstraintEvaluation` |
| `crates/ettlex-core/tests/constraint_engine_slice0_tests.rs` | **NEW** — S1–S5, S8, S12, S13 + Phase 1 error kind + Phase 3 engine tests (17 tests) |
| `crates/ettlex-engine/tests/constraint_engine_slice0_tests.rs` | **NEW** — S6, S7, S9–S11, S14, S15 (7 tests) |

### Modified Files

| File | Change |
|---|---|
| `crates/ettlex-core/src/errors.rs` | Added 4 `ExErrorKind` variants + codes; 3 new `EttleXError` variants; updated bridge: `ConstraintAlreadyAttached→DuplicateAttachment`, new `ConstraintAlreadyExists→AlreadyExists`, new `ConstraintTombstoned→ConstraintTombstoned`, new `InvalidConstraintFamily→InvalidConstraintFamily` |
| `crates/ettlex-core/src/ops/constraint_ops.rs` | `create_constraint` validates empty family (`InvalidConstraintFamily`) and duplicate ID (`ConstraintAlreadyExists`); `attach_constraint_to_ep` uses `ConstraintTombstoned` instead of `ConstraintDeleted`; updated inline test |
| `crates/ettlex-core/src/ops/store.rs` | Added `get_constraint_including_deleted()` method |
| `crates/ettlex-core/src/lib.rs` | Exposed `pub mod constraint_engine` |
| `crates/ettlex-core/src/snapshot/manifest.rs` | `FamilyConstraints` gained `status: ConstraintFamilyStatus` field; `declared_refs` and `active_refs` now use plain constraint IDs; `ConstraintsEnvelope::from_ept()` routes through `constraint_engine::evaluate()` |
| `crates/ettlex-engine/tests/constraint_manifest_integration_tests.rs` | Updated `declared_refs` assertions to plain IDs; added `status` assertions; updated scenario 7 to ordinal-based ordering |
| `crates/ettlex-core/src/apply.rs` | Updated `test_apply_constraint_attach_deleted_constraint` to expect `ConstraintTombstoned` |

---

## Error Kinds Added

| ExErrorKind | Code | EttleXError bridge source |
|---|---|---|
| `InvalidConstraintFamily` | `ERR_INVALID_CONSTRAINT_FAMILY` | `EttleXError::InvalidConstraintFamily` |
| `AlreadyExists` | `ERR_ALREADY_EXISTS` | `EttleXError::ConstraintAlreadyExists` |
| `ConstraintTombstoned` | `ERR_CONSTRAINT_TOMBSTONED` | `EttleXError::ConstraintTombstoned` |
| `DuplicateAttachment` | `ERR_DUPLICATE_ATTACHMENT` | `EttleXError::ConstraintAlreadyAttached` (remapped from ConstraintViolation) |

---

## Constraint Engine Boundary

### Public API (`ettlex_core::constraint_engine`)

```rust
pub struct ConstraintEvalCtx {
    pub leaf_ep_id: String,
    pub ept_ep_ids: Vec<String>,
    pub policy_ref: String,
    pub profile_ref: String,
}

pub struct DeclaredConstraintRef {
    pub constraint_id: String,
    pub family: String,
    pub payload_digest: String,
}

pub enum ConstraintFamilyStatus {
    #[serde(rename = "UNCOMPUTED")]
    Uncomputed,
}

pub struct FamilyEvaluation {
    pub status: ConstraintFamilyStatus,
    pub digest: String,
    pub opaque_section: Option<serde_json::Value>,
}

pub struct ConstraintEvaluation {
    pub declared_refs: Vec<DeclaredConstraintRef>,
    pub families: BTreeMap<String, FamilyEvaluation>,
    pub constraints_digest: String,
}

pub fn evaluate(ctx: &ConstraintEvalCtx, store: &Store) -> Result<ConstraintEvaluation, ExError>
```

### Ordering Rules

`declared_refs` are ordered by `(ordinal, constraint_id)`. First EP in EPT wins for deduplication. Tombstoned constraints are excluded.

### UNCOMPUTED Semantics

In Phase 1, all families report `status: Uncomputed`. No validation is performed. The manifest records which constraints are declared for audit purposes only.

---

## Manifest Changes

- `FamilyConstraints.status: ConstraintFamilyStatus` field added (serialized as `"UNCOMPUTED"`)
- `declared_refs`: now plain constraint IDs, ordered by attachment ordinal (was `"family:kind:id"` sorted alphabetically)
- `active_refs` inside each family: also plain constraint IDs
- `ConstraintsEnvelope::from_ept()` now delegates to `constraint_engine::evaluate()`

---

## Test Coverage

| Scenario | Test file | Status |
|---|---|---|
| S1: Create unknown family | `ettlex-core/tests/constraint_engine_slice0_tests.rs` | ✅ |
| S2: Create rejects empty family | same | ✅ |
| S3: Create rejects duplicate id | same | ✅ |
| S4: Update changes payload_digest | same | ✅ |
| S5: Tombstone prevents attach, reads preserved | same | ✅ |
| S6: Attach → appears in manifest declared_refs | `ettlex-engine/tests/constraint_engine_slice0_tests.rs` | ✅ |
| S7: Attach to EP not in EPT → no manifest entry | same | ✅ |
| S8: Duplicate attachment rejected | `ettlex-core/tests/constraint_engine_slice0_tests.rs` | ✅ |
| S9: Detach removes from declared_refs | `ettlex-engine/tests/constraint_engine_slice0_tests.rs` | ✅ |
| S10: declared_refs ordering is deterministic | same | ✅ |
| S11: constraints_digest changes iff set changes | same | ✅ |
| S12: Attach rejects unknown constraint id | `ettlex-core/tests/constraint_engine_slice0_tests.rs` | ✅ |
| S13: Attach rejects unknown ep id | same | ✅ |
| S14: Evaluate returns UNCOMPUTED for ABB/SBB | `ettlex-engine/tests/constraint_engine_slice0_tests.rs` | ✅ |
| S15: 500 constraints complete + deterministic | same | ✅ |
| Phase 1 error kinds | `ettlex-core/tests/constraint_engine_slice0_tests.rs` | ✅ (4 tests) |
| Engine evaluate | same | ✅ (4 tests) |

**Total new tests: 24** (17 in ettlex-core, 7 in ettlex-engine)

---

## Acceptance Gates

```
make lint          ✅  0 errors, 0 warnings
make test          ✅  0 failures (1 ignored: EptAmbiguous guard)
make coverage-check ✅  81.00% ≥ 80% threshold (+0.52% change)
```
