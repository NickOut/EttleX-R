use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

/// Ettle — the fundamental unit of architectural structure.
///
/// An Ettle represents a concept, component, or decision point in the
/// architecture. The EP construct has been retired (Slice 03); Ettles
/// are now first-class nodes connected directly via the `relations` table.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Ettle {
    /// Unique identifier for this Ettle (UUID v7)
    pub id: String,

    /// Human-readable title
    pub title: String,

    /// Timestamp when this Ettle was created
    pub created_at: DateTime<Utc>,

    /// Timestamp when this Ettle was last updated
    pub updated_at: DateTime<Utc>,
}

impl Ettle {
    /// Create a new Ettle with the given ID and title.
    pub fn new(id: String, title: String) -> Self {
        let now = Utc::now();
        Self {
            id,
            title,
            created_at: now,
            updated_at: now,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_new_ettle() {
        let ettle = Ettle::new("ettle-1".to_string(), "Test Ettle".to_string());
        assert_eq!(ettle.id, "ettle-1");
        assert_eq!(ettle.title, "Test Ettle");
    }
}
