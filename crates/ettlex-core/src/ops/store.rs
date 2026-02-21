use std::collections::HashMap;

use crate::errors::{EttleXError, Result};
use crate::model::{Ep, Ettle};

/// In-memory store for Ettles and EPs
///
/// This is a simple HashMap-based storage implementation for Phase 0.5.
/// Not thread-safe (no Arc/RwLock) - designed for single-threaded use.
/// All storage access is encapsulated here for easy refactoring in Phase 1.
#[derive(Debug, Clone, Default)]
pub struct Store {
    /// Map of Ettle ID to Ettle
    pub(crate) ettles: HashMap<String, Ettle>,
    /// Map of EP ID to EP
    pub(crate) eps: HashMap<String, Ep>,
}

impl Store {
    /// Create a new empty Store
    pub fn new() -> Self {
        Self {
            ettles: HashMap::new(),
            eps: HashMap::new(),
        }
    }

    /// Get an Ettle by ID
    ///
    /// Returns the Ettle if found and not deleted, otherwise returns an error.
    ///
    /// # Errors
    ///
    /// Returns `EttleNotFound` if the ettle doesn't exist, or `EttleDeleted` if it was tombstoned.
    pub fn get_ettle(&self, id: &str) -> Result<&Ettle> {
        let ettle = self
            .ettles
            .get(id)
            .ok_or_else(|| EttleXError::EttleNotFound {
                ettle_id: id.to_string(),
            })?;

        if ettle.deleted {
            return Err(EttleXError::EttleDeleted {
                ettle_id: id.to_string(),
            });
        }

        Ok(ettle)
    }

    /// Get a mutable reference to an Ettle by ID
    ///
    /// Returns the Ettle if found and not deleted, otherwise returns an error.
    /// This is a public method to enable test helpers.
    ///
    /// # Errors
    ///
    /// Returns `EttleNotFound` if the ettle doesn't exist, or `EttleDeleted` if it was tombstoned.
    pub fn get_ettle_mut(&mut self, id: &str) -> Result<&mut Ettle> {
        let ettle = self
            .ettles
            .get_mut(id)
            .ok_or_else(|| EttleXError::EttleNotFound {
                ettle_id: id.to_string(),
            })?;

        if ettle.deleted {
            return Err(EttleXError::EttleDeleted {
                ettle_id: id.to_string(),
            });
        }

        Ok(ettle)
    }

    /// Get an EP by ID
    ///
    /// Returns the EP if found and not deleted, otherwise returns an error.
    ///
    /// # Errors
    ///
    /// Returns `EpNotFound` if the EP doesn't exist, or `EpDeleted` if it was tombstoned.
    pub fn get_ep(&self, id: &str) -> Result<&Ep> {
        let ep = self.eps.get(id).ok_or_else(|| EttleXError::EpNotFound {
            ep_id: id.to_string(),
        })?;

        if ep.deleted {
            return Err(EttleXError::EpDeleted {
                ep_id: id.to_string(),
            });
        }

        Ok(ep)
    }

    /// Get a mutable reference to an EP by ID
    ///
    /// Returns the EP if found and not deleted, otherwise returns an error.
    /// This is a public method to enable test helpers.
    ///
    /// # Errors
    ///
    /// Returns `EpNotFound` if the EP doesn't exist, or `EpDeleted` if it was tombstoned.
    pub fn get_ep_mut(&mut self, id: &str) -> Result<&mut Ep> {
        let ep = self
            .eps
            .get_mut(id)
            .ok_or_else(|| EttleXError::EpNotFound {
                ep_id: id.to_string(),
            })?;

        if ep.deleted {
            return Err(EttleXError::EpDeleted {
                ep_id: id.to_string(),
            });
        }

        Ok(ep)
    }

    /// List all non-deleted Ettles
    pub fn list_ettles(&self) -> Vec<&Ettle> {
        self.ettles.values().filter(|e| !e.deleted).collect()
    }

    /// List all non-deleted EPs
    pub fn list_eps(&self) -> Vec<&Ep> {
        self.eps.values().filter(|ep| !ep.deleted).collect()
    }

    /// Insert an Ettle into the store
    ///
    /// This is an internal method used by CRUD operations and test helpers.
    pub fn insert_ettle(&mut self, ettle: Ettle) {
        self.ettles.insert(ettle.id.clone(), ettle);
    }

    /// Insert an EP into the store
    ///
    /// This is an internal method used by CRUD operations and test helpers.
    pub fn insert_ep(&mut self, ep: Ep) {
        self.eps.insert(ep.id.clone(), ep);
    }

    /// Check if an Ettle exists (ignoring deleted flag)
    #[allow(dead_code)]
    pub(crate) fn ettle_exists(&self, id: &str) -> bool {
        self.ettles.contains_key(id)
    }

    /// Check if an EP exists (ignoring deleted flag)
    #[allow(dead_code)]
    pub(crate) fn ep_exists(&self, id: &str) -> bool {
        self.eps.contains_key(id)
    }

    /// Check if an EP exists in storage (including deleted EPs)
    ///
    /// This is useful for testing hard delete vs tombstone behavior.
    pub fn ep_exists_in_storage(&self, id: &str) -> bool {
        self.eps.contains_key(id)
    }

    /// Get an EP from storage, bypassing deleted check
    ///
    /// This is useful for testing tombstone behavior.
    /// Returns None if EP doesn't exist, Some(EP) if it exists (even if deleted).
    pub fn get_ep_raw(&self, id: &str) -> Option<&Ep> {
        self.eps.get(id)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_new_store() {
        let store = Store::new();
        assert_eq!(store.list_ettles().len(), 0);
        assert_eq!(store.list_eps().len(), 0);
    }

    #[test]
    fn test_insert_and_get_ettle() {
        let mut store = Store::new();
        let ettle = Ettle::new("ettle-1".to_string(), "Test".to_string());

        store.insert_ettle(ettle.clone());

        let retrieved = store.get_ettle("ettle-1").unwrap();
        assert_eq!(retrieved.id, "ettle-1");
        assert_eq!(retrieved.title, "Test");
    }

    #[test]
    fn test_get_nonexistent_ettle() {
        let store = Store::new();
        let result = store.get_ettle("nonexistent");
        assert!(result.is_err());
        assert!(matches!(result, Err(EttleXError::EttleNotFound { .. })));
    }

    #[test]
    fn test_get_deleted_ettle() {
        let mut store = Store::new();
        let mut ettle = Ettle::new("ettle-1".to_string(), "Test".to_string());
        ettle.deleted = true;

        store.insert_ettle(ettle);

        let result = store.get_ettle("ettle-1");
        assert!(result.is_err());
        assert!(matches!(result, Err(EttleXError::EttleDeleted { .. })));
    }
}
