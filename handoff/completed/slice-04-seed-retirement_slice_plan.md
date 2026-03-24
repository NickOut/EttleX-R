# Slice 04 — Seed System Retirement: Binding Execution Plan

---

## 1. Slice Identifier

`slice-04-seed-retirement`

---

## 2. Change Classification

**D: Refactor-only** — no new behaviour. All seed functionality was retired functionally in
Slice 03 (`import_seed` returns `NotImplemented`). This slice removes the dead infrastructure.

---

## 3. Slice Boundary Declaration

### Crates in scope

| Crate | Modules / Files changed |
|---|---|
| `ettlex-store` | `src/seed/` (delete entire directory), `src/lib.rs` (remove seed declaration + update docstring), `README.md` |
| `ettlex-cli` | `src/commands/seed.rs` (delete), `src/commands/mod.rs` (remove `pub mod seed;`), `src/main.rs` (remove `Seed` variant + match arm) |

### New test files (conformance)

| Crate | File |
|---|---|
| `ettlex-store` | `tests/slice_04_conformance_tests.rs` (new, conformance checks) |

### Test files deleted (seed tests)

| File | Reason |
|---|---|
| `crates/ettlex-store/tests/seed_parse_test.rs` | 5 tests exercising deleted seed module |
| `crates/ettlex-store/tests/ep_content_digest_tests.rs` | No-op test (1 test), references Slice 04 for cleanup |
| `crates/ettlex-store/tests/round_trip_test.rs` | No-op test (1 test), references Slice 04 for cleanup |

### Fixture files deleted

| File |
|---|
| `crates/ettlex-store/tests/fixtures/seed_minimal.yaml` |
| `crates/ettlex-store/tests/fixtures/seed_full.yaml` |
| `crates/ettlex-store/tests/fixtures/seed_invalid_duplicate_ordinal.yaml` |
| `crates/ettlex-store/tests/fixtures/seed_invalid_schema_version.yaml` |

### Crates and modules that are read-only (outside boundary)

- `ettlex-core` — `seed_digest: Option<String>` field on `SnapshotManifest` is **kept** (removing it would change the manifest JSON format and break existing snapshot tests). It will always be `None` in practice.
- `ettlex-engine` — no seed references; read-only
- `ettlex-mcp` — no seed references; read-only
- `ettlex-memory` — no seed references; read-only
- All other crates — read-only

### Infrastructure exceptions

- `makefile` — SLICE_TEST_FILTER updated to add new conformance test names. **Justified**: protocol requirement.
- `handoff/slice_registry.toml` — new slice entry appended. **Justified**: protocol requirement.

---

## 4. Replacement Targets

### `ettlex-store/src/seed/` (entire module — 6 files)

| File | Status |
|---|---|
| `src/seed/mod.rs` | **Deleted** (superseded, not extended) |
| `src/seed/importer.rs` | **Deleted** (superseded, not extended) |
| `src/seed/format_v0.rs` | **Deleted** (superseded, not extended) |
| `src/seed/parser.rs` | **Deleted** (superseded, not extended) |
| `src/seed/digest.rs` | **Deleted** (superseded, not extended) |
| `src/seed/provenance.rs` | **Deleted** (superseded, not extended) |

**Post-slice structural invariant**: `ettlex-store` MUST NOT have a `seed` module. `src/lib.rs` MUST NOT declare `pub mod seed`.

### `ettlex-cli/src/commands/seed.rs`

**Deleted** (superseded, not extended).

**Post-slice structural invariant**: `ettlex-cli` MUST NOT have a `seed` command. `src/commands/mod.rs` MUST NOT declare `pub mod seed`. `src/main.rs` MUST NOT have a `Commands::Seed` variant.

---

## 5. Layer Coverage Declaration

This slice covers:

- **Store layer**: seed module removal from `ettlex-store`
- **CLI layer**: seed command removal from `ettlex-cli`

Both layers will be represented in the conformance test suite (file-absence assertions).

