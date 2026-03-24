# Slice WIP — slice-04-seed-retirement

**Ettle ID:** ettle:slice-04
**Status:** IN PROGRESS

## Conformance Table

| SC | Layer(s) | Planned Test | RED Evidence | GREEN Evidence | Code Files | Doc Files | Doc Evidence | Status |
|----|----------|-------------|--------------|----------------|------------|-----------|--------------|--------|
| SC-S04-01 | Store, CLI | `test_s04_seed_module_absent_from_store_lib` | 7/7 fail: "must not declare pub mod seed after Slice 04" | 219/219 pass | `crates/ettlex-store/tests/slice_04_conformance_tests.rs` | — | — | DONE |
| SC-S04-01 | Store, CLI | `test_s04_seed_source_files_absent` | 7/7 fail (seed dir exists) | 219/219 pass | `crates/ettlex-store/tests/slice_04_conformance_tests.rs` | — | — | DONE |
| SC-S04-01 | Store, CLI | `test_s04_seed_cli_command_file_absent` | 7/7 fail (seed.rs exists) | 219/219 pass | `crates/ettlex-store/tests/slice_04_conformance_tests.rs` | — | — | DONE |
| SC-S04-01 | Store, CLI | `test_s04_seed_not_in_cli_commands_mod` | 7/7 fail (pub mod seed present) | 219/219 pass | `crates/ettlex-store/tests/slice_04_conformance_tests.rs` | — | — | DONE |
| SC-S04-01 | Store, CLI | `test_s04_seed_not_in_cli_main` | 7/7 fail (Seed( present) | 219/219 pass | `crates/ettlex-store/tests/slice_04_conformance_tests.rs` | — | — | DONE |
| SC-S04-01 | Store, CLI | `test_s04_seed_parse_test_file_absent` | 7/7 fail (file exists) | 219/219 pass | `crates/ettlex-store/tests/slice_04_conformance_tests.rs` | — | — | DONE |
| SC-S04-01 | Store, CLI | `test_s04_seed_fixtures_absent` | 7/7 fail (fixtures exist) | 219/219 pass | `crates/ettlex-store/tests/slice_04_conformance_tests.rs` | — | — | DONE |
| SC-S04-02 | Store | *(delete seed test files + fixtures)* | (RED via SC-S04-01) | 219/219 pass | Deleted: `tests/seed_parse_test.rs`, `tests/ep_content_digest_tests.rs`, `tests/round_trip_test.rs`, `tests/fixtures/seed_*.yaml` | — | — | DONE |
| SC-S04-03 | CLI | *(delete CLI seed command)* | (RED via SC-S04-01) | 219/219 pass | Deleted: `src/commands/seed.rs`; edited `src/commands/mod.rs`, `src/main.rs` | — | — | DONE |
| SC-S04-04 | Store | *(delete store seed module)* | (RED via SC-S04-01) | 219/219 pass | Deleted: `src/seed/` (6 files); edited `src/lib.rs` | `crates/ettlex-store/README.md`, `src/lib.rs` docstring | rustdoc clean, README updated | DONE |
| SC-S04-05 | Store, CLI | *(documentation update — 4C step)* | — | `make lint` clean, `make doc` no new warnings | — | `crates/ettlex-store/README.md`, `crates/ettlex-cli/README.md` | `make doc`: 2 pre-existing warnings (ettlex-core-types, ettlex-cli), 0 new | DONE |
