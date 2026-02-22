//! Seed importer orchestration
//!
//! Imports seeds by calling Phase 0.5 CRUD operations and persisting to SQLite

#![allow(clippy::result_large_err)]

use crate::errors::Result;
use crate::repo::SqliteRepo;
use crate::seed::{compute_seed_digest, parse_seed_file_with_db};
use ettlex_core::ops::store::Store;
use rusqlite::Connection;
use std::path::Path;

/// Import a seed file into the database
///
/// This is the main entry point for seed import. It:
/// 1. Parses and validates the seed YAML (checking database for cross-seed references)
/// 2. Computes the seed digest
/// 3. Creates Ettles and EPs using Phase 0.5 ops
/// 4. Persists to SQLite within a transaction
/// 5. Emits provenance events
///
/// Returns the seed digest on success
pub fn import_seed(path: &Path, conn: &mut Connection) -> Result<String> {
    // Parse seed (pass connection to allow cross-seed reference validation)
    let seed = parse_seed_file_with_db(path, Some(conn))?;

    // Compute seed digest
    let seed_digest = compute_seed_digest(&seed);

    // Start transaction
    let tx = conn.transaction().map_err(crate::errors::from_rusqlite)?;

    // Emit provenance: started
    crate::seed::provenance::emit_started(&tx, &seed_digest)?;

    // Create an in-memory store for Phase 0.5 operations
    let mut store = Store::new();

    // Import each Ettle
    for seed_ettle in &seed.ettles {
        // Ettle should have at least one EP (EP0)
        if seed_ettle.eps.is_empty() {
            continue; // Skip empty ettles
        }

        let ep0 = &seed_ettle.eps[0];

        // Create Ettle with EP0 using Phase 0.5 ops
        // Note: We can't use the seed IDs directly because create_ettle generates UUIDs
        // For Phase 1, we'll create with generated IDs, then update to seed IDs
        // This is a simplification for bootstrap - Phase 2 will handle stable IDs properly

        // For now, create a basic Ettle directly in the store
        let mut ettle = ettlex_core::model::Ettle::new(
            seed_ettle.id.clone(), // Use seed ID directly for stable identity
            seed_ettle.title.clone(),
        );

        // Create EP0
        let ep0_model = ettlex_core::model::Ep::new(
            ep0.id.clone(),
            seed_ettle.id.clone(),
            ep0.ordinal,
            ep0.normative,
            ep0.why.clone(),
            ep0.what.clone(),
            ep0.how.clone(),
        );

        // Add EP0 to ettle
        ettle.add_ep_id(ep0.id.clone());

        // Insert into in-memory store
        store.insert_ettle(ettle.clone());
        store.insert_ep(ep0_model.clone());

        // Persist to SQLite
        SqliteRepo::persist_ettle_tx(&tx, &ettle)?;
        SqliteRepo::persist_ep_tx(&tx, &ep0_model)?;

        // Additional EPs (EP1+)
        for seed_ep in &seed_ettle.eps[1..] {
            let ep_model = ettlex_core::model::Ep::new(
                seed_ep.id.clone(),
                seed_ettle.id.clone(),
                seed_ep.ordinal,
                seed_ep.normative,
                seed_ep.why.clone(),
                seed_ep.what.clone(),
                seed_ep.how.clone(),
            );

            // Update ettle's ep_ids list
            if let Ok(ettle_mut) = store.get_ettle_mut(&seed_ettle.id) {
                ettle_mut.add_ep_id(seed_ep.id.clone());
            }

            // Insert into store and persist
            store.insert_ep(ep_model.clone());
            SqliteRepo::persist_ep_tx(&tx, &ep_model)?;

            // Re-persist ettle with updated ep_ids
            if let Ok(ettle) = store.get_ettle(&seed_ettle.id) {
                SqliteRepo::persist_ettle_tx(&tx, ettle)?;
            }
        }

        // Emit provenance: applied
        crate::seed::provenance::emit_applied(&tx, &seed_digest, &seed_ettle.id)?;
    }

    // Handle links: update EP.child_ettle_id
    for link in &seed.links {
        // Try to get EP from in-memory store first
        if let Ok(ep) = store.get_ep_mut(&link.parent_ep) {
            ep.child_ettle_id = Some(link.child.clone());
            // Persist updated EP
            SqliteRepo::persist_ep_tx(&tx, ep)?;
        } else {
            // EP not in current seed - load from database and update
            if let Some(mut ep) = SqliteRepo::get_ep(&tx, &link.parent_ep)? {
                ep.child_ettle_id = Some(link.child.clone());
                // Persist updated EP
                SqliteRepo::persist_ep_tx(&tx, &ep)?;
            }
        }
    }

    // Update parent_id for child Ettles
    for link in &seed.links {
        // Try to get Ettle from in-memory store first
        if let Ok(child_ettle) = store.get_ettle_mut(&link.child) {
            child_ettle.parent_id = Some(link.parent.clone());
            // Persist updated Ettle
            SqliteRepo::persist_ettle_tx(&tx, child_ettle)?;
        } else {
            // Ettle not in current seed - load from database and update
            if let Some(mut child_ettle) = SqliteRepo::get_ettle(&tx, &link.child)? {
                child_ettle.parent_id = Some(link.parent.clone());
                // Persist updated Ettle
                SqliteRepo::persist_ettle_tx(&tx, &child_ettle)?;
            }
        }
    }

    // Emit provenance: completed
    crate::seed::provenance::emit_completed(&tx, &seed_digest)?;

    // Commit transaction
    tx.commit().map_err(crate::errors::from_rusqlite)?;

    Ok(seed_digest)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::migrations;
    use std::path::PathBuf;

    fn setup_test_db() -> Connection {
        let mut conn = Connection::open_in_memory().unwrap();
        migrations::apply_migrations(&mut conn).unwrap();
        conn
    }

    fn fixtures_dir() -> PathBuf {
        PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("tests")
            .join("fixtures")
    }

    #[test]
    fn test_import_minimal_seed() {
        let mut conn = setup_test_db();
        let path = fixtures_dir().join("seed_minimal.yaml");

        let result = import_seed(&path, &mut conn);
        assert!(result.is_ok(), "Import should succeed: {:?}", result.err());

        // Verify Ettle was created
        let ettle_count: i64 = conn
            .query_row("SELECT COUNT(*) FROM ettles", [], |row| row.get(0))
            .unwrap();
        assert_eq!(ettle_count, 1);

        // Verify EP was created
        let ep_count: i64 = conn
            .query_row("SELECT COUNT(*) FROM eps", [], |row| row.get(0))
            .unwrap();
        assert_eq!(ep_count, 1);

        // Verify provenance events
        let prov_count: i64 = conn
            .query_row("SELECT COUNT(*) FROM provenance_events", [], |row| {
                row.get(0)
            })
            .unwrap();
        assert!(
            prov_count >= 3,
            "Should have started, applied, completed events"
        );
    }

    #[test]
    fn test_import_full_seed_with_links() {
        let mut conn = setup_test_db();
        let path = fixtures_dir().join("seed_full.yaml");

        let result = import_seed(&path, &mut conn);
        assert!(result.is_ok(), "Import should succeed: {:?}", result.err());

        // Verify 2 Ettles were created
        let ettle_count: i64 = conn
            .query_row("SELECT COUNT(*) FROM ettles", [], |row| row.get(0))
            .unwrap();
        assert_eq!(ettle_count, 2);

        // Verify link was established (parent_id set)
        let has_parent: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM ettles WHERE parent_id IS NOT NULL",
                [],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(has_parent, 1, "Child ettle should have parent_id set");

        // Verify EP has child_ettle_id
        let has_child: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM eps WHERE child_ettle_id IS NOT NULL",
                [],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(has_child, 1, "Parent EP should have child_ettle_id set");
    }

    #[test]
    fn test_import_failure_rollback() {
        let mut conn = setup_test_db();
        let path = fixtures_dir().join("seed_invalid_duplicate_ordinal.yaml");

        // Import should fail due to validation
        let result = import_seed(&path, &mut conn);
        assert!(result.is_err(), "Import should fail on invalid seed");

        // Verify no Ettles were created (rollback)
        let ettle_count: i64 = conn
            .query_row("SELECT COUNT(*) FROM ettles", [], |row| row.get(0))
            .unwrap();
        assert_eq!(ettle_count, 0, "Rollback should remove all changes");

        // Verify no provenance events (rollback)
        let prov_count: i64 = conn
            .query_row("SELECT COUNT(*) FROM provenance_events", [], |row| {
                row.get(0)
            })
            .unwrap();
        assert_eq!(prov_count, 0, "Rollback should remove provenance events");
    }

    #[test]
    fn test_cross_seed_link_import() {
        let mut conn = setup_test_db();

        // Import parent seed first
        let parent_path = fixtures_dir().join("seed_full.yaml");
        let parent_result = import_seed(&parent_path, &mut conn);
        assert!(
            parent_result.is_ok(),
            "Parent seed import should succeed: {:?}",
            parent_result.err()
        );

        // Create a child seed that references entities from parent seed
        let child_yaml = r#"
schema_version: 0
project:
  name: cross-seed-test
ettles:
  - id: ettle:child
    title: "Child Ettle"
    eps:
      - id: ep:child:0
        ordinal: 0
        normative: true
        why: "Child why"
        what: "Child what"
        how: "Child how"
links:
  - parent: ettle:root
    parent_ep: ep:root:1
    child: ettle:child
"#;

        // Write child seed to temp file
        let temp_dir = std::env::temp_dir();
        let child_path = temp_dir.join("seed_cross_test.yaml");
        std::fs::write(&child_path, child_yaml).unwrap();

        // Import child seed (should validate parent against database)
        let child_result = import_seed(&child_path, &mut conn);
        assert!(
            child_result.is_ok(),
            "Child seed import should succeed: {:?}",
            child_result.err()
        );

        // Verify cross-seed link was created
        let parent_ep_child: Option<String> = conn
            .query_row(
                "SELECT child_ettle_id FROM eps WHERE id = 'ep:root:1'",
                [],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(
            parent_ep_child,
            Some("ettle:child".to_string()),
            "Parent EP should link to child Ettle"
        );

        // Verify child Ettle has parent_id set
        let child_parent: Option<String> = conn
            .query_row(
                "SELECT parent_id FROM ettles WHERE id = 'ettle:child'",
                [],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(
            child_parent,
            Some("ettle:root".to_string()),
            "Child Ettle should have parent_id set"
        );

        // Cleanup
        std::fs::remove_file(child_path).ok();
    }
}
