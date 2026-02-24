//! Decision linking test suite
//!
//! Tests scenarios 8, 14-21 from seed_decision_schema_stubs_v2.yaml
//! Phase 2: Linking & Supersession with validation

use ettlex_core::ops::Store;
use ettlex_core::ops::{decision_ops, ep_ops, ettle_ops};

fn setup_store_with_ep() -> (Store, String, String) {
    let mut store = Store::new();

    let ettle_id = ettle_ops::create_ettle(
        &mut store,
        "Test Ettle".to_string(),
        None,
        None,
        Some("what".to_string()),
        Some("how".to_string()),
    )
    .unwrap();

    let ep_id = ep_ops::create_ep(
        &mut store,
        &ettle_id,
        1,
        false,
        "why".to_string(),
        "what".to_string(),
        "how".to_string(),
    )
    .unwrap();

    (store, ettle_id, ep_id)
}

fn create_test_decision(store: &mut Store, decision_id: Option<String>) -> String {
    decision_ops::create_decision(
        store,
        decision_id,
        "Test Decision".to_string(),
        None,
        "Decision text".to_string(),
        "Rationale".to_string(),
        None,
        None,
        "none".to_string(),
        None,
        None,
        None,
    )
    .unwrap()
}

// Scenario 8: Tombstone decision prevents new linking by default
#[test]
fn test_scenario_8_tombstone_prevents_new_linking() {
    let (mut store, _ettle_id, ep_id) = setup_store_with_ep();

    let decision_id = create_test_decision(&mut store, Some("d:002".to_string()));

    // Tombstone the decision
    decision_ops::tombstone_decision(&mut store, &decision_id).unwrap();

    // Verify it's tombstoned
    let result = store.get_decision(&decision_id);
    assert!(result.is_err());
    assert!(matches!(
        result,
        Err(ettlex_core::errors::EttleXError::DecisionDeleted { .. })
    ));

    // Try to link tombstoned decision - should fail
    let result = decision_ops::attach_decision_to_target(
        &mut store,
        &decision_id,
        "ep".to_string(),
        ep_id,
        "grounds".to_string(),
        0,
    );

    assert!(result.is_err());
    assert!(matches!(
        result,
        Err(ettlex_core::errors::EttleXError::DecisionTombstoned { .. })
    ));
}

// Scenario 14: Link decision to EP with deterministic ordering
#[test]
fn test_scenario_14_link_with_deterministic_ordering() {
    let (mut store, _ettle_id, ep_id) = setup_store_with_ep();

    let d010 = create_test_decision(&mut store, Some("d:010".to_string()));
    let d011 = create_test_decision(&mut store, Some("d:011".to_string()));

    // Link d:010 with ordinal 1
    decision_ops::attach_decision_to_target(
        &mut store,
        &d010,
        "ep".to_string(),
        ep_id.clone(),
        "grounds".to_string(),
        1,
    )
    .unwrap();

    // Link d:011 with ordinal 0 (should come first)
    decision_ops::attach_decision_to_target(
        &mut store,
        &d011,
        "ep".to_string(),
        ep_id.clone(),
        "grounds".to_string(),
        0,
    )
    .unwrap();

    // Query decisions for EP - should be ordered by ordinal
    let links = store.list_decision_links_for_target("ep", &ep_id);
    assert_eq!(links.len(), 2);

    // Sort by ordinal to verify ordering
    let mut sorted_links = links.clone();
    sorted_links.sort_by_key(|link| link.ordinal);

    assert_eq!(sorted_links[0].decision_id, "d:011"); // ordinal 0
    assert_eq!(sorted_links[1].decision_id, "d:010"); // ordinal 1
}

// Scenario 15: Duplicate link is rejected
#[test]
fn test_scenario_15_duplicate_link_rejected() {
    let (mut store, _ettle_id, ep_id) = setup_store_with_ep();

    let decision_id = create_test_decision(&mut store, Some("d:010".to_string()));

    // Link once - should succeed
    decision_ops::attach_decision_to_target(
        &mut store,
        &decision_id,
        "ep".to_string(),
        ep_id.clone(),
        "grounds".to_string(),
        0,
    )
    .unwrap();

    // Try to link again with same relation - should fail
    let result = decision_ops::attach_decision_to_target(
        &mut store,
        &decision_id,
        "ep".to_string(),
        ep_id,
        "grounds".to_string(),
        0,
    );

    assert!(result.is_err());
    assert!(matches!(
        result,
        Err(ettlex_core::errors::EttleXError::DuplicateDecisionLink { .. })
    ));
}

