mod common;

use common::{new_store, setup_simple_tree};
use ettlex_core::{
    ops::{ep_ops, ettle_ops, refinement_ops},
    traversal::rt,
    EttleXError,
};

// ===== RT COMPUTATION TESTS =====

#[test]
fn test_rt_for_root_is_single_element() {
    let mut store = new_store();
    let root_id =
        ettle_ops::create_ettle(&mut store, "Root".to_string(), None, None, None, None).unwrap();

    let rt = rt::compute_rt(&store, &root_id).unwrap();

    assert_eq!(rt.len(), 1);
    assert_eq!(rt[0], root_id);
}

#[test]
fn test_rt_is_root_to_leaf_order() {
    let mut store = new_store();
    let (root_id, mid_id, leaf_id) = setup_simple_tree(&mut store);

    let rt = rt::compute_rt(&store, &leaf_id).unwrap();

    assert_eq!(rt.len(), 3);
    assert_eq!(rt[0], root_id);
    assert_eq!(rt[1], mid_id);
    assert_eq!(rt[2], leaf_id);
}

#[test]
fn test_rt_handles_deep_chain() {
    let mut store = new_store();

    // Create a 5-level chain: A -> B -> C -> D -> E
    let a = ettle_ops::create_ettle(&mut store, "A".to_string(), None, None, None, None).unwrap();
    let b = ettle_ops::create_ettle(&mut store, "B".to_string(), None, None, None, None).unwrap();
    let c = ettle_ops::create_ettle(&mut store, "C".to_string(), None, None, None, None).unwrap();
    let d = ettle_ops::create_ettle(&mut store, "D".to_string(), None, None, None, None).unwrap();
    let e = ettle_ops::create_ettle(&mut store, "E".to_string(), None, None, None, None).unwrap();

    let ep_ab = ep_ops::create_ep(
        &mut store,
        &a,
        1,
        true,
        String::new(),
        String::new(),
        String::new(),
    )
    .unwrap();
    let ep_bc = ep_ops::create_ep(
        &mut store,
        &b,
        1,
        true,
        String::new(),
        String::new(),
        String::new(),
    )
    .unwrap();
    let ep_cd = ep_ops::create_ep(
        &mut store,
        &c,
        1,
        true,
        String::new(),
        String::new(),
        String::new(),
    )
    .unwrap();
    let ep_de = ep_ops::create_ep(
        &mut store,
        &d,
        1,
        true,
        String::new(),
        String::new(),
        String::new(),
    )
    .unwrap();

    refinement_ops::link_child(&mut store, &ep_ab, &b).unwrap();
    refinement_ops::link_child(&mut store, &ep_bc, &c).unwrap();
    refinement_ops::link_child(&mut store, &ep_cd, &d).unwrap();
    refinement_ops::link_child(&mut store, &ep_de, &e).unwrap();

    let rt = rt::compute_rt(&store, &e).unwrap();

    assert_eq!(rt.len(), 5);
    assert_eq!(rt, vec![a, b, c, d, e]);
}

#[test]
fn test_rt_fails_on_nonexistent_ettle() {
    let store = new_store();

    let result = rt::compute_rt(&store, "nonexistent");

    assert!(result.is_err());
    assert!(matches!(result, Err(EttleXError::EttleNotFound { .. })));
}

#[test]
fn test_rt_fails_on_broken_parent_chain() {
    let mut store = new_store();

    let child_id =
        ettle_ops::create_ettle(&mut store, "Child".to_string(), None, None, None, None).unwrap();

    // Manually set parent_id to nonexistent ettle (bypass refinement_ops)
    let child = store.get_ettle_mut(&child_id).unwrap();
    child.parent_id = Some("nonexistent-parent".to_string());

    let result = rt::compute_rt(&store, &child_id);

    assert!(result.is_err());
    assert!(matches!(
        result,
        Err(EttleXError::RtParentChainBroken { .. })
    ));
}

#[test]
fn test_rt_deterministic_on_multiple_calls() {
    let mut store = new_store();
    let (_, _, leaf_id) = setup_simple_tree(&mut store);

    let rt1 = rt::compute_rt(&store, &leaf_id).unwrap();
    let rt2 = rt::compute_rt(&store, &leaf_id).unwrap();
    let rt3 = rt::compute_rt(&store, &leaf_id).unwrap();

    assert_eq!(rt1, rt2);
    assert_eq!(rt2, rt3);
}

#[test]
fn test_rt_for_mid_level_ettle() {
    let mut store = new_store();
    let (root_id, mid_id, _) = setup_simple_tree(&mut store);

    let rt = rt::compute_rt(&store, &mid_id).unwrap();

    assert_eq!(rt.len(), 2);
    assert_eq!(rt[0], root_id);
    assert_eq!(rt[1], mid_id);
}
