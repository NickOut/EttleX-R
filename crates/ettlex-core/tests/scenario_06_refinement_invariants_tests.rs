/// Scenario 6: Refinement Invariants
///
/// Tests refinement tree invariants including EP mappings and child relationships.
use ettlex_core::errors::EttleXError;
use ettlex_core::model::Ep;
use ettlex_core::ops::{ep_ops, ettle_ops, refinement_ops, Store};
use ettlex_core::rules::{invariants, validation};

#[test]
fn test_scenario_06_happy_valid_parent_child_via_ep_mapping() {
    // GIVEN a parent and child with proper EP mapping
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

    // WHEN linking child to EP
    refinement_ops::link_child(&mut store, &ep1_id, &child_id).expect("Should link child");

    // THEN validation passes
    let result = validation::validate_tree(&store);
    assert!(result.is_ok());

    // AND no invariant violations
    assert!(invariants::find_children_without_ep_mapping(&store).is_empty());
    assert!(invariants::find_duplicate_child_mappings(&store).is_empty());
}

#[test]
fn test_scenario_06_error_child_without_ep_mapping() {
    // GIVEN a child with parent_id set but no EP mapping
    let mut store = Store::new();
    let parent_id =
        ettle_ops::create_ettle(&mut store, "Parent".to_string(), None, None, None, None)
            .expect("Should create parent");

    let child_id = ettle_ops::create_ettle(&mut store, "Child".to_string(), None, None, None, None)
        .expect("Should create child");

    // Manually set parent_id without EP mapping
    {
        let child = store.get_ettle_mut(&child_id).expect("Child exists");
        child.parent_id = Some(parent_id.clone());
    }

    // WHEN checking invariants
    let missing = invariants::find_children_without_ep_mapping(&store);

    // THEN child without mapping is detected
    assert_eq!(missing.len(), 1);
    assert_eq!(missing[0].0, child_id);
    assert_eq!(missing[0].1, parent_id);

    // AND validate_tree fails
    let result = validation::validate_tree(&store);
    assert!(result.is_err());
    assert!(matches!(
        result,
        Err(EttleXError::ChildWithoutEpMapping { .. })
    ));
}

#[test]
fn test_scenario_06_error_duplicate_mappings() {
    // GIVEN a child mapped by two EPs
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

    // Link both EPs to same child (manually bypass validation)
    refinement_ops::link_child(&mut store, &ep1_id, &child_id).expect("Should link to EP1");

    // Manually set EP2's child (bypassing one-to-one validation for test)
    {
        let ep2 = store.get_ep_mut(&ep2_id).expect("EP2 exists");
        ep2.child_ettle_id = Some(child_id.clone());
    }

    // WHEN checking for duplicates
    let duplicates = invariants::find_duplicate_child_mappings(&store);

    // THEN duplicate mapping is detected
    assert_eq!(duplicates.len(), 1);
    assert_eq!(duplicates[0].0, child_id);

    // AND validate_tree fails
    let result = validation::validate_tree(&store);
    assert!(result.is_err());
    assert!(matches!(
        result,
        Err(EttleXError::ChildReferencedByMultipleEps { .. })
    ));
}

#[test]
fn test_scenario_06_error_mapping_references_deleted_ep() {
    // GIVEN a deleted EP with a child mapping
    let mut store = Store::new();
    let parent_id =
        ettle_ops::create_ettle(&mut store, "Parent".to_string(), None, None, None, None)
            .expect("Should create parent");

    let child_id = ettle_ops::create_ettle(&mut store, "Child".to_string(), None, None, None, None)
        .expect("Should create child");

    // Create deleted EP with child mapping (manually)
    let mut ep = Ep::new(
        "ep-deleted".to_string(),
        parent_id.clone(),
        1,
        false,
        "".to_string(),
        "".to_string(),
        "".to_string(),
    );
    ep.deleted = true;
    ep.child_ettle_id = Some(child_id.clone());

    {
        let parent = store.get_ettle_mut(&parent_id).expect("Parent exists");
        parent.add_ep_id("ep-deleted".to_string());
    }

    store.insert_ep(ep);

    // WHEN checking for deleted EP mappings
    let deleted_mappings = invariants::find_deleted_ep_mappings(&store);

    // THEN deleted EP with mapping is detected
    assert_eq!(deleted_mappings.len(), 1);
    assert_eq!(deleted_mappings[0], "ep-deleted");

    // AND validate_tree fails
    let result = validation::validate_tree(&store);
    assert!(result.is_err());
    assert!(matches!(
        result,
        Err(EttleXError::MappingReferencesDeletedEp { .. })
    ));
}

#[test]
fn test_scenario_06_error_mapping_to_deleted_child() {
    // GIVEN an active EP mapping to a deleted child
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

    // Link child then delete it (manually)
    refinement_ops::link_child(&mut store, &ep1_id, &child_id).expect("Should link child");

    {
        let child = store.get_ettle_mut(&child_id).expect("Child exists");
        child.deleted = true;
    }

    // WHEN checking for deleted child mappings
    let deleted_child_mappings = invariants::find_deleted_child_mappings(&store);

    // THEN deleted child mapping is detected
    assert_eq!(deleted_child_mappings.len(), 1);
    assert_eq!(deleted_child_mappings[0].0, ep1_id);
    assert_eq!(deleted_child_mappings[0].1, child_id);

    // AND validate_tree fails
    let result = validation::validate_tree(&store);
    assert!(result.is_err());
    assert!(matches!(
        result,
        Err(EttleXError::MappingReferencesDeletedChild { .. })
    ));
}
