/// Scenario 7: EP Ordinal Revalidation
///
/// Tests ordinal uniqueness, immutability, and reuse prevention.
use ettlex_core::errors::EttleXError;
use ettlex_core::ops::{active_eps, ep_ops, ettle_ops, Store};

#[test]
fn test_scenario_07_happy_ordinals_unique_and_stable() {
    // GIVEN an Ettle with multiple EPs
    let mut store = Store::new();
    let ettle_id =
        ettle_ops::create_ettle(&mut store, "Test Ettle".to_string(), None, None, None, None)
            .expect("Should create Ettle");

    ep_ops::create_ep(
        &mut store,
        &ettle_id,
        1,
        false,
        "".to_string(),
        "EP1".to_string(),
        "".to_string(),
    )
    .expect("Should create EP1");

    ep_ops::create_ep(
        &mut store,
        &ettle_id,
        2,
        false,
        "".to_string(),
        "EP2".to_string(),
        "".to_string(),
    )
    .expect("Should create EP2");

    // WHEN getting active EPs
    let ettle = store.get_ettle(&ettle_id).expect("Ettle should exist");
    let active = active_eps(&store, ettle).expect("Should get active EPs");

    // THEN ordinals are unique and stable
    assert_eq!(active.len(), 3); // EP0, EP1, EP2
    assert_eq!(active[0].ordinal, 0);
    assert_eq!(active[1].ordinal, 1);
    assert_eq!(active[2].ordinal, 2);

    // AND calling again gives same results (stability)
    let active2 = active_eps(&store, ettle).expect("Should get active EPs");
    assert_eq!(active.len(), active2.len());
    for (ep1, ep2) in active.iter().zip(active2.iter()) {
        assert_eq!(ep1.id, ep2.id);
        assert_eq!(ep1.ordinal, ep2.ordinal);
    }
}

#[test]
fn test_scenario_07_error_reuse_tombstoned_ordinal() {
    // GIVEN an Ettle with EP1 that gets tombstoned
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

    ep_ops::delete_ep(&mut store, &ep1_id).expect("Should delete EP1");

    // WHEN trying to create a new EP with ordinal 1 (reusing tombstoned ordinal)
    let result = ep_ops::create_ep(
        &mut store,
        &ettle_id,
        1, // Reusing ordinal!
        false,
        "".to_string(),
        "New EP1".to_string(),
        "".to_string(),
    );

    // THEN it should fail with EpOrdinalReuseForbidden error
    assert!(result.is_err());
    assert!(matches!(
        result,
        Err(EttleXError::EpOrdinalReuseForbidden { .. })
    ));
}

#[test]
fn test_scenario_07_ordinal_immutability() {
    // Note: Ordinal immutability is enforced at the type level
    // (no set_ordinal method exists), but we can test that
    // update_ep doesn't allow changing ordinals

    // GIVEN an Ettle with EP1
    let mut store = Store::new();
    let ettle_id =
        ettle_ops::create_ettle(&mut store, "Test Ettle".to_string(), None, None, None, None)
            .expect("Should create Ettle");

    let ep1_id = ep_ops::create_ep(
        &mut store,
        &ettle_id,
        1,
        false,
        "Original why".to_string(),
        "Original what".to_string(),
        "Original how".to_string(),
    )
    .expect("Should create EP1");

    // WHEN updating EP1's text fields
    ep_ops::update_ep(
        &mut store,
        &ep1_id,
        Some("New why".to_string()),
        Some("New what".to_string()),
        Some("New how".to_string()),
        None,
    )
    .expect("Should update EP");

    // THEN ordinal remains unchanged
    let ep1 = store.get_ep(&ep1_id).expect("EP1 should exist");
    assert_eq!(ep1.ordinal, 1); // Unchanged
    assert_eq!(ep1.why, "New why"); // Updated
}
