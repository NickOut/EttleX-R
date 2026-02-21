mod common;

use common::new_store;
use ettlex_core::{
    ops::{ep_ops, ettle_ops, refinement_ops},
    EttleXError,
};

// ===== SET_PARENT TESTS =====

#[test]
fn test_set_parent_links_child_to_parent() {
    let mut store = new_store();
    let parent_id =
        ettle_ops::create_ettle(&mut store, "Parent".to_string(), None, None, None, None).unwrap();
    let child_id =
        ettle_ops::create_ettle(&mut store, "Child".to_string(), None, None, None, None).unwrap();

    refinement_ops::set_parent(&mut store, &child_id, Some(&parent_id)).unwrap();

    let child = store.get_ettle(&child_id).unwrap();
    assert_eq!(child.parent_id, Some(parent_id));
}

#[test]
fn test_set_parent_to_none_makes_root() {
    let mut store = new_store();
    let parent_id =
        ettle_ops::create_ettle(&mut store, "Parent".to_string(), None, None, None, None).unwrap();
    let child_id =
        ettle_ops::create_ettle(&mut store, "Child".to_string(), None, None, None, None).unwrap();

    refinement_ops::set_parent(&mut store, &child_id, Some(&parent_id)).unwrap();
    assert!(store.get_ettle(&child_id).unwrap().has_parent());

    refinement_ops::set_parent(&mut store, &child_id, None).unwrap();
    assert!(store.get_ettle(&child_id).unwrap().is_root());
}

#[test]
fn test_set_parent_detects_direct_cycle() {
    let mut store = new_store();
    let ettle_id =
        ettle_ops::create_ettle(&mut store, "Ettle".to_string(), None, None, None, None).unwrap();

    // Try to set self as parent
    let result = refinement_ops::set_parent(&mut store, &ettle_id, Some(&ettle_id));

    assert!(result.is_err());
    assert!(matches!(result, Err(EttleXError::CycleDetected { .. })));
}

#[test]
fn test_set_parent_detects_indirect_cycle() {
    let mut store = new_store();
    let a_id =
        ettle_ops::create_ettle(&mut store, "A".to_string(), None, None, None, None).unwrap();
    let b_id =
        ettle_ops::create_ettle(&mut store, "B".to_string(), None, None, None, None).unwrap();
    let c_id =
        ettle_ops::create_ettle(&mut store, "C".to_string(), None, None, None, None).unwrap();

    // Create chain: A -> B -> C
    refinement_ops::set_parent(&mut store, &b_id, Some(&a_id)).unwrap();
    refinement_ops::set_parent(&mut store, &c_id, Some(&b_id)).unwrap();

    // Try to make A a child of C (would create cycle)
    let result = refinement_ops::set_parent(&mut store, &a_id, Some(&c_id));

    assert!(result.is_err());
    assert!(matches!(result, Err(EttleXError::CycleDetected { .. })));
}

#[test]
fn test_set_parent_fails_on_nonexistent_parent() {
    let mut store = new_store();
    let child_id =
        ettle_ops::create_ettle(&mut store, "Child".to_string(), None, None, None, None).unwrap();

    let result = refinement_ops::set_parent(&mut store, &child_id, Some("nonexistent"));

    assert!(result.is_err());
    assert!(matches!(result, Err(EttleXError::ParentNotFound { .. })));
}

#[test]
fn test_set_parent_fails_on_nonexistent_child() {
    let mut store = new_store();
    let parent_id =
        ettle_ops::create_ettle(&mut store, "Parent".to_string(), None, None, None, None).unwrap();

    let result = refinement_ops::set_parent(&mut store, "nonexistent", Some(&parent_id));

    assert!(result.is_err());
    assert!(matches!(result, Err(EttleXError::EttleNotFound { .. })));
}

// ===== LINK_CHILD TESTS =====

