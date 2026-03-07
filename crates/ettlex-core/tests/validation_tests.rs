mod common;

use common::{create_test_ep, create_test_ettle, new_store};
use ettlex_core::{
    ops::{ep_ops, ettle_ops, refinement_ops},
    rules::validation,
};

// ===== VALIDATE_TREE SUCCESS TESTS =====

#[test]
fn test_validate_tree_succeeds_on_empty_store() {
    let store = new_store();
    assert!(validation::validate_tree(&store).is_ok());
}

#[test]
fn test_validate_tree_succeeds_on_single_ettle() {
    let mut store = new_store();
    ettle_ops::create_ettle(&mut store, "Single".to_string(), None, None, None, None).unwrap();

    assert!(validation::validate_tree(&store).is_ok());
}

#[test]
fn test_validate_tree_succeeds_on_valid_tree() {
    let mut store = new_store();

    let root =
        ettle_ops::create_ettle(&mut store, "Root".to_string(), None, None, None, None).unwrap();
    let child1 =
        ettle_ops::create_ettle(&mut store, "Child1".to_string(), None, None, None, None).unwrap();
    let child2 =
        ettle_ops::create_ettle(&mut store, "Child2".to_string(), None, None, None, None).unwrap();

    let ep1 = ep_ops::create_ep(
        &mut store,
        &root,
        1,
        true,
        String::new(),
        String::new(),
        String::new(),
    )
    .unwrap();
    let ep2 = ep_ops::create_ep(
        &mut store,
        &root,
        2,
        true,
        String::new(),
        String::new(),
        String::new(),
    )
    .unwrap();

    refinement_ops::link_child(&mut store, &ep1, &child1).unwrap();
    refinement_ops::link_child(&mut store, &ep2, &child2).unwrap();

    assert!(validation::validate_tree(&store).is_ok());
}

// ===== CYCLE DETECTION TESTS =====

#[test]
fn test_validate_tree_detects_cycle_via_direct_manipulation() {
    let mut store = new_store();

    // Create cycle manually (bypassing refinement_ops which prevents this)
    let a_id = create_test_ettle(&mut store, "A");
    let b_id = create_test_ettle(&mut store, "B");

    let a = store.get_ettle_mut(&a_id).unwrap();
    a.parent_id = Some(b_id.clone());

    let b = store.get_ettle_mut(&b_id).unwrap();
    b.parent_id = Some(a_id.clone());

    let result = validation::validate_tree(&store);
    assert!(result.is_err());
}

// ===== ORPHAN DETECTION TESTS =====

#[test]
fn test_validate_tree_detects_orphaned_ettle() {
    let mut store = new_store();

    let child_id = create_test_ettle(&mut store, "Child");

    // Set parent_id to nonexistent ettle (bypass refinement_ops)
    let child = store.get_ettle_mut(&child_id).unwrap();
    child.parent_id = Some("nonexistent-parent".to_string());

    let result = validation::validate_tree(&store);
    assert!(result.is_err());
}

#[test]
fn test_validate_tree_detects_child_without_ep_mapping() {
    let mut store = new_store();

    let parent_id =
        ettle_ops::create_ettle(&mut store, "Parent".to_string(), None, None, None, None).unwrap();
    let child_id =
        ettle_ops::create_ettle(&mut store, "Child".to_string(), None, None, None, None).unwrap();

    // Set child's parent_id but don't create EP mapping (bypass refinement_ops)
    let child = store.get_ettle_mut(&child_id).unwrap();
    child.parent_id = Some(parent_id.clone());

    let result = validation::validate_tree(&store);
    assert!(result.is_err());
    // Should be ChildWithoutEpMapping error
}

// ===== EP VALIDATION TESTS =====

#[test]
fn test_validate_tree_detects_duplicate_ordinal() {
    let mut store = new_store();

    let ettle_id = create_test_ettle(&mut store, "Ettle");

    // Create two EPs with same ordinal (bypass ep_ops)
    let _ep1_id = create_test_ep(&mut store, &ettle_id, 1, true, "", "", "");
    let _ep2_id = create_test_ep(&mut store, &ettle_id, 1, true, "", "", "");

    // Note: create_test_ep adds to ep_ids, so we now have duplicate ordinals
    let result = validation::validate_tree(&store);
    assert!(result.is_err());
}

