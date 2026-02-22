mod common;

use common::{new_store, setup_simple_tree};
use ettlex_core::{
    ops::{ep_ops, ettle_ops},
    traversal::ept,
    EttleXError,
};

// ===== EPT COMPUTATION TESTS =====

#[test]
fn test_ept_for_root_returns_ep0() {
    let mut store = new_store();
    let root_id =
        ettle_ops::create_ettle(&mut store, "Root".to_string(), None, None, None, None).unwrap();

    let ept = ept::compute_ept(&store, &root_id, None).unwrap();

    assert_eq!(ept.len(), 1);

    let ep = store.get_ep(&ept[0]).unwrap();
    assert_eq!(ep.ordinal, 0);
}

#[test]
fn test_ept_walks_rt_and_collects_eps() {
    let mut store = new_store();
    let (root_id, mid_id, leaf_id) = setup_simple_tree(&mut store);

    let ept = ept::compute_ept(&store, &leaf_id, None).unwrap();

    // Should have 4 EPs: root EP0, root EP1 (to mid), mid EP1 (to leaf), leaf EP0
    assert_eq!(ept.len(), 4);

    // Verify ordinals
    let ep0 = store.get_ep(&ept[0]).unwrap();
    assert_eq!(ep0.ettle_id, root_id);
    assert_eq!(ep0.ordinal, 0);

    let ep1 = store.get_ep(&ept[1]).unwrap();
    assert_eq!(ep1.ettle_id, root_id);
    assert_eq!(ep1.ordinal, 1);

    let ep2 = store.get_ep(&ept[2]).unwrap();
    assert_eq!(ep2.ettle_id, mid_id);
    assert_eq!(ep2.ordinal, 1);

    let ep3 = store.get_ep(&ept[3]).unwrap();
    assert_eq!(ep3.ettle_id, leaf_id);
    assert_eq!(ep3.ordinal, 0);
}

#[test]
fn test_ept_fails_on_missing_mapping() {
    let mut store = new_store();

    let parent_id =
        ettle_ops::create_ettle(&mut store, "Parent".to_string(), None, None, None, None).unwrap();
    let child_id =
        ettle_ops::create_ettle(&mut store, "Child".to_string(), None, None, None, None).unwrap();

    // Set child's parent but don't create EP mapping (bypass refinement_ops)
    let child = store.get_ettle_mut(&child_id).unwrap();
    child.parent_id = Some(parent_id.clone());

    let result = ept::compute_ept(&store, &child_id, None);

    assert!(result.is_err());
    assert!(matches!(result, Err(EttleXError::EptMissingMapping { .. })));
}

#[test]
fn test_ept_fails_on_duplicate_mapping() {
    let mut store = new_store();

    let parent_id =
        ettle_ops::create_ettle(&mut store, "Parent".to_string(), None, None, None, None).unwrap();
    let child_id =
        ettle_ops::create_ettle(&mut store, "Child".to_string(), None, None, None, None).unwrap();

    let ep1 = ep_ops::create_ep(
        &mut store,
        &parent_id,
        1,
        true,
        String::new(),
        String::new(),
        String::new(),
    )
    .unwrap();
    let ep2 = ep_ops::create_ep(
        &mut store,
        &parent_id,
        2,
        true,
        String::new(),
        String::new(),
        String::new(),
    )
    .unwrap();

    // Link child to both EPs (bypass refinement_ops which prevents this)
    let ep1_mut = store.get_ep_mut(&ep1).unwrap();
    ep1_mut.child_ettle_id = Some(child_id.clone());

    let ep2_mut = store.get_ep_mut(&ep2).unwrap();
    ep2_mut.child_ettle_id = Some(child_id.clone());

    let child = store.get_ettle_mut(&child_id).unwrap();
    child.parent_id = Some(parent_id.clone());

    let result = ept::compute_ept(&store, &child_id, None);

    assert!(result.is_err());
    assert!(matches!(
        result,
        Err(EttleXError::EptDuplicateMapping { .. })
    ));
}

