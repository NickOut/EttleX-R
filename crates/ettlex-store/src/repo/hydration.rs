//! Hydration layer - loads domain models from SQLite into Store
//!
//! Converts database rows back into Ettle/EP structs with deterministic ordering

#![allow(clippy::result_large_err)]

use crate::errors::{from_rusqlite, Result};
use crate::repo::SqliteRepo;
use ettlex_core::model::{Ep, Ettle};
use ettlex_core::ops::store::Store;
use rusqlite::Connection;
use std::collections::BTreeMap;

/// Load a single Ettle from the database into the Store
pub fn load_ettle(conn: &Connection, ettle_id: &str, store: &mut Store) -> Result<()> {
    let mut stmt = conn
        .prepare("SELECT id, title, parent_id, deleted, created_at, updated_at, metadata FROM ettles WHERE id = ?")
        .map_err(from_rusqlite)?;

    let ettle = stmt
        .query_row([ettle_id], |row| {
            let id: String = row.get(0)?;
            let title: String = row.get(1)?;
            let parent_id: Option<String> = row.get(2)?;
            let deleted: i32 = row.get(3)?;
            let created_at: i64 = row.get(4)?;
            let updated_at: i64 = row.get(5)?;
            let metadata_json: String = row.get(6)?;

            let mut ettle = Ettle::new(id, title);
            ettle.parent_id = parent_id;
            ettle.deleted = deleted != 0;
            ettle.created_at =
                chrono::DateTime::from_timestamp(created_at, 0).unwrap_or_else(chrono::Utc::now);
            ettle.updated_at =
                chrono::DateTime::from_timestamp(updated_at, 0).unwrap_or_else(chrono::Utc::now);
            ettle.metadata = serde_json::from_str(&metadata_json).unwrap_or_default();

            Ok(ettle)
        })
        .map_err(from_rusqlite)?;

    // Load associated EPs
    let mut stmt = conn
        .prepare("SELECT id FROM eps WHERE ettle_id = ? ORDER BY ordinal")
        .map_err(from_rusqlite)?;

    let ep_ids: Vec<String> = stmt
        .query_map([&ettle.id], |row| row.get(0))
        .map_err(from_rusqlite)?
        .collect::<std::result::Result<Vec<_>, _>>()
        .map_err(from_rusqlite)?;

    // Reconstruct ep_ids list
    let mut loaded_ettle = ettle;
    for ep_id in ep_ids {
        loaded_ettle.add_ep_id(ep_id);
    }

    store.insert_ettle(loaded_ettle);

    Ok(())
}

/// Load all Ettles from the database into the Store
///
/// Returns Ettles in deterministic order (sorted by ID)
pub fn load_all_ettles(conn: &Connection, store: &mut Store) -> Result<()> {
    let mut stmt = conn
        .prepare("SELECT id, title, parent_id, deleted, created_at, updated_at, metadata FROM ettles ORDER BY id")
        .map_err(from_rusqlite)?;

    let ettles: Vec<Ettle> = stmt
        .query_map([], |row| {
            let id: String = row.get(0)?;
            let title: String = row.get(1)?;
            let parent_id: Option<String> = row.get(2)?;
            let deleted: i32 = row.get(3)?;
            let created_at: i64 = row.get(4)?;
            let updated_at: i64 = row.get(5)?;
            let metadata_json: String = row.get(6)?;

            let mut ettle = Ettle::new(id, title);
            ettle.parent_id = parent_id;
            ettle.deleted = deleted != 0;
            ettle.created_at =
                chrono::DateTime::from_timestamp(created_at, 0).unwrap_or_else(chrono::Utc::now);
            ettle.updated_at =
                chrono::DateTime::from_timestamp(updated_at, 0).unwrap_or_else(chrono::Utc::now);
            ettle.metadata = serde_json::from_str(&metadata_json).unwrap_or_default();

            Ok(ettle)
        })
        .map_err(from_rusqlite)?
        .collect::<std::result::Result<Vec<_>, _>>()
        .map_err(from_rusqlite)?;

    // Load into Store
    for ettle in ettles {
        store.insert_ettle(ettle);
    }

    Ok(())
}

/// Load a single EP from the database into the Store
pub fn load_ep(conn: &Connection, ep_id: &str, store: &mut Store) -> Result<()> {
    let mut stmt = conn
        .prepare("SELECT id, ettle_id, ordinal, normative, child_ettle_id, content_inline, deleted, created_at, updated_at FROM eps WHERE id = ?")
        .map_err(from_rusqlite)?;

    let ep = stmt
        .query_row([ep_id], |row| {
            let id: String = row.get(0)?;
            let ettle_id: String = row.get(1)?;
            let ordinal: u32 = row.get(2)?;
            let normative: i32 = row.get(3)?;
            let child_ettle_id: Option<String> = row.get(4)?;
            let content_inline: String = row.get(5)?;
            let deleted: i32 = row.get(6)?;
            let created_at: i64 = row.get(7)?;
            let updated_at: i64 = row.get(8)?;

            // Parse content
            let content: serde_json::Value =
                serde_json::from_str(&content_inline).unwrap_or_default();
            let why = content["why"].as_str().unwrap_or_default().to_string();
            let what = content["what"].as_str().unwrap_or_default().to_string();
            let how = content["how"].as_str().unwrap_or_default().to_string();

            let mut ep = Ep::new(id, ettle_id, ordinal, normative != 0, why, what, how);
            ep.child_ettle_id = child_ettle_id;
            ep.deleted = deleted != 0;
            ep.created_at =
                chrono::DateTime::from_timestamp(created_at, 0).unwrap_or_else(chrono::Utc::now);
            ep.updated_at =
                chrono::DateTime::from_timestamp(updated_at, 0).unwrap_or_else(chrono::Utc::now);

            Ok(ep)
        })
        .map_err(from_rusqlite)?;

    store.insert_ep(ep);

    Ok(())
}

