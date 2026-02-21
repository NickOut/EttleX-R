/// Scenario 4: Remove/Tombstone EP
///
/// Tests EP deletion (tombstoning) and active EP projection filtering.
use ettlex_core::errors::EttleXError;
use ettlex_core::ops::{active_eps, ep_ops, ettle_ops, refinement_ops, Store};

#[test]
fn test_scenario_04_happy_tombstone_ep_disappears_from_active() {
    // GIVEN an Ettle with EP0 and EP1
    let mut store = Store::new();
    let ettle_id =
        ettle_ops::create_ettle(&mut store, "Test Ettle".to_string(), None, None, None, None)
            .expect("Should create Ettle");

    let ep1_id = ep_ops::create_ep(
        &mut store,
        &ettle_id,
        1,
        false,
        "".to_string(),
        "EP1".to_string(),
        "".to_string(),
    )
    .expect("Should create EP1");

    // WHEN tombstoning EP1
    ep_ops::delete_ep(&mut store, &ep1_id).expect("Should delete EP1");

    // THEN active_eps only returns EP0
    let ettle = store.get_ettle(&ettle_id).expect("Ettle should exist");
    let active = active_eps(&store, ettle).expect("Should get active EPs");

    assert_eq!(active.len(), 1);
    assert_eq!(active[0].ordinal, 0);

    // AND EP1 still exists but get_ep returns error (deleted)
    let result = store.get_ep(&ep1_id);
    assert!(result.is_err());
    assert!(matches!(result, Err(EttleXError::EpDeleted { .. })));
}

#[test]
fn test_scenario_04_error_delete_only_mapping_ep_strands_child() {
    // GIVEN a parent with EP1 mapping to a child
    let mut store = Store::new();
    let parent_id =
        ettle_ops::create_ettle(&mut store, "Parent".to_string(), None, None, None, None)
            .expect("Should create parent");

    let child_id = ettle_ops::create_ettle(&mut store, "Child".to_string(), None, None, None, None)
        .expect("Should create child");

    let ep1_id = ep_ops::create_ep(
        &mut store,
        &parent_id,
        1,
        false,
        "".to_string(),
        "EP1".to_string(),
        "".to_string(),
    )
    .expect("Should create EP1");

    // Link EP1 to child
    refinement_ops::link_child(&mut store, &ep1_id, &child_id).expect("Should link child");

    // WHEN trying to delete EP1 (the only mapping to child)
    let result = ep_ops::delete_ep(&mut store, &ep1_id);

    // THEN it should fail with TombstoneStrandsChild error
    assert!(result.is_err());
    assert!(matches!(
        result,
        Err(EttleXError::TombstoneStrandsChild { .. })
    ));
}

#[test]
fn test_scenario_04_error_cannot_delete_ep0() {
    // GIVEN an Ettle with EP0
    let mut store = Store::new();
    let ettle_id =
        ettle_ops::create_ettle(&mut store, "Test Ettle".to_string(), None, None, None, None)
            .expect("Should create Ettle");

    // Get EP0 ID
    let ettle = store.get_ettle(&ettle_id).expect("Ettle should exist");
    let ep0_id = ettle.ep_ids[0].clone();

    // WHEN trying to delete EP0
    let result = ep_ops::delete_ep(&mut store, &ep0_id);

    // THEN it should fail with CannotDeleteEp0 error
    assert!(result.is_err());
    assert!(matches!(result, Err(EttleXError::CannotDeleteEp0 { .. })));
}

#[test]
fn test_scenario_04_delete_allowed_when_multiple_mappings_exist() {
    // GIVEN a parent with EP1 and EP2 both mapping to the same child
    let mut store = Store::new();
    let parent_id =
        ettle_ops::create_ettle(&mut store, "Parent".to_string(), None, None, None, None)
            .expect("Should create parent");

    let child_id = ettle_ops::create_ettle(&mut store, "Child".to_string(), None, None, None, None)
        .expect("Should create child");

    let ep1_id = ep_ops::create_ep(
        &mut store,
        &parent_id,
        1,
        false,
        "".to_string(),
        "EP1".to_string(),
        "".to_string(),
    )
    .expect("Should create EP1");

    let ep2_id = ep_ops::create_ep(
        &mut store,
        &parent_id,
        2,
        false,
        "".to_string(),
        "EP2".to_string(),
        "".to_string(),
    )
    .expect("Should create EP2");

    // Link both EPs to child (violates one-to-one, but for this test scenario)
    // Unlink child first
    refinement_ops::link_child(&mut store, &ep1_id, &child_id).expect("Should link to EP1");

    // Manually set EP2's child (bypassing validation for test setup)
    {
        let ep2 = store.get_ep_mut(&ep2_id).expect("EP2 exists");
        ep2.child_ettle_id = Some(child_id.clone());
    }

    // WHEN deleting EP1 (not the only mapping - EP2 still maps to child)
    let result = ep_ops::delete_ep(&mut store, &ep1_id);

    // THEN deletion succeeds because EP2 still maps to child
    assert!(result.is_ok());
}
