//! Hydration layer - loads domain models from SQLite into Store
//!
//! Converts database rows back into Ettle structs with deterministic ordering.
//! EP loading has been retired in Slice 03.

#![allow(clippy::result_large_err)]

use crate::errors::{from_rusqlite, Result};
use crate::repo::SqliteRepo;
use ettlex_core::model::Ettle;
use ettlex_core::ops::store::Store;
use rusqlite::Connection;

/// Load all Ettles from the database into the Store.
///
/// Returns Ettles in deterministic order (sorted by ID).
pub fn load_all_ettles(conn: &Connection, store: &mut Store) -> Result<()> {
    let mut stmt = conn
        .prepare("SELECT id, title, created_at, updated_at FROM ettles ORDER BY id")
        .map_err(from_rusqlite)?;

    let ettles: Vec<Ettle> = stmt
        .query_map([], |row| {
            let id: String = row.get(0)?;
            let title: String = row.get(1)?;
            let created_at_str: String = row.get(2)?;
            let updated_at_str: String = row.get(3)?;

            let mut ettle = Ettle::new(id, title);
            if let Ok(ts) = chrono::DateTime::parse_from_rfc3339(&created_at_str) {
                ettle.created_at = ts.with_timezone(&chrono::Utc);
            }
            if let Ok(ts) = chrono::DateTime::parse_from_rfc3339(&updated_at_str) {
                ettle.updated_at = ts.with_timezone(&chrono::Utc);
            }

            Ok(ettle)
        })
        .map_err(from_rusqlite)?
        .collect::<std::result::Result<Vec<_>, _>>()
        .map_err(from_rusqlite)?;

    for ettle in ettles {
        store.insert_ettle(ettle);
    }

    Ok(())
}

/// Load the complete tree from the database.
///
/// Loads all Ettles, Decisions, and Decision Links in deterministic order.
/// EP loading has been retired in Slice 03.
pub fn load_tree(conn: &Connection) -> Result<Store> {
    let mut store = Store::new();

    // Load all Ettles (deterministic order: sorted by ID)
    load_all_ettles(conn, &mut store)?;

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
    fn test_load_tree_empty() {
        let conn = setup_test_db();
        let store = load_tree(&conn).unwrap();
        assert_eq!(store.list_ettles().len(), 0);
    }
}
