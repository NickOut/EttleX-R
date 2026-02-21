use chrono::Utc;
use uuid::Uuid;

use super::{active_eps, store::Store};
use crate::errors::{EttleXError, Result};
use crate::model::{Ep, Ettle, Metadata};

/// Create a new Ettle with the given title and optional EP0 content
///
/// Automatically generates a UUID v7 for the Ettle ID and creates EP0
/// (ordinal 0, normative) as the initial partition.
///
/// # Arguments
/// * `store` - Mutable reference to the Store
/// * `title` - Title for the new Ettle (must not be empty or whitespace-only)
/// * `metadata` - Optional metadata for the Ettle
/// * `why` - Optional WHY content for EP0
/// * `what` - Optional WHAT content for EP0 (cannot be empty string)
/// * `how` - Optional HOW content for EP0 (cannot be empty string)
///
/// # Returns
/// The ID of the newly created Ettle
///
/// # Errors
/// * `InvalidTitle` - If title is empty or contains only whitespace
/// * `InvalidWhat` - If what is provided as empty string
/// * `InvalidHow` - If how is provided as empty string
pub fn create_ettle(
    store: &mut Store,
    title: String,
    metadata: Option<Metadata>,
    why: Option<String>,
    what: Option<String>,
    how: Option<String>,
) -> Result<String> {
    // Validate title
    if title.trim().is_empty() {
        return Err(EttleXError::InvalidTitle {
            reason: "Title cannot be empty or whitespace-only".to_string(),
        });
    }

    // Generate UUID v7 for deterministic time-ordered IDs
    let ettle_id = Uuid::now_v7().to_string();
    let ep0_id = Uuid::now_v7().to_string();

    // Validate WHAT content (cannot be empty string)
    let what_content = if let Some(w) = what {
        if w.is_empty() {
            return Err(EttleXError::InvalidWhat {
                ep_id: ep0_id.clone(),
            });
        }
        w
    } else {
        String::new()
    };

    // Validate HOW content (cannot be empty string)
    let how_content = if let Some(h) = how {
        if h.is_empty() {
            return Err(EttleXError::InvalidHow {
                ep_id: ep0_id.clone(),
            });
        }
        h
    } else {
        String::new()
    };

    // Create the Ettle with optional metadata
    let mut ettle = Ettle::new(ettle_id.clone(), title);
    if let Some(m) = metadata {
        ettle.metadata = m;
    }

    // Create EP0 (initial partition) with provided content
    let ep0 = Ep::new(
        ep0_id.clone(),
        ettle_id.clone(),
        0,                       // ordinal 0
        true,                    // normative
        why.unwrap_or_default(), // WHY content
        what_content,            // WHAT content (validated)
        how_content,             // HOW content (validated)
    );

    // Insert EP0 first
    store.insert_ep(ep0);

    // Update Ettle with EP0 ID
    ettle.add_ep_id(ep0_id);

    // Insert Ettle
    store.insert_ettle(ettle);

    Ok(ettle_id)
}

/// Read an Ettle by ID
///
/// # Arguments
/// * `store` - Reference to the Store
/// * `id` - The Ettle ID to read
///
/// # Returns
/// Reference to the Ettle
///
/// # Errors
/// * `EttleNotFound` - If Ettle doesn't exist
/// * `EttleDeleted` - If Ettle was previously deleted
pub fn read_ettle<'a>(store: &'a Store, id: &str) -> Result<&'a Ettle> {
    store.get_ettle(id)
}

