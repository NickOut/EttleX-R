// Integration tests for EP content_digest persistence
// Covers seed_store_v6 scenarios:
//   - "Seed import persists full EP bodies inline and persists content_digest"
//   - "Seed import MUST NOT persist HOW-only EP bodies (regression guard)"

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

/// Helper: import seed_minimal.yaml and return the connection
fn import_minimal(conn: &mut Connection) {
    let seed_path = fixtures_dir().join("seed_minimal.yaml");
    ettlex_store::seed::import_seed(&seed_path, conn).unwrap();
}

#[test]
fn test_seed_import_content_digest_non_null() {
    // Given: a seed is imported
    let mut conn = setup_test_db();
    import_minimal(&mut conn);

    // When: we query the raw content_digest column
    let digest: Option<String> = conn
        .query_row(
            "SELECT content_digest FROM eps WHERE id = 'ep:root:0'",
            [],
            |row| row.get(0),
        )
        .unwrap();

    // Then: content_digest must be non-null and 64 hex chars
    assert!(digest.is_some(), "content_digest must not be NULL");
    let d = digest.unwrap();
    assert_eq!(d.len(), 64, "SHA-256 hex digest must be 64 chars, got: {d}");
}

#[test]
fn test_seed_import_content_digest_correct_value() {
    // Given: a seed is imported
    let mut conn = setup_test_db();
    import_minimal(&mut conn);

    // Compute expected digest: SHA-256 of alphabetical BTreeMap JSON
    let mut map = std::collections::BTreeMap::new();
    map.insert("how", "Import via Seed Format v0");
    map.insert("what", "A minimal root Ettle");
    map.insert("why", "Bootstrap the semantic kernel");
    let json = serde_json::to_string(&map).unwrap();
    use sha2::Digest;
    let mut h = sha2::Sha256::new();
    h.update(json.as_bytes());
    let expected = hex::encode(h.finalize());

    // When: we query the stored digest
    let stored: String = conn
        .query_row(
            "SELECT content_digest FROM eps WHERE id = 'ep:root:0'",
            [],
            |row| row.get(0),
        )
        .unwrap();

    // Then: stored digest matches expected
    assert_eq!(
        stored, expected,
        "content_digest must match SHA-256 of canonical JSON"
    );
}

#[test]
fn test_seed_import_persists_why_and_what_bodies() {
    // Given: a seed is imported
    let mut conn = setup_test_db();
    import_minimal(&mut conn);

    // When: we inspect content_inline
    let content_inline: String = conn
        .query_row(
            "SELECT content_inline FROM eps WHERE id = 'ep:root:0'",
            [],
            |row| row.get(0),
        )
        .unwrap();

    let v: serde_json::Value = serde_json::from_str(&content_inline).unwrap();

    // Then: why and what must be non-empty (regression guard against HOW-only storage)
    assert_eq!(
        v["why"].as_str().unwrap_or(""),
        "Bootstrap the semantic kernel",
        "WHY must be persisted in content_inline"
    );
    assert_eq!(
        v["what"].as_str().unwrap_or(""),
        "A minimal root Ettle",
        "WHAT must be persisted in content_inline"
    );

    // And: content_digest must be non-null on the same row (WHY+WHAT guard)
    let digest: Option<String> = conn
        .query_row(
            "SELECT content_digest FROM eps WHERE id = 'ep:root:0'",
            [],
            |row| row.get(0),
        )
        .unwrap();
    assert!(
        digest.as_deref().is_some_and(|d| !d.is_empty()),
        "content_digest must be non-null when WHY/WHAT are present"
    );

    // And: the loaded EP model also exposes the digest field
    let store = ettlex_store::repo::hydration::load_tree(&conn).unwrap();
    let ep = store.get_ep("ep:root:0").unwrap();
    assert!(
        !ep.content_digest.is_empty(),
        "Ep::content_digest field must be non-empty after reload"
    );
}

#[test]
fn test_seed_import_content_digest_stable_on_reload() {
    // Given: a seed is imported
    let mut conn = setup_test_db();
    import_minimal(&mut conn);

    // When: we read the raw stored digest
    let stored_digest: String = conn
        .query_row(
            "SELECT content_digest FROM eps WHERE id = 'ep:root:0'",
            [],
            |row| row.get(0),
        )
        .unwrap();

    // And: reload the EP through the hydration layer
    let store = ettlex_store::repo::hydration::load_tree(&conn).unwrap();
    let reloaded_ep = store.get_ep("ep:root:0").unwrap();

    // Then: the reloaded Ep's content_digest matches what was stored
    assert_eq!(
        reloaded_ep.content_digest, stored_digest,
        "Ep::content_digest after reload must equal the stored DB value"
    );
}
