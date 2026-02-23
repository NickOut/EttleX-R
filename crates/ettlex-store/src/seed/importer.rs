//! Seed importer orchestration
//!
//! Imports seeds by calling Phase 0.5 CRUD operations and persisting to SQLite

#![allow(clippy::result_large_err)]

use crate::errors::Result;
use crate::repo::SqliteRepo;
use crate::seed::{compute_seed_digest, parse_seed_file_with_db};
use ettlex_core::ops::refinement_ops;
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

        // Check if ettle already exists in database
        let existing_ettle = SqliteRepo::get_ettle(&tx, &seed_ettle.id)?;
        let is_update = existing_ettle.is_some();

        // Get set of existing EP ordinals for this ettle
        let mut existing_ep_ordinals = std::collections::HashSet::new();
        if is_update {
            let mut stmt = tx
                .prepare("SELECT ordinal FROM eps WHERE ettle_id = ?1 AND deleted = 0")
                .map_err(crate::errors::from_rusqlite)?;
            let ordinal_rows = stmt
                .query_map([&seed_ettle.id], |row| row.get::<_, u32>(0))
                .map_err(crate::errors::from_rusqlite)?;

            for ordinal_result in ordinal_rows {
                existing_ep_ordinals.insert(ordinal_result.map_err(crate::errors::from_rusqlite)?);
            }
        }

        let mut ettle = if let Some(mut existing) = existing_ettle {
            // Update mode: Load existing ettle and its EPs into store
            let mut stmt = tx
                .prepare("SELECT id FROM eps WHERE ettle_id = ?1 AND deleted = 0")
                .map_err(crate::errors::from_rusqlite)?;
            let ep_rows = stmt
                .query_map([&seed_ettle.id], |row| row.get::<_, String>(0))
                .map_err(crate::errors::from_rusqlite)?;

            for ep_id_result in ep_rows {
                let ep_id = ep_id_result.map_err(crate::errors::from_rusqlite)?;
                if let Some(ep) = SqliteRepo::get_ep(&tx, &ep_id)? {
                    if !existing.ep_ids.contains(&ep_id) {
                        existing.ep_ids.push(ep_id.clone());
                    }
                    store.insert_ep(ep);
                }
            }
            store.insert_ettle(existing.clone());
            existing
        } else {
            // Create mode: New ettle - persist immediately before adding EPs
            let new_ettle =
                ettlex_core::model::Ettle::new(seed_ettle.id.clone(), seed_ettle.title.clone());
            // Persist new ettle first (required for FK constraints on EPs)
            SqliteRepo::persist_ettle_tx(&tx, &new_ettle)?;
            store.insert_ettle(new_ettle.clone());
            new_ettle
        };

        // Process all EPs from seed
        for seed_ep in &seed_ettle.eps {
            // Skip if EP with this ordinal already exists (don't overwrite)
            if existing_ep_ordinals.contains(&seed_ep.ordinal) {
                continue;
            }

            // Create new EP
            let ep_model = ettlex_core::model::Ep::new(
                seed_ep.id.clone(),
                seed_ettle.id.clone(),
                seed_ep.ordinal,
                seed_ep.normative,
                seed_ep.why.clone(),
                seed_ep.what.clone(),
                seed_ep.how.clone(),
            );

            // Add EP to ettle's ep_ids list if not already there
            if !ettle.ep_ids.contains(&seed_ep.id) {
                ettle.add_ep_id(seed_ep.id.clone());
            }

            // Insert into store and persist
            store.insert_ep(ep_model.clone());
            SqliteRepo::persist_ep_tx(&tx, &ep_model)?;
        }

        // Update store with modified ettle
        store.insert_ettle(ettle.clone());

        // Persist ettle (upsert will update if exists, insert if new)
        SqliteRepo::persist_ettle_tx(&tx, &ettle)?;

        // Emit provenance: applied
        crate::seed::provenance::emit_applied(&tx, &seed_digest, &seed_ettle.id)?;
    }

    // Handle links using core refinement ops (enforces invariants)
    for link in &seed.links {
        // For cross-seed links, we may need to load entities from database into store
        // to make them available to link_child operation

        // Load parent Ettle if not in store (including all its EPs)
        if store.get_ettle(&link.parent).is_err() {
            if let Some(mut ettle) = SqliteRepo::get_ettle(&tx, &link.parent)? {
                // Load all EPs for this parent Ettle so link_child can verify EP is active
                let mut stmt = tx
                    .prepare("SELECT id FROM eps WHERE ettle_id = ?1 AND deleted = 0")
                    .map_err(crate::errors::from_rusqlite)?;
                let ep_rows = stmt
                    .query_map([&link.parent], |row| row.get::<_, String>(0))
                    .map_err(crate::errors::from_rusqlite)?;

                let mut ep_ids = Vec::new();
                for ep_id_result in ep_rows {
                    let ep_id = ep_id_result.map_err(crate::errors::from_rusqlite)?;
                    if let Some(ep) = SqliteRepo::get_ep(&tx, &ep_id)? {
                        ep_ids.push(ep_id.clone());
                        store.insert_ep(ep);
                    }
                }

                // Update ettle's ep_ids list before inserting into store
                ettle.ep_ids = ep_ids;
                store.insert_ettle(ettle);
            }
        }

        // Load parent EP if not in store
        if store.get_ep(&link.parent_ep).is_err() {
            if let Some(ep) = SqliteRepo::get_ep(&tx, &link.parent_ep)? {
                store.insert_ep(ep);
            }
        }

        // Load child Ettle if not in store
        if store.get_ettle(&link.child).is_err() {
            if let Some(ettle) = SqliteRepo::get_ettle(&tx, &link.child)? {
                store.insert_ettle(ettle);
            }
        }

        // Check if link already exists - skip if so (update mode support)
        let parent_ep = store.get_ep(&link.parent_ep)?;
        if parent_ep.child_ettle_id.is_some() {
            // Link already exists - skip it (update mode)
            // Verify it's the same child (safety check)
            if parent_ep.child_ettle_id.as_ref() != Some(&link.child) {
                // Conflicting link - this would be an error, let link_child handle it
            } else {
                // Link already exists and matches - skip
                continue;
            }
        }

        // Use core refinement operation (enforces EpAlreadyHasChild invariant)
        refinement_ops::link_child(&mut store, &link.parent_ep, &link.child)?;

        // Persist updated EP and Ettle
        let ep = store.get_ep(&link.parent_ep)?;
        let child = store.get_ettle(&link.child)?;
        SqliteRepo::persist_ep_tx(&tx, ep)?;
        SqliteRepo::persist_ettle_tx(&tx, child)?;
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
        // Use ep:root:0 which doesn't have a child yet (ep:root:1 is already linked)
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
    parent_ep: ep:root:0
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
                "SELECT child_ettle_id FROM eps WHERE id = 'ep:root:0'",
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

    #[test]
    fn test_import_adds_new_eps_to_existing_ettle() {
        let mut conn = setup_test_db();

        // Import initial seed with only EP 0
        let initial_yaml = r#"
schema_version: 0
project:
  name: update-test
ettles:
  - id: ettle:test
    title: "Test Ettle"
    eps:
      - id: ep:test:0
        ordinal: 0
        normative: true
        why: "EP 0 why"
        what: "EP 0 what"
        how: "EP 0 how"
links: []
"#;
        let temp_dir = std::env::temp_dir();
        let initial_path = temp_dir.join("seed_initial.yaml");
        std::fs::write(&initial_path, initial_yaml).unwrap();

        let initial_result = import_seed(&initial_path, &mut conn);
        assert!(
            initial_result.is_ok(),
            "Initial import should succeed: {:?}",
            initial_result.err()
        );

        // Verify only EP 0 exists
        let ep_count: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM eps WHERE ettle_id = 'ettle:test'",
                [],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(ep_count, 1, "Should have 1 EP initially");

        // Import updated seed with EP 0, 1, and 2
        let updated_yaml = r#"
schema_version: 0
project:
  name: update-test
ettles:
  - id: ettle:test
    title: "Test Ettle"
    eps:
      - id: ep:test:0
        ordinal: 0
        normative: true
        why: "EP 0 why"
        what: "EP 0 what"
        how: "EP 0 how"
      - id: ep:test:1
        ordinal: 1
        normative: true
        why: "EP 1 why"
        what: "EP 1 what"
        how: "EP 1 how"
      - id: ep:test:2
        ordinal: 2
        normative: false
        why: "EP 2 why"
        what: "EP 2 what"
        how: "EP 2 how"
links: []
"#;
        let updated_path = temp_dir.join("seed_updated.yaml");
        std::fs::write(&updated_path, updated_yaml).unwrap();

        let updated_result = import_seed(&updated_path, &mut conn);
        assert!(
            updated_result.is_ok(),
            "Updated import should succeed: {:?}",
            updated_result.err()
        );

        // Verify all 3 EPs now exist
        let ep_count_after: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM eps WHERE ettle_id = 'ettle:test'",
                [],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(ep_count_after, 3, "Should have 3 EPs after update");

        // Verify EP 1 and EP 2 were added
        let ep1_exists: bool = conn
            .query_row("SELECT 1 FROM eps WHERE id = 'ep:test:1'", [], |_| Ok(true))
            .unwrap_or(false);
        assert!(ep1_exists, "EP 1 should exist");

        let ep2_exists: bool = conn
            .query_row("SELECT 1 FROM eps WHERE id = 'ep:test:2'", [], |_| Ok(true))
            .unwrap_or(false);
        assert!(ep2_exists, "EP 2 should exist");

        // Verify EP 0 was not duplicated
        let ep0_count: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM eps WHERE id = 'ep:test:0'",
                [],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(ep0_count, 1, "EP 0 should not be duplicated");

        // Cleanup
        std::fs::remove_file(initial_path).ok();
        std::fs::remove_file(updated_path).ok();
    }

    #[test]
    fn test_import_update_skips_existing_eps() {
        let mut conn = setup_test_db();

        // Import seed with EP 0 and EP 1
        let initial_yaml = r#"
schema_version: 0
project:
  name: skip-test
ettles:
  - id: ettle:skip
    title: "Skip Test"
    eps:
      - id: ep:skip:0
        ordinal: 0
        normative: true
        why: "Original EP 0"
        what: "Original"
        how: "Original"
      - id: ep:skip:1
        ordinal: 1
        normative: true
        why: "Original EP 1"
        what: "Original"
        how: "Original"
links: []
"#;
        let temp_dir = std::env::temp_dir();
        let initial_path = temp_dir.join("seed_skip_initial.yaml");
        std::fs::write(&initial_path, initial_yaml).unwrap();

        import_seed(&initial_path, &mut conn).unwrap();

        // Get original EP 0 content
        let original_ep0_what: String = conn
            .query_row(
                "SELECT content_inline FROM eps WHERE id = 'ep:skip:0'",
                [],
                |row| row.get(0),
            )
            .unwrap();

        // Import updated seed that tries to modify EP 0 and adds EP 2
        let updated_yaml = r#"
schema_version: 0
project:
  name: skip-test
ettles:
  - id: ettle:skip
    title: "Skip Test"
    eps:
      - id: ep:skip:0
        ordinal: 0
        normative: true
        why: "MODIFIED EP 0"
        what: "MODIFIED"
        how: "MODIFIED"
      - id: ep:skip:1
        ordinal: 1
        normative: true
        why: "MODIFIED EP 1"
        what: "MODIFIED"
        how: "MODIFIED"
      - id: ep:skip:2
        ordinal: 2
        normative: true
        why: "New EP 2"
        what: "New"
        how: "New"
links: []
"#;
        let updated_path = temp_dir.join("seed_skip_updated.yaml");
        std::fs::write(&updated_path, updated_yaml).unwrap();

        import_seed(&updated_path, &mut conn).unwrap();

        // Verify EP 0 was NOT modified (existing EPs are immutable)
        let current_ep0_what: String = conn
            .query_row(
                "SELECT content_inline FROM eps WHERE id = 'ep:skip:0'",
                [],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(
            original_ep0_what, current_ep0_what,
            "Existing EP 0 should not be modified"
        );

        // Verify EP 2 was added
        let ep2_exists: bool = conn
            .query_row("SELECT 1 FROM eps WHERE id = 'ep:skip:2'", [], |_| Ok(true))
            .unwrap_or(false);
        assert!(ep2_exists, "EP 2 should be added");

        // Cleanup
        std::fs::remove_file(initial_path).ok();
        std::fs::remove_file(updated_path).ok();
    }

    #[test]
    fn test_import_update_with_links_to_existing_ettle() {
        let mut conn = setup_test_db();

        // Import parent ettle with EP 0
        let parent_yaml = r#"
schema_version: 0
project:
  name: link-test
ettles:
  - id: ettle:parent
    title: "Parent Ettle"
    eps:
      - id: ep:parent:0
        ordinal: 0
        normative: true
        why: "Parent EP 0"
        what: "Parent"
        how: "Parent"
links: []
"#;
        let temp_dir = std::env::temp_dir();
        let parent_path = temp_dir.join("seed_link_parent.yaml");
        std::fs::write(&parent_path, parent_yaml).unwrap();

        import_seed(&parent_path, &mut conn).unwrap();

        // Import child ettle
        let child_yaml = r#"
schema_version: 0
project:
  name: link-test
ettles:
  - id: ettle:child
    title: "Child Ettle"
    eps:
      - id: ep:child:0
        ordinal: 0
        normative: true
        why: "Child EP 0"
        what: "Child"
        how: "Child"
links:
  - parent: ettle:parent
    parent_ep: ep:parent:0
    child: ettle:child
"#;
        let child_path = temp_dir.join("seed_link_child.yaml");
        std::fs::write(&child_path, child_yaml).unwrap();

        import_seed(&child_path, &mut conn).unwrap();

        // Verify link was created
        let ep_child: Option<String> = conn
            .query_row(
                "SELECT child_ettle_id FROM eps WHERE id = 'ep:parent:0'",
                [],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(ep_child, Some("ettle:child".to_string()));

        // Now add EP 1 to parent ettle
        let parent_updated_yaml = r#"
schema_version: 0
project:
  name: link-test
ettles:
  - id: ettle:parent
    title: "Parent Ettle"
    eps:
      - id: ep:parent:0
        ordinal: 0
        normative: true
        why: "Parent EP 0"
        what: "Parent"
        how: "Parent"
      - id: ep:parent:1
        ordinal: 1
        normative: true
        why: "Parent EP 1 NEW"
        what: "New EP"
        how: "New EP"
links: []
"#;
        let parent_updated_path = temp_dir.join("seed_link_parent_updated.yaml");
        std::fs::write(&parent_updated_path, parent_updated_yaml).unwrap();

        let update_result = import_seed(&parent_updated_path, &mut conn);
        assert!(
            update_result.is_ok(),
            "Update should succeed even with existing links: {:?}",
            update_result.err()
        );

        // Verify EP 1 was added
        let ep1_exists: bool = conn
            .query_row("SELECT 1 FROM eps WHERE id = 'ep:parent:1'", [], |_| {
                Ok(true)
            })
            .unwrap_or(false);
        assert!(ep1_exists, "EP 1 should be added");

        // Verify original link is still intact
        let ep_child_after: Option<String> = conn
            .query_row(
                "SELECT child_ettle_id FROM eps WHERE id = 'ep:parent:0'",
                [],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(
            ep_child_after,
            Some("ettle:child".to_string()),
            "Original link should remain intact"
        );

        // Cleanup
        std::fs::remove_file(parent_path).ok();
        std::fs::remove_file(child_path).ok();
        std::fs::remove_file(parent_updated_path).ok();
    }

    #[test]
    fn test_import_fails_when_ep_already_has_child() {
        let mut conn = setup_test_db();

        // Import first seed that creates a link
        let first_path = fixtures_dir().join("seed_full.yaml");
        let first_result = import_seed(&first_path, &mut conn);
        assert!(
            first_result.is_ok(),
            "First seed import should succeed: {:?}",
            first_result.err()
        );

        // Verify the link was created
        let ep_child: Option<String> = conn
            .query_row(
                "SELECT child_ettle_id FROM eps WHERE id = 'ep:root:1'",
                [],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(
            ep_child,
            Some("ettle:store".to_string()),
            "EP should already have a child"
        );

        // Create a second seed that tries to link the SAME EP to a different child
        let conflicting_yaml = r#"
schema_version: 0
project:
  name: conflict-test
ettles:
  - id: ettle:other_child
    title: "Other Child"
    eps:
      - id: ep:other:0
        ordinal: 0
        normative: true
        why: "Other why"
        what: "Other what"
        how: "Other how"
links:
  - parent: ettle:root
    parent_ep: ep:root:1
    child: ettle:other_child
"#;

        // Write conflicting seed to temp file
        let temp_dir = std::env::temp_dir();
        let conflict_path = temp_dir.join("seed_conflict_test.yaml");
        std::fs::write(&conflict_path, conflicting_yaml).unwrap();

        // Import should fail with EpAlreadyHasChild error
        let conflict_result = import_seed(&conflict_path, &mut conn);
        assert!(
            conflict_result.is_err(),
            "Import should fail when EP already has a child"
        );

        let err = conflict_result.unwrap_err();
        let err_msg = format!("{:?}", err);
        assert!(
            err_msg.contains("DuplicateMapping") || err_msg.contains("already maps"),
            "Error should indicate EP already has a child, got: {}",
            err_msg
        );

        // Verify the original link is unchanged (rollback)
        let ep_child_after: Option<String> = conn
            .query_row(
                "SELECT child_ettle_id FROM eps WHERE id = 'ep:root:1'",
                [],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(
            ep_child_after,
            Some("ettle:store".to_string()),
            "Original link should be unchanged after failed import"
        );

        // Verify the conflicting ettle was not created (rollback)
        let other_child_exists: bool = conn
            .query_row(
                "SELECT 1 FROM ettles WHERE id = 'ettle:other_child'",
                [],
                |_| Ok(true),
            )
            .unwrap_or(false);
        assert!(
            !other_child_exists,
            "Conflicting ettle should not exist after rollback"
        );

        // Cleanup
        std::fs::remove_file(conflict_path).ok();
    }
}