#[test]
fn test_link_child_maps_ep_to_child() {
    let mut store = new_store();
    let parent_id =
        ettle_ops::create_ettle(&mut store, "Parent".to_string(), None, None, None, None).unwrap();
    let child_id =
        ettle_ops::create_ettle(&mut store, "Child".to_string(), None, None, None, None).unwrap();

    let ep_id = ep_ops::create_ep(
        &mut store,
        &parent_id,
        1,
        true,
        String::new(),
        String::new(),
        String::new(),
    )
    .unwrap();

    refinement_ops::link_child(&mut store, &ep_id, &child_id).unwrap();

    let ep = store.get_ep(&ep_id).unwrap();
    assert_eq!(ep.child_ettle_id, Some(child_id.clone()));

    let child = store.get_ettle(&child_id).unwrap();
    assert_eq!(child.parent_id, Some(parent_id));
}

#[test]
fn test_link_child_fails_when_child_already_has_parent() {
    let mut store = new_store();
    let parent1_id =
        ettle_ops::create_ettle(&mut store, "Parent1".to_string(), None, None, None, None).unwrap();
    let parent2_id =
        ettle_ops::create_ettle(&mut store, "Parent2".to_string(), None, None, None, None).unwrap();
    let child_id =
        ettle_ops::create_ettle(&mut store, "Child".to_string(), None, None, None, None).unwrap();

    let ep1_id = ep_ops::create_ep(
        &mut store,
        &parent1_id,
        1,
        true,
        String::new(),
        String::new(),
        String::new(),
    )
    .unwrap();

    let ep2_id = ep_ops::create_ep(
        &mut store,
        &parent2_id,
        1,
        true,
        String::new(),
        String::new(),
        String::new(),
    )
    .unwrap();

    // Link child to parent1
    refinement_ops::link_child(&mut store, &ep1_id, &child_id).unwrap();

    // Try to link same child to parent2
    let result = refinement_ops::link_child(&mut store, &ep2_id, &child_id);

    assert!(result.is_err());
    assert!(matches!(
        result,
        Err(EttleXError::ChildAlreadyHasParent { .. })
    ));
}

#[test]
fn test_link_child_fails_when_ep_already_has_child() {
    let mut store = new_store();
    let parent_id =
        ettle_ops::create_ettle(&mut store, "Parent".to_string(), None, None, None, None).unwrap();
    let child1_id =
        ettle_ops::create_ettle(&mut store, "Child1".to_string(), None, None, None, None).unwrap();
    let child2_id =
        ettle_ops::create_ettle(&mut store, "Child2".to_string(), None, None, None, None).unwrap();

    let ep_id = ep_ops::create_ep(
        &mut store,
        &parent_id,
        1,
        true,
        String::new(),
        String::new(),
        String::new(),
    )
    .unwrap();

    // Link child1 to EP
    refinement_ops::link_child(&mut store, &ep_id, &child1_id).unwrap();

    // Try to link child2 to same EP
    let result = refinement_ops::link_child(&mut store, &ep_id, &child2_id);

    assert!(result.is_err());
    assert!(matches!(result, Err(EttleXError::EpAlreadyHasChild { .. })));
}

// ===== UNLINK_CHILD TESTS =====

#[test]
fn test_unlink_child_removes_mapping() {
    let mut store = new_store();
    let parent_id =
        ettle_ops::create_ettle(&mut store, "Parent".to_string(), None, None, None, None).unwrap();
    let child_id =
        ettle_ops::create_ettle(&mut store, "Child".to_string(), None, None, None, None).unwrap();

    let ep_id = ep_ops::create_ep(
        &mut store,
        &parent_id,
        1,
        true,
        String::new(),
        String::new(),
        String::new(),
    )
    .unwrap();

    refinement_ops::link_child(&mut store, &ep_id, &child_id).unwrap();
    refinement_ops::unlink_child(&mut store, &ep_id).unwrap();

    let ep = store.get_ep(&ep_id).unwrap();
    assert!(ep.is_leaf());

    let child = store.get_ettle(&child_id).unwrap();
    assert!(child.is_root());
}