#[test]
fn test_ept_with_leaf_ep_ordinal_specified() {
    let mut store = new_store();
    let root_id =
        ettle_ops::create_ettle(&mut store, "Root".to_string(), None, None, None, None).unwrap();

    // Create multiple EPs for root
    let ep0_id = store.get_ettle(&root_id).unwrap().ep_ids[0].clone();
    ep_ops::create_ep(
        &mut store,
        &root_id,
        1,
        true,
        String::new(),
        String::new(),
        String::new(),
    )
    .unwrap();
    let ep2_id = ep_ops::create_ep(
        &mut store,
        &root_id,
        2,
        false,
        String::new(),
        String::new(),
        String::new(),
    )
    .unwrap();

    // Compute EPT ending with EP ordinal 2
    let ept = ept::compute_ept(&store, &root_id, Some(2)).unwrap();

    assert_eq!(ept.len(), 2);
    assert_eq!(ept[0], ep0_id);
    assert_eq!(ept[1], ep2_id);
}

#[test]
fn test_ept_fails_when_leaf_has_multiple_eps_and_no_ordinal() {
    let mut store = new_store();
    let root_id =
        ettle_ops::create_ettle(&mut store, "Root".to_string(), None, None, None, None).unwrap();

    // Create multiple EPs (EP0 already exists, add EP1 and EP2)
    ep_ops::create_ep(
        &mut store,
        &root_id,
        1,
        true,
        String::new(),
        String::new(),
        String::new(),
    )
    .unwrap();
    ep_ops::create_ep(
        &mut store,
        &root_id,
        2,
        true,
        String::new(),
        String::new(),
        String::new(),
    )
    .unwrap();

    // Try to compute EPT without specifying leaf EP ordinal
    let result = ept::compute_ept(&store, &root_id, None);

    assert!(result.is_err());
    assert!(matches!(
        result,
        Err(EttleXError::EptAmbiguousLeafEp { .. })
    ));
}

#[test]
fn test_ept_fails_when_specified_leaf_ep_not_found() {
    let mut store = new_store();
    let root_id =
        ettle_ops::create_ettle(&mut store, "Root".to_string(), None, None, None, None).unwrap();

    // Try to compute EPT with non-existent ordinal
    let result = ept::compute_ept(&store, &root_id, Some(99));

    assert!(result.is_err());
    assert!(matches!(result, Err(EttleXError::EptLeafEpNotFound { .. })));
}

#[test]
fn test_ept_deterministic_on_multiple_calls() {
    let mut store = new_store();
    let (_, _, leaf_id) = setup_simple_tree(&mut store);

    let ept1 = ept::compute_ept(&store, &leaf_id, None).unwrap();
    let ept2 = ept::compute_ept(&store, &leaf_id, None).unwrap();
    let ept3 = ept::compute_ept(&store, &leaf_id, None).unwrap();

    assert_eq!(ept1, ept2);
    assert_eq!(ept2, ept3);
}

#[test]
fn test_ept_includes_ep0_from_each_level() {
    let mut store = new_store();
    let (root_id, mid_id, leaf_id) = setup_simple_tree(&mut store);

    let ept = ept::compute_ept(&store, &leaf_id, None).unwrap();

    // Verify we have EP0 from root, EP1 from root (to mid), EP1 from mid (to leaf), EP0 from leaf
    assert_eq!(ept.len(), 4);

    // Root EP0
    let ep0 = store.get_ep(&ept[0]).unwrap();
    assert_eq!(ep0.ettle_id, root_id);
    assert_eq!(ep0.ordinal, 0);

    // Root EP1 (maps to mid)
    let ep1 = store.get_ep(&ept[1]).unwrap();
    assert_eq!(ep1.ettle_id, root_id);
    assert_eq!(ep1.ordinal, 1);
    assert_eq!(ep1.child_ettle_id, Some(mid_id.clone()));

    // Mid EP1 (maps to leaf)
    let ep2 = store.get_ep(&ept[2]).unwrap();
    assert_eq!(ep2.ettle_id, mid_id);
    assert_eq!(ep2.ordinal, 1);
    assert_eq!(ep2.child_ettle_id, Some(leaf_id.clone()));

    // Leaf EP0 (the actual leaf EP)
    let ep3 = store.get_ep(&ept[3]).unwrap();
    assert_eq!(ep3.ettle_id, leaf_id);
    assert_eq!(ep3.ordinal, 0);
}
