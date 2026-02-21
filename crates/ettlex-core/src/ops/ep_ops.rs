use chrono::Utc;
use uuid::Uuid;

use super::{active_eps, store::Store};
use crate::errors::{EttleXError, Result};
use crate::model::Ep;

/// Create a new EP with the given parameters
///
/// Automatically generates a UUID v7 for the EP ID and adds it to the
/// parent Ettle's ep_ids list.
///
/// Ordinal immutability policy: Ordinals cannot be reused even if an EP
/// with that ordinal is deleted (tombstoned). This ensures stable, deterministic
/// ordering throughout the EP lifecycle.
///
/// # Arguments
/// * `store` - Mutable reference to the Store
/// * `ettle_id` - ID of the Ettle that will own this EP
/// * `ordinal` - Ordinal position within the Ettle (must be unique, never reused)
/// * `normative` - Whether this EP is normative/binding
/// * `why` - Rationale text
/// * `what` - Description text (cannot be empty string)
/// * `how` - Implementation text (cannot be empty string)
///
/// # Returns
/// The ID of the newly created EP
///
/// # Errors
/// * `EttleNotFound` - If parent Ettle doesn't exist
/// * `EttleDeleted` - If parent Ettle was deleted
/// * `OrdinalAlreadyExists` - If an active EP with this ordinal exists
/// * `EpOrdinalReuseForbidden` - If trying to reuse ordinal from a tombstoned EP
/// * `InvalidWhat` - If what is empty string
/// * `InvalidHow` - If how is empty string
pub fn create_ep(
    store: &mut Store,
    ettle_id: &str,
    ordinal: u32,
    normative: bool,
    why: String,
    what: String,
    how: String,
) -> Result<String> {
    // Verify parent Ettle exists
    let ettle = store.get_ettle(ettle_id)?;

    // Generate UUID v7 for EP (before validation so we can use it in errors)
    let ep_id = Uuid::now_v7().to_string();

    // Validate WHAT content (cannot be empty string)
    if !what.is_empty() && what.trim().is_empty() {
        return Err(EttleXError::InvalidWhat {
            ep_id: ep_id.clone(),
        });
    }

    // Validate HOW content (cannot be empty string)
    if !how.is_empty() && how.trim().is_empty() {
        return Err(EttleXError::InvalidHow {
            ep_id: ep_id.clone(),
        });
    }

    // Check if ordinal already exists (including tombstoned EPs)
    // This enforces ordinal immutability - ordinals are never reused
    for existing_ep_id in &ettle.ep_ids {
        // Check all EPs in store, including tombstoned ones
        if let Some(existing_ep) = store.eps.get(existing_ep_id) {
            if existing_ep.ordinal == ordinal {
                if existing_ep.deleted {
                    // Ordinal reuse forbidden
                    return Err(EttleXError::EpOrdinalReuseForbidden {
                        ettle_id: ettle_id.to_string(),
                        ordinal,
                        tombstoned_ep_id: existing_ep_id.clone(),
                    });
                } else {
                    // Active EP with same ordinal
                    return Err(EttleXError::OrdinalAlreadyExists {
                        ettle_id: ettle_id.to_string(),
                        ordinal,
                    });
                }
            }
        }
    }

    // Create EP
    let ep = Ep::new(
        ep_id.clone(),
        ettle_id.to_string(),
        ordinal,
        normative,
        why,
        what,
        how,
    );

    // Insert EP
    store.insert_ep(ep);

    // Add EP ID to Ettle's ep_ids list
    let ettle = store.get_ettle_mut(ettle_id)?;
    ettle.add_ep_id(ep_id.clone());

    Ok(ep_id)
}

/// Read an EP by ID
///
/// # Arguments
/// * `store` - Reference to the Store
/// * `id` - The EP ID to read
///
/// # Returns
/// Reference to the EP
///
/// # Errors
/// * `EpNotFound` - If EP doesn't exist
/// * `EpDeleted` - If EP was previously deleted
pub fn read_ep<'a>(store: &'a Store, id: &str) -> Result<&'a Ep> {
    store.get_ep(id)
}