No Engine, MCP, Memory, or Core layer changes.

---

## 6. Deletion Impact Analysis

### Scan results

**Seed test files and inline test functions:**

| Deleted Entity | Affected Test File | Affected Test Function(s) | Disposition | Reason |
|---|---|---|---|---|
| `seed/importer.rs` | `src/seed/importer.rs` (inline) | `test_import_failure_rollback` | DELETE (with file) | File deleted |
| `seed/format_v0.rs` | `src/seed/format_v0.rs` (inline) | `test_parse_minimal_seed_inline`, etc. | DELETE (with file) | File deleted |
| `seed/parser.rs` | `src/seed/parser.rs` (inline) | 5 inline parser tests | DELETE (with file) | File deleted |
| `seed/digest.rs` | `src/seed/digest.rs` (inline) | `test_seed_digest_stable`, `test_seed_digest_format_independent`, `test_seed_digest_what_polymorphism`, `test_seed_digest_stable_with_sorting` | DELETE (with file) | File deleted |
| `seed/provenance.rs` | `src/seed/provenance.rs` (inline) | 2 inline provenance tests | DELETE (with file) | File deleted |
| `seed_parse_test.rs` | `crates/ettlex-store/tests/seed_parse_test.rs` | `test_parse_minimal_seed`, `test_parse_full_seed`, `test_reject_invalid_schema_version`, `test_reject_duplicate_ordinals`, `test_what_polymorphism` | DELETE | Seed module deleted |
| `ep_content_digest_tests.rs` | `crates/ettlex-store/tests/ep_content_digest_tests.rs` | `test_ep_content_digest_retired` | DELETE | No-op, pre-authorised in Slice 03 PAFR for removal in Slice 04 |
| `round_trip_test.rs` | `crates/ettlex-store/tests/round_trip_test.rs` | `test_round_trip_retired` | DELETE | No-op, pre-authorised in Slice 03 PAFR for removal in Slice 04 |
| `commands/seed.rs` | `crates/ettlex-cli/src/commands/seed.rs` | (no test file; inline logic only) | DELETE | CLI seed command removed |

**References in tests outside the boundary that will NOT break:**

| File | Reference type | Impact |
|---|---|---|
| `crates/ettlex-core/tests/snapshot_manifest_tests.rs` | `manifest.seed_digest` field (kept in struct) | NO IMPACT — field is retained |
| `crates/ettlex-core/tests/snapshot_diff_tests.rs` | `"seed_digest": null` in JSON | NO IMPACT — field is retained |
| `crates/ettlex-mcp/tests/mcp_missing_tools_tests.rs` | Helper fn `seed_decision()`, `seed_profile()` (test data generators, NOT seed importer) | NO IMPACT — different `seed` meaning |
| `crates/ettlex-mcp/tests/mcp_integration_tests.rs` | Same helper fns | NO IMPACT |

**None of the seed test function names are registered in `SLICE_TEST_FILTER`.** Deleting them has no impact on `make test-slice`.

---

## 7. Scenario Sequence (Destructive Slice)

Scenarios follow the mandatory destructive ordering: retire tests first, then remove code.

1. **SC-S04-01** — Write conformance tests (file-absence assertions) → RED
2. **SC-S04-02** — Delete seed test files and fixtures → GREEN SC-S04-01
3. **SC-S04-03** — Delete CLI seed command (seed.rs, wiring) → GREEN remaining SC-S04 tests
4. **SC-S04-04** — Delete store seed module (6 files, lib.rs update) → GREEN remaining SC-S04 tests
5. **SC-S04-05** — Update documentation (README, docstring) → 4C doc step

No schema migration is required: the seed module wrote to `ettles` and `provenance_events` tables
(which still exist and are managed by other modules). No dedicated seed tables exist in the schema.

---

## 6 (continued). Pre-Authorised Failure Registry (PAFR)

No NEW pre-authorised failures are introduced by this slice.

