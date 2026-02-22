// Integration tests for round-trip determinism
// Covers Gherkin scenarios E.1-E.2: Round-trip stability
// ACCEPTANCE GATE: Import → reload → render must be byte-for-byte identical

use rusqlite::Connection;
use std::path::PathBuf;

fn setup_test_db() -> Connection {
    let mut conn = Connection::open_in_memory().unwrap();
    ettlex_store::migrations::apply_migrations(&mut conn).unwrap();
    conn
}

fn fixtures_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests")
        .join("fixtures")
}

#[test]
fn test_import_reload_render_stable() {
    // Given: A seed file
    let mut conn = setup_test_db();
    let seed_path = fixtures_dir().join("seed_full.yaml");

    // When: We import the seed
    let seed_digest = ettlex_store::seed::import_seed(&seed_path, &mut conn).unwrap();
    assert!(!seed_digest.is_empty(), "Should return seed digest");

    // And: Reload from SQLite into Store (first time)
    let store1 = ettlex_store::repo::hydration::load_tree(&conn).unwrap();

    // And: Render the root Ettle (first time)
    let output1 = ettlex_core::render::ettle_render::render_ettle(&store1, "ettle:root").unwrap();

    // And: Reload from SQLite into Store (second time)
    let store2 = ettlex_store::repo::hydration::load_tree(&conn).unwrap();

    // And: Render the root Ettle (second time)
    let output2 = ettlex_core::render::ettle_render::render_ettle(&store2, "ettle:root").unwrap();

    // Then: Both renders are byte-for-byte identical
    assert_eq!(
        output1, output2,
        "Round-trip should produce identical output"
    );

    // And: Output should contain expected content
    assert!(
        output1.contains("EttleX Product"),
        "Should contain ettle title"
    );
    assert!(
        output1.contains("Storage Spine"),
        "Should contain child ettle"
    );
    assert!(
        output1.contains("SQLite + CAS storage"),
        "Should contain EP content"
    );
}

#[test]
fn test_reload_ordering_deterministic() {
    // Given: A database with multiple entities in random order
    let mut conn = setup_test_db();
    let seed_path = fixtures_dir().join("seed_full.yaml");
    ettlex_store::seed::import_seed(&seed_path, &mut conn).unwrap();

    // When: We reload twice
    let store1 = ettlex_store::repo::hydration::load_tree(&conn).unwrap();
    let store2 = ettlex_store::repo::hydration::load_tree(&conn).unwrap();

    // Then: Both stores have the same Ettles (in any order, since HashMap)
    let mut ids1: Vec<String> = store1.list_ettles().iter().map(|e| e.id.clone()).collect();
    let mut ids2: Vec<String> = store2.list_ettles().iter().map(|e| e.id.clone()).collect();
    ids1.sort();
    ids2.sort();

    assert_eq!(ids1, ids2, "Reload should be deterministic");

    // And: Renders should be identical (proves determinism)
    let render1 = ettlex_core::render::ettle_render::render_ettle(&store1, "ettle:root").unwrap();
    let render2 = ettlex_core::render::ettle_render::render_ettle(&store2, "ettle:root").unwrap();

    assert_eq!(render1, render2, "Renders should be deterministic");
}

#[test]
fn test_minimal_seed_round_trip() {
    // Given: A minimal seed
    let mut conn = setup_test_db();
    let seed_path = fixtures_dir().join("seed_minimal.yaml");

    // When: We import and reload
    ettlex_store::seed::import_seed(&seed_path, &mut conn).unwrap();
    let store = ettlex_store::repo::hydration::load_tree(&conn).unwrap();

    // Then: We can render the root
    let output = ettlex_core::render::ettle_render::render_ettle(&store, "ettle:root").unwrap();

    // And: Output contains expected content
    assert!(output.contains("EttleX Root"), "Should contain title");
    assert!(
        output.contains("Bootstrap the semantic kernel"),
        "Should contain WHY"
    );
    assert!(
        output.contains("A minimal root Ettle"),
        "Should contain WHAT"
    );
    assert!(
        output.contains("Import via Seed Format v0"),
        "Should contain HOW"
    );
}

#[test]
fn test_round_trip_preserves_links() {
    // Given: A seed with links
    let mut conn = setup_test_db();
    let seed_path = fixtures_dir().join("seed_full.yaml");

    // When: We import and reload
    ettlex_store::seed::import_seed(&seed_path, &mut conn).unwrap();
    let store = ettlex_store::repo::hydration::load_tree(&conn).unwrap();

    // Then: Parent-child links are preserved
    let child = store.get_ettle("ettle:store").unwrap();
    assert_eq!(
        child.parent_id,
        Some("ettle:root".to_string()),
        "Child should have parent_id"
    );

    // And: EP child_ettle_id is preserved
    let parent_ep = store.get_ep("ep:root:1").unwrap();
    assert_eq!(
        parent_ep.child_ettle_id,
        Some("ettle:store".to_string()),
        "EP should have child_ettle_id"
    );
}
