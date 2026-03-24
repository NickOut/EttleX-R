// Slice 04 conformance tests — Seed System Retirement
//
// These tests assert that all seed-related infrastructure has been removed.
// Each test fails RED while the seed code/files still exist, and goes GREEN
// once the corresponding deletion scenario has been completed.

use std::path::Path;

fn workspace_root() -> std::path::PathBuf {
    // CARGO_MANIFEST_DIR is crates/ettlex-store; workspace root is two levels up
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .parent()
        .unwrap()
        .to_path_buf()
}

fn store_crate() -> std::path::PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).to_path_buf()
}

fn cli_crate() -> std::path::PathBuf {
    workspace_root().join("crates").join("ettlex-cli")
}

// SC-S04-01 — Store: `pub mod seed` must not appear in ettlex-store/src/lib.rs

#[test]
fn test_s04_seed_module_absent_from_store_lib() {
    let lib_rs = store_crate().join("src").join("lib.rs");
    let content = std::fs::read_to_string(&lib_rs)
        .unwrap_or_else(|_| panic!("Could not read {}", lib_rs.display()));
    assert!(
        !content.contains("pub mod seed"),
        "ettlex-store/src/lib.rs must not declare `pub mod seed` after Slice 04"
    );
}

// SC-S04-02 — Store: seed source files must not exist

#[test]
fn test_s04_seed_source_files_absent() {
    let seed_dir = store_crate().join("src").join("seed");
    assert!(
        !seed_dir.exists(),
        "crates/ettlex-store/src/seed/ directory must not exist after Slice 04 (found: {})",
        seed_dir.display()
    );
}

// SC-S04-03 — CLI: commands/seed.rs must not exist

#[test]
fn test_s04_seed_cli_command_file_absent() {
    let seed_rs = cli_crate().join("src").join("commands").join("seed.rs");
    assert!(
        !seed_rs.exists(),
        "crates/ettlex-cli/src/commands/seed.rs must not exist after Slice 04 (found: {})",
        seed_rs.display()
    );
}

// SC-S04-04 — CLI: `pub mod seed` must not appear in ettlex-cli/src/commands/mod.rs

#[test]
fn test_s04_seed_not_in_cli_commands_mod() {
    let mod_rs = cli_crate().join("src").join("commands").join("mod.rs");
    let content = std::fs::read_to_string(&mod_rs)
        .unwrap_or_else(|_| panic!("Could not read {}", mod_rs.display()));
    assert!(
        !content.contains("pub mod seed"),
        "crates/ettlex-cli/src/commands/mod.rs must not declare `pub mod seed` after Slice 04"
    );
}

// SC-S04-05 — CLI: `Commands::Seed` must not appear in ettlex-cli/src/main.rs

#[test]
fn test_s04_seed_not_in_cli_main() {
    let main_rs = cli_crate().join("src").join("main.rs");
    let content = std::fs::read_to_string(&main_rs)
        .unwrap_or_else(|_| panic!("Could not read {}", main_rs.display()));
    assert!(
        !content.contains("Seed("),
        "crates/ettlex-cli/src/main.rs must not contain `Seed(` variant after Slice 04"
    );
    assert!(
        !content.contains("commands::seed"),
        "crates/ettlex-cli/src/main.rs must not reference `commands::seed` after Slice 04"
    );
}

// SC-S04-06 — Store: seed_parse_test.rs must not exist

#[test]
fn test_s04_seed_parse_test_file_absent() {
    let test_file = store_crate().join("tests").join("seed_parse_test.rs");
    assert!(
        !test_file.exists(),
        "crates/ettlex-store/tests/seed_parse_test.rs must not exist after Slice 04 (found: {})",
        test_file.display()
    );
}

// SC-S04-07 — Store: seed fixture YAML files must not exist in tests/fixtures/

#[test]
fn test_s04_seed_fixtures_absent() {
    let fixtures_dir = store_crate().join("tests").join("fixtures");
    let seed_fixtures = [
        "seed_minimal.yaml",
        "seed_full.yaml",
        "seed_invalid_duplicate_ordinal.yaml",
        "seed_invalid_schema_version.yaml",
    ];
    for fixture in &seed_fixtures {
        let path = fixtures_dir.join(fixture);
        assert!(
            !path.exists(),
            "Seed fixture {} must not exist after Slice 04 (found: {})",
            fixture,
            path.display()
        );
    }
}