#[test]
fn test_unlink_child_on_leaf_ep_is_noop() {
    let mut store = new_store();
    let parent_id =
        ettle_ops::create_ettle(&mut store, "Parent".to_string(), None, None, None, None).unwrap();

    let ep_id = ep_ops::create_ep(
        &mut store,
        &parent_id,
        1,
        true,
        String::new(),
        String::new(),
        String::new(),
    )
    .unwrap();

    // EP has no child, unlink should succeed as no-op
    refinement_ops::unlink_child(&mut store, &ep_id).unwrap();

    let ep = store.get_ep(&ep_id).unwrap();
    assert!(ep.is_leaf());
}

// ===== LIST_CHILDREN TESTS =====

#[test]
fn test_list_children_returns_children_in_ordinal_order() {
    let mut store = new_store();
    let parent_id =
        ettle_ops::create_ettle(&mut store, "Parent".to_string(), None, None, None, None).unwrap();
    let child1_id =
        ettle_ops::create_ettle(&mut store, "Child1".to_string(), None, None, None, None).unwrap();
    let child2_id =
        ettle_ops::create_ettle(&mut store, "Child2".to_string(), None, None, None, None).unwrap();
    let child3_id =
        ettle_ops::create_ettle(&mut store, "Child3".to_string(), None, None, None, None).unwrap();

    // Create EPs out of order: EP3, EP1, EP2
    let ep3_id = ep_ops::create_ep(
        &mut store,
        &parent_id,
        3,
        true,
        String::new(),
        String::new(),
        String::new(),
    )
    .unwrap();
    let ep1_id = ep_ops::create_ep(
        &mut store,
        &parent_id,
        1,
        true,
        String::new(),
        String::new(),
        String::new(),
    )
    .unwrap();
    let ep2_id = ep_ops::create_ep(
        &mut store,
        &parent_id,
        2,
        true,
        String::new(),
        String::new(),
        String::new(),
    )
    .unwrap();

    // Link children
    refinement_ops::link_child(&mut store, &ep1_id, &child1_id).unwrap();
    refinement_ops::link_child(&mut store, &ep2_id, &child2_id).unwrap();
    refinement_ops::link_child(&mut store, &ep3_id, &child3_id).unwrap();

    let children = refinement_ops::list_children(&store, &parent_id).unwrap();

    assert_eq!(children.len(), 3);
    assert_eq!(children[0], child1_id);
    assert_eq!(children[1], child2_id);
    assert_eq!(children[2], child3_id);
}

#[test]
fn test_list_children_returns_empty_for_leaf() {
    let mut store = new_store();
    let ettle_id =
        ettle_ops::create_ettle(&mut store, "Leaf".to_string(), None, None, None, None).unwrap();

    let children = refinement_ops::list_children(&store, &ettle_id).unwrap();

    assert_eq!(children.len(), 0);
}

#[test]
fn test_list_children_skips_deleted_eps() {
    let mut store = new_store();
    let parent_id =
        ettle_ops::create_ettle(&mut store, "Parent".to_string(), None, None, None, None).unwrap();
    let child1_id =
        ettle_ops::create_ettle(&mut store, "Child1".to_string(), None, None, None, None).unwrap();
    let child2_id =
        ettle_ops::create_ettle(&mut store, "Child2".to_string(), None, None, None, None).unwrap();

    let ep1_id = ep_ops::create_ep(
        &mut store,
        &parent_id,
        1,
        true,
        String::new(),
        String::new(),
        String::new(),
    )
    .unwrap();
    let ep2_id = ep_ops::create_ep(
        &mut store,
        &parent_id,
        2,
        true,
        String::new(),
        String::new(),
        String::new(),
    )
    .unwrap();

    refinement_ops::link_child(&mut store, &ep1_id, &child1_id).unwrap();
    refinement_ops::link_child(&mut store, &ep2_id, &child2_id).unwrap();

    // Delete child2's EP
    refinement_ops::unlink_child(&mut store, &ep2_id).unwrap();
    ep_ops::delete_ep(&mut store, &ep2_id).unwrap();

    let children = refinement_ops::list_children(&store, &parent_id).unwrap();

    assert_eq!(children.len(), 1);
    assert_eq!(children[0], child1_id);
}