This slice **closes** two pre-authorised failures from Slice 02:
- `ettlex-store/src/seed/importer.rs (7 fns)` — will be removed by SC-S04-04
- `make coverage-check` at 69% — may improve or remain pre-authorised (see note below)

**Coverage note**: After seed module removal, overall coverage will change. The seed module had
inline test coverage of its own source lines. Deleting both the source and inline tests should be
roughly neutral on the percentage. The primary coverage deficit is the snapshot pipeline stub
returning `NotImplemented` (Slice 03 PAFR). Coverage at or below 80% remains pre-authorised
until the snapshot pipeline is re-implemented in a future slice.

---

## 8. Scenario Inventory

### SC-S04-01: Write conformance tests asserting seed absence

- **Layer(s)**: Store, CLI (file-system assertions; no crate imports needed)
- **Expected error kind**: N/A (conformance / compile/runtime pass)
- **Predicted RED failure reason**: Tests assert files do not exist, but they DO exist at the time of writing. Each test will `assert!(!Path::exists())` and fail RED.
- **Minimal production module**: None. This scenario only creates the test file; no source changes.
- **Test file**: `crates/ettlex-store/tests/slice_04_conformance_tests.rs`
- **Test functions** (7):
  1. `test_s04_seed_module_absent_from_store_lib`
  2. `test_s04_seed_source_files_absent`
  3. `test_s04_seed_cli_command_file_absent`
  4. `test_s04_seed_not_in_cli_commands_mod`
  5. `test_s04_seed_not_in_cli_main`
  6. `test_s04_seed_parse_test_file_absent`
  7. `test_s04_seed_fixtures_absent`

### SC-S04-02: Delete seed test files and fixtures → GREEN SC-S04-01 subset

- **Layer(s)**: Store (test infrastructure)
- **Expected error kind**: Compile error (if test file uses `ettlex_store::seed::*`) or test failure (file still exists)
- **Predicted RED failure reason**: File-existence assertions fail (see SC-S04-01)
- **Minimal production module**: Delete:
  - `crates/ettlex-store/tests/seed_parse_test.rs`
  - `crates/ettlex-store/tests/ep_content_digest_tests.rs`
  - `crates/ettlex-store/tests/round_trip_test.rs`
  - `crates/ettlex-store/tests/fixtures/seed_minimal.yaml`
  - `crates/ettlex-store/tests/fixtures/seed_full.yaml`
  - `crates/ettlex-store/tests/fixtures/seed_invalid_duplicate_ordinal.yaml`
  - `crates/ettlex-store/tests/fixtures/seed_invalid_schema_version.yaml`
- **Post-state**: `test_s04_seed_parse_test_file_absent` and `test_s04_seed_fixtures_absent` go GREEN.

### SC-S04-03: Delete CLI seed command → GREEN CLI conformance tests

- **Layer(s)**: CLI
- **Expected error kind**: Compile error (if seed.rs still referenced in mod.rs/main.rs)
- **Predicted RED failure reason**: `test_s04_seed_cli_command_file_absent`, `test_s04_seed_not_in_cli_commands_mod`, `test_s04_seed_not_in_cli_main` fail RED (files still contain seed references)
- **Minimal production module**:
  - Delete `crates/ettlex-cli/src/commands/seed.rs`
  - Remove `pub mod seed;` from `crates/ettlex-cli/src/commands/mod.rs`
  - Remove `Seed(commands::seed::SeedArgs)` variant + match arm from `crates/ettlex-cli/src/main.rs`
  - Remove `rusqlite` direct use in main.rs if only referenced by seed command

### SC-S04-04: Delete store seed module → GREEN store conformance tests

- **Layer(s)**: Store
- **Expected error kind**: Compile error (if seed module imported by anything remaining)
- **Predicted RED failure reason**: `test_s04_seed_module_absent_from_store_lib` and `test_s04_seed_source_files_absent` fail RED (seed module still exists)
- **Minimal production module**:
  - Delete entire `crates/ettlex-store/src/seed/` directory (6 files)
  - Remove `pub mod seed;` from `crates/ettlex-store/src/lib.rs`
  - Update `ettlex-store/src/lib.rs` docstring (remove seed mention)
  - Verify compile: `make build`

