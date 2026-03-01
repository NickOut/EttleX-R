# Completion Report: ep:policy_codegen_handoff:0 + :1

**Date**: 2026-03-01
**Crates modified**: `ettlex-core`, `ettlex-store`, `ettlex-engine`, `ettlex-cli`

---

## Summary

Implements a backend-agnostic `PolicyProvider` indirection layer that supersedes `CommitPolicyHook`, adds `FilePolicyProvider` (Markdown-backed), exposes policy read/export on the engine query surface, enforces `PolicyRefMissing` before any writes, and fires the policy check before the dry_run short-circuit.

---

## TDD Evidence (RED→GREEN per cycle)

### Cycle 1 — Error kinds
- **RED**: Added test `test_policy_error_kind_codes` referencing variants that didn't exist → compile failure.
- **GREEN**: Added 5 new `ExErrorKind` variants + `code()`/`display_name()` arms in `errors.rs`.

### Cycle 2 — `PolicyProvider` trait + built-in impls
- **RED**: Created `policy_provider_tests.rs` importing `ettlex_core::policy_provider` (module didn't exist) → compile failure.
- **GREEN**: Created `crates/ettlex-core/src/policy_provider.rs` with `PolicyProvider`, `PolicyListEntry`, `NoopPolicyProvider`, `DenyAllPolicyProvider`.

### Cycle 3 — Engine signature change
- **RED**: Tests calling `apply_engine_command(..., &DenyAllPolicyProvider, ...)` failed to compile (engine still used `&dyn CommitPolicyHook`).
- **GREEN**: Changed `engine_command.rs` and `snapshot.rs` signatures to `&dyn PolicyProvider`. Added `PolicyRefMissing` guard at step 1a. Updated all callers.

### Cycle 4 — `FilePolicyProvider` + policy file
- **RED**: Tests importing `ettlex_store::file_policy_provider::FilePolicyProvider` failed to compile.
- **GREEN**: Created `crates/ettlex-store/src/file_policy_provider.rs`. Created `policies/codegen_handoff_policy_v1.md` with B1.1–B1.6 obligations.

### Cycle 5 — Engine query variants
- **RED**: Tests referencing `EngineQuery::PolicyList` / `PolicyRead` / `PolicyExport` / `SnapshotManifestPolicyRef` failed to compile.
- **GREEN**: Added 4 new `EngineQuery` variants, 4 new `EngineQueryResult` variants, `PolicyReadResult`, `PolicyExportResult`, and updated `apply_engine_query` signature (4th arg: `Option<&dyn PolicyProvider>`).

### Cycle 6 — `PolicyProviderAnchorAdapter`
- **RED**: Tests importing `PolicyProviderAnchorAdapter` failed to compile.
- **GREEN**: Added `PolicyProviderAnchorAdapter<'a>` to `policy.rs` implementing `AnchorPolicy` with NeverAnchored semantics.

---

## 15 Scenario → Test Mapping

| Scenario | Test | File |
|----------|------|------|
| S1 Commit denied by policy | `test_s1_commit_denied_by_policy` | `policy_provider_tests.rs` |
| S2 Commit allowed proceeds | `test_s2_commit_allowed_by_policy_proceeds` | `policy_provider_tests.rs` |
| S3 dry_run denied before writes | `test_s3_dry_run_policy_denied_before_any_writes` | `policy_provider_tests.rs` |
| S4 Backend-agnostic indirection | `test_s4_engine_depends_on_policy_provider_trait` | `policy_provider_tests.rs` |
| S5 AnchorPolicy adapter | `test_s5_anchor_adapter_matches_never_anchored` | `policy_provider_tests.rs` |
| S6 Export returns obligations | `test_s6_export_returns_all_obligations` | `policy_provider_tests.rs` |
| S7 Export deterministic | `test_s7_export_is_deterministic` | `policy_provider_tests.rs` |
| S8 Export malformed markers | `test_s8_export_fails_on_malformed_markers` | `policy_provider_tests.rs` |
| S9 Export unknown policy_ref | `test_s9_export_fails_policy_not_found` | `policy_provider_tests.rs` |
| S10 policy_list stable + versions | `test_s10_policy_list_stable_ids_and_versions` | `policy_provider_tests.rs` |
| S11 policy_read full text | `test_s11_policy_read_returns_full_text` | `policy_provider_tests.rs` |
| S12 manifest policy_ref | `test_s12_manifest_policy_ref_from_committed_snapshot` | `policy_provider_tests.rs` |
| S13 PolicyRefMissing | `test_s13_empty_policy_ref_returns_policy_ref_missing` | `policy_provider_tests.rs` |
| S14 ExportTooLarge | `test_s14_export_too_large_error` | `policy_provider_tests.rs` |
| S15 PolicyParseError | `test_s15_invalid_utf8_returns_parse_error` | `policy_provider_tests.rs` |

---

## Acceptance Gate Evidence

| Gate | Result |
|------|--------|
| `make lint` | ✅ Pass (0 warnings, fmt clean) |
| `make test` | ✅ Pass (all suites pass; 15 new scenario tests) |
| `make coverage-check` | ✅ 80.06% (≥ 80% threshold) |
| `make coverage-html` | ✅ Generated `coverage/tarpaulin-report.html` |
| Documentation | ✅ All 5 doc artefacts produced (see below) |

---

## Documentation Produced

| Artefact | Type | Status |
|----------|------|--------|
| `docs/policy-system.md` | New product doc | ✅ Created |
| `docs/action-read-tools.md` | Updated (4 new variants + updated signature + error table) | ✅ Updated |
| `crates/ettlex-core/README.md` | Updated (PolicyProvider, adapter, 6 new error kinds) | ✅ Updated |
| `crates/ettlex-store/README.md` | Updated (FilePolicyProvider section) | ✅ Updated |
| `crates/ettlex-engine/README.md` | Updated (4 new query variants + error table) | ✅ Updated |
| `policy_provider.rs` rustdoc | Full `//!`/`///` in all public items | ✅ |
| `file_policy_provider.rs` rustdoc | Full `//!`/`///` in all public items | ✅ |

---

## Files Changed

**New files:**
- `crates/ettlex-core/src/policy_provider.rs`
- `crates/ettlex-store/src/file_policy_provider.rs`
- `crates/ettlex-engine/tests/policy_provider_tests.rs`
- `policies/codegen_handoff_policy_v1.md`
- `docs/policy-system.md`

**Modified files:**
- `crates/ettlex-core/src/errors.rs` — 5 new `ExErrorKind` variants
- `crates/ettlex-core/src/lib.rs` — `pub mod policy_provider`
- `crates/ettlex-core/src/policy.rs` — `PolicyProviderAnchorAdapter`
- `crates/ettlex-engine/src/commands/engine_command.rs` — `&dyn PolicyProvider` signature
- `crates/ettlex-engine/src/commands/snapshot.rs` — `&dyn PolicyProvider` + `PolicyRefMissing` guard
- `crates/ettlex-engine/src/commands/engine_query.rs` — 4 new variants + 4th arg
- `crates/ettlex-engine/src/commands/read_tools.rs` — `PolicyReadResult`, `PolicyExportResult`
- `crates/ettlex-store/src/lib.rs` — `pub mod file_policy_provider`
- `crates/ettlex-cli/src/commands/snapshot.rs` — `&NoopPolicyProvider`
- All engine test files — updated `apply_engine_command`/`apply_engine_query` call sites

---

## Constraints Deferred

None. All 15 scenarios implemented. Phase 2 selective anchoring (reading anchor state from policy documents) noted in `PolicyProviderAnchorAdapter` as future work.