/// Load all EPs from the database into the Store
///
/// Returns EPs in deterministic order (sorted by ettle_id, then ordinal)
pub fn load_all_eps(conn: &Connection, store: &mut Store) -> Result<()> {
    let mut stmt = conn
        .prepare("SELECT id, ettle_id, ordinal, normative, child_ettle_id, content_inline, deleted, created_at, updated_at FROM eps ORDER BY ettle_id, ordinal")
        .map_err(from_rusqlite)?;

    let eps: Vec<Ep> = stmt
        .query_map([], |row| {
            let id: String = row.get(0)?;
            let ettle_id: String = row.get(1)?;
            let ordinal: u32 = row.get(2)?;
            let normative: i32 = row.get(3)?;
            let child_ettle_id: Option<String> = row.get(4)?;
            let content_inline: String = row.get(5)?;
            let deleted: i32 = row.get(6)?;
            let created_at: i64 = row.get(7)?;
            let updated_at: i64 = row.get(8)?;

            // Parse content
            let content: serde_json::Value =
                serde_json::from_str(&content_inline).unwrap_or_default();
            let why = content["why"].as_str().unwrap_or_default().to_string();
            let what = content["what"].as_str().unwrap_or_default().to_string();
            let how = content["how"].as_str().unwrap_or_default().to_string();

            let mut ep = Ep::new(id, ettle_id, ordinal, normative != 0, why, what, how);
            ep.child_ettle_id = child_ettle_id;
            ep.deleted = deleted != 0;
            ep.created_at =
                chrono::DateTime::from_timestamp(created_at, 0).unwrap_or_else(chrono::Utc::now);
            ep.updated_at =
                chrono::DateTime::from_timestamp(updated_at, 0).unwrap_or_else(chrono::Utc::now);

            Ok(ep)
        })
        .map_err(from_rusqlite)?
        .collect::<std::result::Result<Vec<_>, _>>()
        .map_err(from_rusqlite)?;

    // Load into Store
    for ep in eps {
        store.insert_ep(ep);
    }

    Ok(())
}

/// Load the complete tree from the database
///
/// Loads all Ettles, EPs, Constraints, and their relationships in deterministic order
pub fn load_tree(conn: &Connection) -> Result<Store> {
    let mut store = Store::new();

    // Load all Ettles (deterministic order: sorted by ID)
    load_all_ettles(conn, &mut store)?;

    // Load all EPs (deterministic order: sorted by ettle_id, ordinal)
    load_all_eps(conn, &mut store)?;

    // Load all Constraints (deterministic order: sorted by constraint_id)
    let constraints = SqliteRepo::list_constraints(conn)?;
    for constraint in constraints {
        store.insert_constraint(constraint);
    }

    // Load all EP-Constraint attachment records (deterministic order: sorted by ep_id, ordinal)
    let ep_constraint_refs = SqliteRepo::list_all_ep_constraint_refs(conn)?;
    for ref_record in ep_constraint_refs {
        store.insert_ep_constraint_ref(ref_record);
    }

    // Load all Decisions (deterministic order: sorted by created_at, decision_id)
    let decisions = SqliteRepo::list_decisions(conn)?;
    for decision in decisions {
        store.insert_decision(decision);
    }

    // Load all Decision Evidence Items (deterministic order: sorted by evidence_capture_id)
    let evidence_items = SqliteRepo::list_all_evidence_items(conn)?;
    for item in evidence_items {
        store.insert_evidence_item(item);
    }

    // Load all Decision Links (deterministic order: sorted by decision_id, target_kind, target_id, relation_kind)
    let decision_links = SqliteRepo::list_all_decision_links(conn)?;
    for link in decision_links {
        store.insert_decision_link(link);
    }

    // Reconstruct ep_ids lists for each Ettle
    // Group EPs by ettle_id using BTreeMap for deterministic iteration
    let mut ettle_eps: BTreeMap<String, Vec<String>> = BTreeMap::new();

    for ep in store.list_eps() {
        ettle_eps
            .entry(ep.ettle_id.clone())
            .or_default()
            .push(ep.id.clone());
    }

    // Update each Ettle's ep_ids list
    for (ettle_id, ep_ids) in ettle_eps {
        if let Ok(ettle) = store.get_ettle_mut(&ettle_id) {
            ettle.ep_ids.clear();
            for ep_id in ep_ids {
                ettle.add_ep_id(ep_id);
            }
        }
    }

    Ok(store)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::migrations;

    fn setup_test_db() -> Connection {
        let mut conn = Connection::open_in_memory().unwrap();
        migrations::apply_migrations(&mut conn).unwrap();
        conn
    }

    #[test]
    fn test_load_ettle() {
        let conn = setup_test_db();
        let ettle = Ettle::new("test-1".to_string(), "Test".to_string());
        crate::repo::SqliteRepo::persist_ettle(&conn, &ettle).unwrap();

        let mut store = Store::new();
        load_ettle(&conn, "test-1", &mut store).unwrap();

        let loaded = store.get_ettle("test-1").unwrap();
        assert_eq!(loaded.id, "test-1");
        assert_eq!(loaded.title, "Test");
    }

    #[test]
    fn test_load_tree_empty() {
        let conn = setup_test_db();
        let store = load_tree(&conn).unwrap();
        assert_eq!(store.list_ettles().len(), 0);
        assert_eq!(store.list_eps().len(), 0);
    }
}