// Scenario 16: Unlink removes link but preserves decision history
#[test]
fn test_scenario_16_unlink_preserves_decision() {
    let (mut store, _ettle_id, ep_id) = setup_store_with_ep();

    let decision_id = create_test_decision(&mut store, Some("d:010".to_string()));

    // Link decision to EP
    decision_ops::attach_decision_to_target(
        &mut store,
        &decision_id,
        "ep".to_string(),
        ep_id.clone(),
        "grounds".to_string(),
        0,
    )
    .unwrap();

    // Verify link exists
    let links = store.list_decision_links_for_target("ep", &ep_id);
    assert_eq!(links.len(), 1);

    // Unlink
    decision_ops::detach_decision_from_target(&mut store, &decision_id, "ep", &ep_id, "grounds")
        .unwrap();

    // Verify link is gone
    let links = store.list_decision_links_for_target("ep", &ep_id);
    assert_eq!(links.len(), 0);

    // Verify decision still exists
    assert!(store.get_decision(&decision_id).is_ok());
}

// Scenario 17: Link rejects unknown decision id
#[test]
fn test_scenario_17_link_rejects_unknown_decision() {
    let (mut store, _ettle_id, ep_id) = setup_store_with_ep();

    let result = decision_ops::attach_decision_to_target(
        &mut store,
        "d:missing",
        "ep".to_string(),
        ep_id,
        "grounds".to_string(),
        0,
    );

    assert!(result.is_err());
    assert!(matches!(
        result,
        Err(ettlex_core::errors::EttleXError::DecisionNotFound { .. })
    ));
}

// Scenario 18: Link rejects unknown EP id
#[test]
fn test_scenario_18_link_rejects_unknown_ep() {
    let mut store = Store::new();

    let decision_id = create_test_decision(&mut store, Some("d:010".to_string()));

    let result = decision_ops::attach_decision_to_target(
        &mut store,
        &decision_id,
        "ep".to_string(),
        "ep:missing".to_string(),
        "grounds".to_string(),
        0,
    );

    assert!(result.is_err());
    assert!(matches!(
        result,
        Err(ettlex_core::errors::EttleXError::EpNotFound { .. })
    ));
}

// Scenario 19: Link rejects unknown target_kind unless explicitly allowed
#[test]
fn test_scenario_19_link_rejects_invalid_target_kind() {
    let mut store = Store::new();

    let decision_id = create_test_decision(&mut store, Some("d:010".to_string()));

    let result = decision_ops::attach_decision_to_target(
        &mut store,
        &decision_id,
        "weird".to_string(),
        "x".to_string(),
        "grounds".to_string(),
        0,
    );

    assert!(result.is_err());
    assert!(matches!(
        result,
        Err(ettlex_core::errors::EttleXError::InvalidTargetKind { .. })
    ));
}

// Scenario 20: Supersede creates a deterministic supersedes link
#[test]
fn test_scenario_20_supersede_creates_link() {
    let mut store = Store::new();

    let old_id = create_test_decision(&mut store, Some("d:100".to_string()));
    let new_id = create_test_decision(&mut store, Some("d:101".to_string()));

    // Supersede old with new
    decision_ops::supersede_decision(&mut store, &old_id, &new_id).unwrap();

    // Verify link exists with relation_kind="supersedes"
    let links = store.list_decision_links_for_target("decision", &new_id);
    assert_eq!(links.len(), 1);
    assert_eq!(links[0].decision_id, old_id);
    assert_eq!(links[0].relation_kind, "supersedes");
}

// Scenario 21: Supersede does not tombstone the old decision
#[test]
fn test_scenario_21_supersede_preserves_old_decision() {
    let mut store = Store::new();

    let old_id = create_test_decision(&mut store, Some("d:100".to_string()));
    let new_id = create_test_decision(&mut store, Some("d:101".to_string()));

    // Supersede old with new
    decision_ops::supersede_decision(&mut store, &old_id, &new_id).unwrap();

    // Verify old decision still exists and is not tombstoned
    let old_decision = store.get_decision(&old_id).unwrap();
    assert!(!old_decision.is_tombstoned());
    assert_eq!(old_decision.decision_id, old_id);
}