### SC-S04-05: Documentation update (4C step)

- Update `crates/ettlex-store/README.md` — remove seed capability documentation
- Update `crates/ettlex-cli/README.md` if it exists — remove seed command documentation
- Run `make lint` (clean)
- Run `make doc` (no new warnings)

---

## 9. Makefile Update Plan

The following 7 test names are added to `SLICE_TEST_FILTER`:

```
test_s04_seed_module_absent_from_store_lib
test_s04_seed_source_files_absent
test_s04_seed_cli_command_file_absent
test_s04_seed_not_in_cli_commands_mod
test_s04_seed_not_in_cli_main
test_s04_seed_parse_test_file_absent
test_s04_seed_fixtures_absent
```

The existing `test` and `test-full` targets are **unchanged**.

---

## 10. Slice Registry Update Plan

The following TOML entry will be appended to `handoff/slice_registry.toml` on completion:

```toml
[[slice]]
id = "slice-04-seed-retirement"
ettle_id = "ettle:slice-04"
description = "Seed System Retirement — delete ettlex-store seed module (6 files), delete CLI seed command, delete seed test files and fixtures, update store lib.rs and README"
layers = ["store", "cli"]
status = "complete"

[[slice.tests]]
crate = "ettlex-store"
file = "tests/slice_04_conformance_tests.rs"
test = "test_s04_seed_module_absent_from_store_lib"
scenario = "SC-S04-01"

[[slice.tests]]
crate = "ettlex-store"
file = "tests/slice_04_conformance_tests.rs"
test = "test_s04_seed_source_files_absent"
scenario = "SC-S04-02"

[[slice.tests]]
crate = "ettlex-store"
file = "tests/slice_04_conformance_tests.rs"
test = "test_s04_seed_cli_command_file_absent"
scenario = "SC-S04-03"

[[slice.tests]]
crate = "ettlex-store"
file = "tests/slice_04_conformance_tests.rs"
test = "test_s04_seed_not_in_cli_commands_mod"
scenario = "SC-S04-04"

[[slice.tests]]
crate = "ettlex-store"
file = "tests/slice_04_conformance_tests.rs"
test = "test_s04_seed_not_in_cli_main"
scenario = "SC-S04-05"

[[slice.tests]]
crate = "ettlex-store"
file = "tests/slice_04_conformance_tests.rs"
test = "test_s04_seed_parse_test_file_absent"
scenario = "SC-S04-06"

[[slice.tests]]
crate = "ettlex-store"
file = "tests/slice_04_conformance_tests.rs"
test = "test_s04_seed_fixtures_absent"
scenario = "SC-S04-07"

[[slice.pre_authorised_failures]]
test = "make coverage-check"
reason = "Coverage pre-authorized at 69% (threshold 80%) from Slice 03. Snapshot pipeline stub (NotImplemented) is untestable at depth. Seed removal effect on coverage is roughly neutral."
```

---

## 11. Acceptance Strategy

| Gate | Command | Pass Condition |
|---|---|---|
| Compile | `make build` | Zero errors after each SC |
| Lint | `make lint` | Clean (zero warnings/errors) |
| Slice tests | `make test-slice` | All 249 registered tests pass (212 existing + 7 new) |
| Full suite | `make test` | All tests pass or are `#[ignore]`d (no new failures) |
| Doc | `make doc` | No new warnings |
| Coverage | `make coverage-check` | Pre-authorized: 69% threshold; no regression below current level |

---

## 12. Plan Integrity Declaration

> No production code will be written before RED evidence exists.
> No code outside the declared slice boundary will be modified except the Makefile and handoff/slice_registry.toml (and any declared infrastructure exceptions).
> All replacement targets have been identified and their post-slice structural invariants declared.
