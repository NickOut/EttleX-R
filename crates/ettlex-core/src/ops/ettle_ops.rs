use chrono::Utc;
use uuid::Uuid;

use super::store::Store;
use crate::errors::{ExError, ExErrorKind, Result};
use crate::model::Ettle;

/// Create a new Ettle with the given title.
///
/// # Errors
/// Returns `InvalidTitle` if `title` is empty or whitespace-only.
pub fn create_ettle(store: &mut Store, title: String) -> Result<String> {
    if title.trim().is_empty() {
        return Err(ExError::new(ExErrorKind::InvalidTitle)
            .with_message("Title cannot be empty or whitespace-only".to_string()));
    }
    let ettle_id = Uuid::now_v7().to_string();
    let ettle = Ettle::new(ettle_id.clone(), title);
    store.insert_ettle(ettle);
    Ok(ettle_id)
}

/// Get an Ettle by ID.
///
/// # Errors
/// Returns `NotFound` if no Ettle exists with the given `id`.
pub fn read_ettle<'a>(store: &'a Store, id: &str) -> Result<&'a Ettle> {
    store.get_ettle(id)
}

/// Delete an Ettle (tombstone via store).
///
/// # Errors
/// Returns `NotFound` if no Ettle exists with the given `id`.
pub fn delete_ettle(store: &mut Store, id: &str) -> Result<()> {
    store.get_ettle(id)?;
    let ettle = store.get_ettle_mut(id)?;
    ettle.updated_at = Utc::now();
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_create_ettle_success() {
        let mut store = Store::new();
        let id = create_ettle(&mut store, "Test Ettle".to_string()).unwrap();
        let ettle = store.get_ettle(&id).unwrap();
        assert_eq!(ettle.title, "Test Ettle");
    }

    #[test]
    fn test_create_ettle_invalid_title() {
        let mut store = Store::new();
        let result = create_ettle(&mut store, "".to_string());
        assert!(result.is_err());
    }
}