// In the new model, multiple child_ettle_id entries pointing to the same child
// via the legacy field do NOT create a violation. Each child has one authoritative
// parent_ep_id. validate_tree should PASS in this scenario.
#[test]
fn test_validate_tree_multiple_child_ettle_id_entries_are_valid() {
    let mut store = new_store();

    let parent_id =
        ettle_ops::create_ettle(&mut store, "Parent".to_string(), None, None, None, None).unwrap();
    let child_id =
        ettle_ops::create_ettle(&mut store, "Child".to_string(), None, None, None, None).unwrap();

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

    // Link child to ep1 properly (sets parent_ep_id = ep1_id, parent_id = parent_id)
    refinement_ops::link_child(&mut store, &ep1_id, &child_id).unwrap();

    // Manually set ep2's child_ettle_id (legacy field, non-authoritative)
    let ep2 = store.get_ep_mut(&ep2_id).unwrap();
    ep2.child_ettle_id = Some(child_id.clone());

    // validate_tree should PASS — child.parent_ep_id is ep1_id (unambiguous)
    let result = validation::validate_tree(&store);
    assert!(
        result.is_ok(),
        "Multiple child_ettle_id entries are now valid: {:?}",
        result.err()
    );
}

#[test]
fn test_validate_tree_detects_ettle_with_nonexistent_parent_ep_id() {
    let mut store = new_store();

    let parent_id =
        ettle_ops::create_ettle(&mut store, "Parent".to_string(), None, None, None, None).unwrap();
    let child_id =
        ettle_ops::create_ettle(&mut store, "Child".to_string(), None, None, None, None).unwrap();

    // Manually set child's parent_ep_id to a nonexistent EP (inconsistent state)
    let child = store.get_ettle_mut(&child_id).unwrap();
    child.parent_id = Some(parent_id.clone());
    child.parent_ep_id = Some("nonexistent-ep".to_string());

    let result = validation::validate_tree(&store);
    assert!(result.is_err());
}

// ===== EDGE CASE TESTS =====

#[test]
fn test_validate_tree_ignores_deleted_ettles() {
    let mut store = new_store();

    let ettle_id =
        ettle_ops::create_ettle(&mut store, "Test".to_string(), None, None, None, None).unwrap();
    ettle_ops::delete_ettle(&mut store, &ettle_id).unwrap();

    // Deleted ettles should be ignored by validation
    assert!(validation::validate_tree(&store).is_ok());
}

#[test]
fn test_validate_tree_ignores_deleted_eps() {
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

    ep_ops::delete_ep(&mut store, &ep_id).unwrap();

    // Deleted EPs should be ignored
    assert!(validation::validate_tree(&store).is_ok());
}

#[test]
fn test_validate_tree_allows_multiple_roots() {
    let mut store = new_store();

    ettle_ops::create_ettle(&mut store, "Root1".to_string(), None, None, None, None).unwrap();
    ettle_ops::create_ettle(&mut store, "Root2".to_string(), None, None, None, None).unwrap();
    ettle_ops::create_ettle(&mut store, "Root3".to_string(), None, None, None, None).unwrap();

    // Multiple roots (forest) is valid
    assert!(validation::validate_tree(&store).is_ok());
}

#[test]
fn test_validate_tree_allows_leaf_eps() {
    let mut store = new_store();

    let ettle_id =
        ettle_ops::create_ettle(&mut store, "Test".to_string(), None, None, None, None).unwrap();

    // Create additional EPs that don't have children (leaf EPs)
    ep_ops::create_ep(
        &mut store,
        &ettle_id,
        1,
        true,
        String::new(),
        String::new(),
        String::new(),
    )
    .unwrap();
    ep_ops::create_ep(
        &mut store,
        &ettle_id,
        2,
        false,
        String::new(),
        String::new(),
        String::new(),
    )
    .unwrap();

    // Leaf EPs are valid
    assert!(validation::validate_tree(&store).is_ok());
}