/// Update an Ettle's title and/or metadata
///
/// Updates the `updated_at` timestamp. If both title and metadata are None,
/// this is a no-op (but still updates the timestamp).
///
/// # Arguments
/// * `store` - Mutable reference to the Store
/// * `id` - The Ettle ID to update
/// * `title` - Optional new title (if Some, must not be empty/whitespace)
/// * `metadata` - Optional new metadata (completely replaces existing metadata)
///
/// # Errors
/// * `EttleNotFound` - If Ettle doesn't exist
/// * `EttleDeleted` - If Ettle was previously deleted
/// * `InvalidTitle` - If title is provided but is empty or whitespace-only
pub fn update_ettle(
    store: &mut Store,
    id: &str,
    title: Option<String>,
    metadata: Option<Metadata>,
) -> Result<()> {
    // Validate title if provided
    if let Some(ref t) = title {
        if t.trim().is_empty() {
            return Err(EttleXError::InvalidTitle {
                reason: "Title cannot be empty or whitespace-only".to_string(),
            });
        }
    }

    // Get mutable reference to Ettle
    let ettle = store.get_ettle_mut(id)?;

    // Update fields
    if let Some(new_title) = title {
        ettle.title = new_title;
    }

    if let Some(new_metadata) = metadata {
        ettle.metadata = new_metadata;
    }

    // Update timestamp
    ettle.updated_at = Utc::now();

    Ok(())
}

/// Delete an Ettle (tombstone deletion)
///
/// Sets the `deleted` flag to true. The Ettle remains in storage but is
/// filtered from queries.
///
/// Only checks active (non-deleted) EPs for children.
///
/// # Arguments
/// * `store` - Mutable reference to the Store
/// * `id` - The Ettle ID to delete
///
/// # Errors
/// * `EttleNotFound` - If Ettle doesn't exist
/// * `EttleDeleted` - If Ettle was already deleted
/// * `DeleteWithChildren` - If Ettle has any children (checks active EPs only)
pub fn delete_ettle(store: &mut Store, id: &str) -> Result<()> {
    // Check if Ettle exists and is not deleted
    let ettle = store.get_ettle(id)?;

    // Check if Ettle has any children (check only active EPs)
    let active = active_eps(store, ettle)?;
    let children_count = active
        .iter()
        .filter(|ep| ep.child_ettle_id.is_some())
        .count();

    if children_count > 0 {
        return Err(EttleXError::DeleteWithChildren {
            ettle_id: id.to_string(),
            child_count: children_count,
        });
    }

    // Get mutable reference and set deleted flag
    let ettle = store.get_ettle_mut(id)?;
    ettle.deleted = true;
    ettle.updated_at = Utc::now();

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_create_ettle_success() {
        let mut store = Store::new();
        let id =
            create_ettle(&mut store, "Test Ettle".to_string(), None, None, None, None).unwrap();

        let ettle = store.get_ettle(&id).unwrap();
        assert_eq!(ettle.title, "Test Ettle");
        assert!(ettle.is_root());
        assert_eq!(ettle.ep_ids.len(), 1);
    }

    #[test]
    fn test_create_ettle_with_ep0_content() {
        let mut store = Store::new();
        let id = create_ettle(
            &mut store,
            "Test Ettle".to_string(),
            None,
            Some("Why text".to_string()),
            Some("What text".to_string()),
            Some("How text".to_string()),
        )
        .unwrap();

        let ettle = store.get_ettle(&id).unwrap();
        assert_eq!(ettle.ep_ids.len(), 1);

        // Verify EP0 content
        let ep0 = store.get_ep(&ettle.ep_ids[0]).unwrap();
        assert_eq!(ep0.ordinal, 0);
        assert_eq!(ep0.why, "Why text");
        assert_eq!(ep0.what, "What text");
        assert_eq!(ep0.how, "How text");
    }

    #[test]
    fn test_create_ettle_invalid_title() {
        let mut store = Store::new();
        let result = create_ettle(&mut store, "".to_string(), None, None, None, None);

        assert!(result.is_err());
    }

    #[test]
    fn test_create_ettle_invalid_what() {
        let mut store = Store::new();
        let result = create_ettle(
            &mut store,
            "Test".to_string(),
            None,
            None,
            Some("".to_string()), // Empty WHAT
            None,
        );

        assert!(result.is_err());
        assert!(matches!(result, Err(EttleXError::InvalidWhat { .. })));
    }

    #[test]
    fn test_create_ettle_invalid_how() {
        let mut store = Store::new();
        let result = create_ettle(
            &mut store,
            "Test".to_string(),
            None,
            None,
            None,
            Some("".to_string()), // Empty HOW
        );

        assert!(result.is_err());
        assert!(matches!(result, Err(EttleXError::InvalidHow { .. })));
    }
}
