use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Metadata wrapper for extensible key-value storage
///
/// Stores arbitrary metadata as JSON values, allowing for flexible
/// extension without schema changes.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
pub struct Metadata {
    data: HashMap<String, serde_json::Value>,
}

impl Metadata {
    /// Create a new empty Metadata instance
    pub fn new() -> Self {
        Self {
            data: HashMap::new(),
        }
    }

    /// Get a value by key
    pub fn get(&self, key: &str) -> Option<&serde_json::Value> {
        self.data.get(key)
    }

    /// Set a value by key
    pub fn set(&mut self, key: String, value: serde_json::Value) {
        self.data.insert(key, value);
    }

    /// Remove a value by key
    pub fn remove(&mut self, key: &str) -> Option<serde_json::Value> {
        self.data.remove(key)
    }

    /// Check if a key exists
    pub fn contains_key(&self, key: &str) -> bool {
        self.data.contains_key(key)
    }

    /// Get all keys
    pub fn keys(&self) -> impl Iterator<Item = &String> {
        self.data.keys()
    }

    /// Get the number of metadata entries
    pub fn len(&self) -> usize {
        self.data.len()
    }

    /// Check if metadata is empty
    pub fn is_empty(&self) -> bool {
        self.data.is_empty()
    }
}

impl From<HashMap<String, serde_json::Value>> for Metadata {
    fn from(data: HashMap<String, serde_json::Value>) -> Self {
        Self { data }
    }
}

impl From<Metadata> for HashMap<String, serde_json::Value> {
    fn from(metadata: Metadata) -> Self {
        metadata.data
    }
}