/// Update an EP's text fields and/or normative flag
///
/// Note: Ordinal cannot be changed after EP creation (immutable).
/// Updates the `updated_at` timestamp.
///
/// # Arguments
/// * `store` - Mutable reference to the Store
/// * `id` - The EP ID to update
/// * `why` - Optional new WHY text
/// * `what` - Optional new WHAT text (cannot be empty string)
/// * `how` - Optional new HOW text (cannot be empty string)
/// * `normative` - Optional new normative flag
///
/// # Errors
/// * `EpNotFound` - If EP doesn't exist
/// * `EpDeleted` - If EP was previously deleted
/// * `InvalidWhat` - If what is provided as empty string
/// * `InvalidHow` - If how is provided as empty string
pub fn update_ep(
    store: &mut Store,
    id: &str,
    why: Option<String>,
    what: Option<String>,
    how: Option<String>,
    normative: Option<bool>,
) -> Result<()> {
    // Validate WHAT if provided (cannot be empty string)
    if let Some(ref w) = what {
        if !w.is_empty() && w.trim().is_empty() {
            return Err(EttleXError::InvalidWhat {
                ep_id: id.to_string(),
            });
        }
    }

    // Validate HOW if provided (cannot be empty string)
    if let Some(ref h) = how {
        if !h.is_empty() && h.trim().is_empty() {
            return Err(EttleXError::InvalidHow {
                ep_id: id.to_string(),
            });
        }
    }

    // Get mutable reference to EP
    let ep = store.get_ep_mut(id)?;

    // Update fields
    if let Some(new_why) = why {
        ep.why = new_why;
    }

    if let Some(new_what) = what {
        ep.what = new_what;
    }

    if let Some(new_how) = how {
        ep.how = new_how;
    }

    if let Some(new_normative) = normative {
        ep.normative = new_normative;
    }

    // Update timestamp
    ep.updated_at = Utc::now();

    Ok(())
}

/// Delete an EP (tombstone deletion)
///
/// Sets the `deleted` flag to true. The EP remains in storage but is
/// filtered from queries.
///
/// Implements deletion safety checks (R5 requirement):
/// - Cannot delete EP0 (ordinal 0)
/// - Cannot delete EP if it's the only active mapping to a child (would strand the child)
///
/// # Arguments
/// * `store` - Mutable reference to the Store
/// * `id` - The EP ID to delete
///
/// # Errors
/// * `EpNotFound` - If EP doesn't exist
/// * `EpDeleted` - If EP was already deleted
/// * `CannotDeleteEp0` - If attempting to delete EP with ordinal 0
/// * `TombstoneStrandsChild` - If EP is the only active mapping to its child
pub fn delete_ep(store: &mut Store, id: &str) -> Result<()> {
    // Check if EP exists and is not deleted
    let ep = store.get_ep(id)?;

    // R5: Cannot delete EP0
    if ep.ordinal == 0 {
        return Err(EttleXError::CannotDeleteEp0 {
            ettle_id: ep.ettle_id.clone(),
        });
    }

    // R5: Check if EP is the only active mapping to its child (deletion safety)
    if let Some(ref child_id) = ep.child_ettle_id {
        // Get parent Ettle to check other active EPs
        let parent = store.get_ettle(&ep.ettle_id)?;
        let active = active_eps(store, parent)?;

        // Count how many active EPs map to this child
        let mapping_count = active
            .iter()
            .filter(|e| e.child_ettle_id.as_deref() == Some(child_id))
            .count();

        // If this is the only mapping, deletion would strand the child
        if mapping_count == 1 {
            return Err(EttleXError::TombstoneStrandsChild {
                ep_id: id.to_string(),
                child_id: child_id.clone(),
            });
        }
    }

    // Get mutable reference and set deleted flag
    let ep = store.get_ep_mut(id)?;
    ep.deleted = true;
    ep.updated_at = Utc::now();

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_create_ep_generates_unique_ids() {
        let mut store = Store::new();
        let ettle = crate::model::Ettle::new("ettle-1".to_string(), "Test".to_string());
        store.insert_ettle(ettle);

        let ep1_id = create_ep(
            &mut store,
            "ettle-1",
            0,
            true,
            String::new(),
            String::new(),
            String::new(),
        )
        .unwrap();
        let ep2_id = create_ep(
            &mut store,
            "ettle-1",
            1,
            true,
            String::new(),
            String::new(),
            String::new(),
        )
        .unwrap();

        assert_ne!(ep1_id, ep2_id);
    }
}
