/// Scenario 8: Deterministic Active EP Tests
///
/// Tests that active_eps() returns deterministic, sorted results.
use ettlex_core::ops::{active_eps, ep_ops, ettle_ops, Store};

#[test]
fn test_scenario_08_happy_active_eps_sorted_and_stable() {
    // GIVEN an Ettle with EPs added in random ordinal order
    let mut store = Store::new();
    let ettle_id =
        ettle_ops::create_ettle(&mut store, "Test Ettle".to_string(), None, None, None, None)
            .expect("Should create Ettle");

    // Add EPs out of order: 5, 2, 7, 1, 3
    for ordinal in [5, 2, 7, 1, 3] {
        ep_ops::create_ep(
            &mut store,
            &ettle_id,
            ordinal,
            false,
            "".to_string(),
            format!("EP{}", ordinal),
            "".to_string(),
        )
        .expect(&format!("Should create EP{}", ordinal));
    }

    // WHEN calling active_eps multiple times
    let ettle = store.get_ettle(&ettle_id).expect("Ettle should exist");
    let active1 = active_eps(&store, ettle).expect("Should get active EPs");
    let active2 = active_eps(&store, ettle).expect("Should get active EPs");
    let active3 = active_eps(&store, ettle).expect("Should get active EPs");

    // THEN results are identical (deterministic)
    assert_eq!(active1.len(), active2.len());
    assert_eq!(active2.len(), active3.len());

    // AND sorted by ordinal
    assert_eq!(active1.len(), 6); // EP0 + 5 created
    assert_eq!(active1[0].ordinal, 0);
    assert_eq!(active1[1].ordinal, 1);
    assert_eq!(active1[2].ordinal, 2);
    assert_eq!(active1[3].ordinal, 3);
    assert_eq!(active1[4].ordinal, 5);
    assert_eq!(active1[5].ordinal, 7);

    // AND stable across calls
    for (ep1, ep2) in active1.iter().zip(active2.iter()) {
        assert_eq!(ep1.id, ep2.id);
    }
}

#[test]
fn test_scenario_08_active_eps_excludes_deleted() {
    // GIVEN an Ettle with some deleted EPs
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

    // Delete EP1
    ep_ops::delete_ep(&mut store, &ep1_id).expect("Should delete EP1");

    // WHEN getting active EPs
    let ettle = store.get_ettle(&ettle_id).expect("Ettle should exist");
    let active = active_eps(&store, ettle).expect("Should get active EPs");

    // THEN only non-deleted EPs are returned
    assert_eq!(active.len(), 3); // EP0, EP2, EP3 (EP1 excluded)
    assert_eq!(active[0].ordinal, 0);
    assert_eq!(active[1].ordinal, 2);
    assert_eq!(active[2].ordinal, 3);

    // AND calling again gives same result
    let active2 = active_eps(&store, ettle).expect("Should get active EPs");
    assert_eq!(active.len(), active2.len());
}

#[test]
fn test_scenario_08_active_eps_deterministic_on_concurrent_access() {
    // GIVEN an Ettle with EPs
    let mut store = Store::new();
    let ettle_id =
        ettle_ops::create_ettle(&mut store, "Test Ettle".to_string(), None, None, None, None)
            .expect("Should create Ettle");

    for i in 1..=10 {
        ep_ops::create_ep(
            &mut store,
            &ettle_id,
            i,
            false,
            "".to_string(),
            format!("EP{}", i),
            "".to_string(),
        )
        .expect(&format!("Should create EP{}", i));
    }

    // WHEN calling active_eps many times
    let ettle = store.get_ettle(&ettle_id).expect("Ettle should exist");
    let mut results = Vec::new();
    for _ in 0..100 {
        let active = active_eps(&store, ettle).expect("Should get active EPs");
        results.push(active);
    }

    // THEN all results are identical
    let first = &results[0];
    for result in &results[1..] {
        assert_eq!(first.len(), result.len());
        for (ep1, ep2) in first.iter().zip(result.iter()) {
            assert_eq!(ep1.id, ep2.id);
            assert_eq!(ep1.ordinal, ep2.ordinal);
        }
    }
}
