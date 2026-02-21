/// Scenario 3: Add EP to Ettle
///
/// Tests adding EPs to an Ettle and verifying active EP projection.
use ettlex_core::errors::EttleXError;
use ettlex_core::ops::{active_eps, ep_ops, ettle_ops, Store};

#[test]
fn test_scenario_03_happy_add_ep_and_list_via_active_eps() {
    // GIVEN an Ettle with EP0
    let mut store = Store::new();
    let ettle_id =
        ettle_ops::create_ettle(&mut store, "Test Ettle".to_string(), None, None, None, None)
            .expect("Should create Ettle");

    // WHEN adding EP1
    let ep1_id = ep_ops::create_ep(
        &mut store,
        &ettle_id,
        1,
        false,
        "Why EP1".to_string(),
        "What EP1".to_string(),
        "How EP1".to_string(),
    )
    .expect("Should create EP1");

    // THEN active_eps returns both EP0 and EP1 in ordinal order
    let ettle = store.get_ettle(&ettle_id).expect("Ettle should exist");
    let active = active_eps(&store, ettle).expect("Should get active EPs");

    assert_eq!(active.len(), 2);
    assert_eq!(active[0].ordinal, 0);
    assert_eq!(active[1].ordinal, 1);
    assert_eq!(active[1].id, ep1_id);
}

#[test]
fn test_scenario_03_error_duplicate_ordinal() {
    // GIVEN an Ettle with EP0
    let mut store = Store::new();
    let ettle_id =
        ettle_ops::create_ettle(&mut store, "Test Ettle".to_string(), None, None, None, None)
            .expect("Should create Ettle");

    // WHEN trying to add another EP with ordinal 0
    let result = ep_ops::create_ep(
        &mut store,
        &ettle_id,
        0, // Duplicate ordinal!
        false,
        "Why".to_string(),
        "What".to_string(),
        "How".to_string(),
    );

    // THEN it should fail with OrdinalAlreadyExists error
    assert!(result.is_err());
    assert!(matches!(
        result,
        Err(EttleXError::OrdinalAlreadyExists { .. })
    ));
}

#[test]
fn test_scenario_03_error_add_to_tombstoned_ettle() {
    // GIVEN a tombstoned Ettle
    let mut store = Store::new();
    let ettle_id =
        ettle_ops::create_ettle(&mut store, "Test Ettle".to_string(), None, None, None, None)
            .expect("Should create Ettle");

    // Delete the Ettle (tombstone it)
    ettle_ops::delete_ettle(&mut store, &ettle_id).expect("Should delete Ettle");

    // WHEN trying to add an EP to the tombstoned Ettle
    let result = ep_ops::create_ep(
        &mut store,
        &ettle_id,
        1,
        false,
        "Why".to_string(),
        "What".to_string(),
        "How".to_string(),
    );

    // THEN it should fail with EttleDeleted error
    assert!(result.is_err());
    assert!(matches!(result, Err(EttleXError::EttleDeleted { .. })));
}

#[test]
fn test_scenario_03_active_eps_deterministic_ordering() {
    // GIVEN an Ettle with multiple EPs added in non-ordinal order
    let mut store = Store::new();
    let ettle_id =
        ettle_ops::create_ettle(&mut store, "Test Ettle".to_string(), None, None, None, None)
            .expect("Should create Ettle");

    // Add EPs in random ordinal order
    ep_ops::create_ep(
        &mut store,
        &ettle_id,
        3,
        false,
        "".to_string(),
        "EP3".to_string(),
        "".to_string(),
    )
    .expect("Should create EP3");

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

    // THEN they are sorted by ordinal (0, 1, 2, 3)
    assert_eq!(active.len(), 4);
    assert_eq!(active[0].ordinal, 0);
    assert_eq!(active[1].ordinal, 1);
    assert_eq!(active[2].ordinal, 2);
    assert_eq!(active[3].ordinal, 3);
}
