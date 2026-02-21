/// Scenario 5: Membership Integrity
///
/// Tests bidirectional membership consistency between Ettle.ep_ids and EP.ettle_id.
use ettlex_core::errors::EttleXError;
use ettlex_core::model::{Ep, Ettle};
use ettlex_core::ops::{active_eps, ettle_ops, Store};
use ettlex_core::rules::{invariants, validation};

#[test]
fn test_scenario_05_happy_consistent_bidirectional_membership() {
    // GIVEN an Ettle created normally with EP0
    let mut store = Store::new();
    let ettle_id =
        ettle_ops::create_ettle(&mut store, "Test Ettle".to_string(), None, None, None, None)
            .expect("Should create Ettle");

    // WHEN checking membership consistency
    let inconsistencies = invariants::find_membership_inconsistencies(&store);
    let orphans = invariants::find_ep_orphans(&store);

    // THEN there are no inconsistencies
    assert!(inconsistencies.is_empty());
    assert!(orphans.is_empty());

    // AND active_eps works correctly
    let ettle = store.get_ettle(&ettle_id).expect("Ettle should exist");
    let active = active_eps(&store, ettle);
    assert!(active.is_ok());
}

#[test]
fn test_scenario_05_error_ep_listed_but_ownership_mismatch() {
    // GIVEN two Ettles and an EP with mismatched ownership
    let mut store = Store::new();
    let mut ettle1 = Ettle::new("ettle-1".to_string(), "Test 1".to_string());
    let ettle2 = Ettle::new("ettle-2".to_string(), "Test 2".to_string());

    // Create EP pointing to ettle-2
    let ep = Ep::new(
        "ep-1".to_string(),
        "ettle-2".to_string(), // Points to ettle-2
        0,
        true,
        "".to_string(),
        "".to_string(),
        "".to_string(),
    );

    // Add EP to ettle-1's list (but ownership points to ettle-2)
    ettle1.add_ep_id("ep-1".to_string());

    store.insert_ettle(ettle1);
    store.insert_ettle(ettle2);
    store.insert_ep(ep);

    // WHEN checking membership
    let inconsistencies = invariants::find_membership_inconsistencies(&store);

    // THEN membership inconsistency is detected
    assert_eq!(inconsistencies.len(), 1);
    assert_eq!(inconsistencies[0].0, "ep-1");
    assert_eq!(inconsistencies[0].1, "ettle-2"); // EP points here
    assert_eq!(inconsistencies[0].2, "ettle-1"); // But listed here

    // AND validate_tree fails
    let result = validation::validate_tree(&store);
    assert!(result.is_err());
    assert!(matches!(
        result,
        Err(EttleXError::MembershipInconsistent { .. })
    ));
}

#[test]
fn test_scenario_05_error_ep_orphaned_not_listed() {
    // GIVEN an Ettle and an EP where EP points to Ettle but Ettle doesn't list EP
    let mut store = Store::new();
    let ettle = Ettle::new("ettle-1".to_string(), "Test".to_string());

    // Create EP pointing to ettle
    let ep = Ep::new(
        "ep-1".to_string(),
        "ettle-1".to_string(),
        0,
        true,
        "".to_string(),
        "".to_string(),
        "".to_string(),
    );

    // Insert without adding EP to ettle.ep_ids
    store.insert_ettle(ettle);
    store.insert_ep(ep);

    // WHEN checking for orphans
    let orphans = invariants::find_ep_orphans(&store);

    // THEN EP orphan is detected
    assert_eq!(orphans.len(), 1);
    assert_eq!(orphans[0].0, "ep-1");
    assert_eq!(orphans[0].1, "ettle-1");

    // AND validate_tree fails
    let result = validation::validate_tree(&store);
    assert!(result.is_err());
    assert!(matches!(result, Err(EttleXError::EpOrphaned { .. })));
}

#[test]
fn test_scenario_05_unknown_ep_ref_detected() {
    // GIVEN an Ettle that references a non-existent EP
    let mut store = Store::new();
    let mut ettle = Ettle::new("ettle-1".to_string(), "Test".to_string());
    ettle.add_ep_id("nonexistent-ep".to_string()); // EP doesn't exist!

    store.insert_ettle(ettle);

    // WHEN checking for unknown EP refs
    let unknown_refs = invariants::find_unknown_ep_refs(&store);

    // THEN unknown EP reference is detected
    assert_eq!(unknown_refs.len(), 1);
    assert_eq!(unknown_refs[0].0, "ettle-1");
    assert_eq!(unknown_refs[0].1, "nonexistent-ep");

    // AND validate_tree fails
    let result = validation::validate_tree(&store);
    assert!(result.is_err());
    assert!(matches!(
        result,
        Err(EttleXError::EpListContainsUnknownId { .. })
    ));
}
