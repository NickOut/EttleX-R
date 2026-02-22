/// Scenario 9: EPT Mapping Sensitivity Tests
///
/// Tests EPT computation with respect to active EP projection and membership integrity.
use ettlex_core::ops::{active_eps, ep_ops, ettle_ops, refinement_ops, Store};
use ettlex_core::traversal::ept::compute_ept;

#[test]
fn test_scenario_09_happy_ept_with_consistent_membership() {
    // GIVEN a simple parent-child hierarchy
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
        "Refinement".to_string(),
        "".to_string(),
    )
    .expect("Should create EP1");

    refinement_ops::link_child(&mut store, &ep1_id, &child_id).expect("Should link child");

    // WHEN computing EPT for child
    let ept_ids = compute_ept(&store, &child_id, None).expect("Should compute EPT");

    // THEN EPT includes EP IDs from parent hierarchy
    assert!(!ept_ids.is_empty());
    assert!(ept_ids.len() >= 2); // At least parent and child EPs

    // Verify all EPs in EPT exist and are valid
    for ep_id in &ept_ids {
        // EPT should only include active (non-deleted) EPs
        store
            .get_ep(ep_id)
            .expect("EP in EPT should exist and be active");
    }

    // AND active_eps consistency is maintained
    let parent = store.get_ettle(&parent_id).expect("Parent should exist");
    let active = active_eps(&store, parent).expect("Should get active EPs");
    assert!(active.iter().any(|ep| ep.id == ep1_id));
}

#[test]
fn test_scenario_09_ept_uses_only_active_eps() {
    // GIVEN a parent with some deleted EPs
    let mut store = Store::new();
    let parent_id =
        ettle_ops::create_ettle(&mut store, "Parent".to_string(), None, None, None, None)
            .expect("Should create parent");

    let child_id = ettle_ops::create_ettle(&mut store, "Child".to_string(), None, None, None, None)
        .expect("Should create child");

    // Create EP1 and EP2, link EP1 to child
    let ep1_id = ep_ops::create_ep(
        &mut store,
        &parent_id,
        1,
        false,
        "".to_string(),
        "Active EP".to_string(),
        "".to_string(),
    )
    .expect("Should create EP1");

    let ep2_id = ep_ops::create_ep(
        &mut store,
        &parent_id,
        2,
        false,
        "".to_string(),
        "To be deleted".to_string(),
        "".to_string(),
    )
    .expect("Should create EP2");

    refinement_ops::link_child(&mut store, &ep1_id, &child_id).expect("Should link child to EP1");

    // Delete EP2
    ep_ops::delete_ep(&mut store, &ep2_id).expect("Should delete EP2");

    // WHEN computing EPT
    let ept_ids = compute_ept(&store, &child_id, None).expect("Should compute EPT");

    // THEN EPT should include parent EP0, parent EP1 (active), and child EP0
    assert_eq!(ept_ids.len(), 3);

    // Verify EP1 is in EPT (not deleted EP2)
    assert!(ept_ids.contains(&ep1_id));
    assert!(!ept_ids.contains(&ep2_id));

    // Verify child EP0 is included
    let child = store.get_ettle(&child_id).expect("Child should exist");
    let child_eps = active_eps(&store, child).expect("Should get child EPs");
    let child_ep0 = &child_eps[0];
    assert!(ept_ids.contains(&child_ep0.id));
}

#[test]
fn test_scenario_09_ept_stable_across_calls() {
    // GIVEN a multi-level hierarchy
    let mut store = Store::new();
    let root_id = ettle_ops::create_ettle(&mut store, "Root".to_string(), None, None, None, None)
        .expect("Should create root");

    let mid_id = ettle_ops::create_ettle(&mut store, "Mid".to_string(), None, None, None, None)
        .expect("Should create mid");

    let leaf_id = ettle_ops::create_ettle(&mut store, "Leaf".to_string(), None, None, None, None)
        .expect("Should create leaf");

    // Link root -> mid -> leaf
    let root_ep1 = ep_ops::create_ep(
        &mut store,
        &root_id,
        1,
        false,
        "".to_string(),
        "Root->Mid".to_string(),
        "".to_string(),
    )
    .expect("Should create root EP");

    refinement_ops::link_child(&mut store, &root_ep1, &mid_id).expect("Should link mid to root");

    let mid_ep1 = ep_ops::create_ep(
        &mut store,
        &mid_id,
        1,
        false,
        "".to_string(),
        "Mid->Leaf".to_string(),
        "".to_string(),
    )
    .expect("Should create mid EP");

    refinement_ops::link_child(&mut store, &mid_ep1, &leaf_id).expect("Should link leaf to mid");

    // WHEN computing EPT multiple times
    let ept1 = compute_ept(&store, &leaf_id, None).expect("Should compute EPT");
    let ept2 = compute_ept(&store, &leaf_id, None).expect("Should compute EPT");
    let ept3 = compute_ept(&store, &leaf_id, None).expect("Should compute EPT");

    // THEN results are stable and deterministic
    assert_eq!(ept1.len(), ept2.len());
    assert_eq!(ept2.len(), ept3.len());

    // AND EP IDs match across calls
    for (id1, id2) in ept1.iter().zip(ept2.iter()) {
        assert_eq!(id1, id2);
    }

    for (id2, id3) in ept2.iter().zip(ept3.iter()) {
        assert_eq!(id2, id3);
    }
}
