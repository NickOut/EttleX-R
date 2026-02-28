//! Apply Command Coverage Tests
//!
//! This test suite verifies that all Phase 0.5 operations are correctly
//! exposed through the Command enum and apply() function.
//!
//! ## Scenarios Covered
//!
//! 1. Ettle operations (Create, Update, Delete)
//! 2. EP operations (Create, Update, Delete)
//! 3. Refinement operations (Link, Unlink)
//! 4. All error paths for each command type

use ettlex_core::{
    apply,
    ops::{decision_ops, ep_ops, ettle_ops, refinement_ops},
    policy::NeverAnchoredPolicy,
    Command, EttleXError, Metadata, Store,
};

#[test]
fn test_command_ettle_create_with_metadata() {
    let state = Store::new();

    let mut metadata = Metadata::new();
    metadata.set("author".to_string(), serde_json::json!("Test Author"));
    metadata.set("version".to_string(), serde_json::json!("1.0"));
    metadata.set("tags".to_string(), serde_json::json!(["test", "demo"]));

    let cmd = Command::EttleCreate {
        title: "Test Ettle".to_string(),
        metadata: Some(metadata.clone()),
        why: Some("Why content".to_string()),
        what: Some("What content".to_string()),
        how: Some("How content".to_string()),
    };

    let policy = NeverAnchoredPolicy;
    let new_state = apply(state, cmd, &policy).unwrap();

    let ettles = new_state.list_ettles();
    assert_eq!(ettles.len(), 1);

    let ettle = &ettles[0];
    assert_eq!(ettle.title, "Test Ettle");
    assert_eq!(
        ettle.metadata.get("author"),
        Some(&serde_json::json!("Test Author"))
    );
    assert_eq!(
        ettle.metadata.get("version"),
        Some(&serde_json::json!("1.0"))
    );

    // Verify EP0 has the content
    let ep0 = new_state.get_ep(&ettle.ep_ids[0]).unwrap();
    assert_eq!(ep0.why, "Why content");
    assert_eq!(ep0.what, "What content");
    assert_eq!(ep0.how, "How content");
}

#[test]
fn test_command_ettle_update() {
    let mut state = Store::new();
    let ettle_id = ettle_ops::create_ettle(
        &mut state,
        "Original Title".to_string(),
        None,
        None,
        None,
        None,
    )
    .unwrap();

    let mut new_metadata = Metadata::new();
    new_metadata.set("author".to_string(), serde_json::json!("Updated Author"));
    new_metadata.set("version".to_string(), serde_json::json!("2.0"));

    let cmd = Command::EttleUpdate {
        ettle_id: ettle_id.clone(),
        title: Some("Updated Title".to_string()),
        metadata: Some(new_metadata),
    };

    let policy = NeverAnchoredPolicy;
    let new_state = apply(state, cmd, &policy).unwrap();

    let ettle = new_state.get_ettle(&ettle_id).unwrap();
    assert_eq!(ettle.title, "Updated Title");
    assert_eq!(
        ettle.metadata.get("author"),
        Some(&serde_json::json!("Updated Author"))
    );
}

#[test]
fn test_command_ettle_delete() {
    let mut state = Store::new();
    let ettle_id =
        ettle_ops::create_ettle(&mut state, "To Delete".to_string(), None, None, None, None)
            .unwrap();

    let cmd = Command::EttleDelete {
        ettle_id: ettle_id.clone(),
    };

    let policy = NeverAnchoredPolicy;
    let new_state = apply(state, cmd, &policy).unwrap();

    // Ettle should be tombstoned
    let result = new_state.get_ettle(&ettle_id);
    assert!(matches!(result, Err(EttleXError::EttleDeleted { .. })));
}

#[test]
fn test_command_ep_create() {
    let mut state = Store::new();
    let ettle_id =
        ettle_ops::create_ettle(&mut state, "Test Ettle".to_string(), None, None, None, None)
            .unwrap();

    let cmd = Command::EpCreate {
        ettle_id: ettle_id.clone(),
        ordinal: 1,
        normative: true,
        why: "Test why".to_string(),
        what: "Test what".to_string(),
        how: "Test how".to_string(),
    };

    let policy = NeverAnchoredPolicy;
    let new_state = apply(state, cmd, &policy).unwrap();

    let ettle = new_state.get_ettle(&ettle_id).unwrap();
    assert_eq!(ettle.ep_ids.len(), 2); // EP0 + EP1

    // Find the new EP (ordinal 1)
    let ep1 = ettle
        .ep_ids
        .iter()
        .filter_map(|id| new_state.get_ep(id).ok())
        .find(|ep| ep.ordinal == 1)
        .unwrap();

    assert_eq!(ep1.why, "Test why");
    assert_eq!(ep1.what, "Test what");
    assert_eq!(ep1.how, "Test how");
    assert!(ep1.normative);
}

#[test]
fn test_command_ep_update() {
    let mut state = Store::new();
    let ettle_id =
        ettle_ops::create_ettle(&mut state, "Test".to_string(), None, None, None, None).unwrap();
    let ep_id = ep_ops::create_ep(
        &mut state,
        &ettle_id,
        1,
        true,
        "Original why".to_string(),
        "Original what".to_string(),
        "Original how".to_string(),
    )
    .unwrap();

    let cmd = Command::EpUpdate {
        ep_id: ep_id.clone(),
        why: Some("Updated why".to_string()),
        what: Some("Updated what".to_string()),
        how: Some("Updated how".to_string()),
        normative: Some(false),
    };

    let policy = NeverAnchoredPolicy;
    let new_state = apply(state, cmd, &policy).unwrap();

    let ep = new_state.get_ep(&ep_id).unwrap();
    assert_eq!(ep.why, "Updated why");
    assert_eq!(ep.what, "Updated what");
    assert_eq!(ep.how, "Updated how");
    assert!(!ep.normative);
}

#[test]
fn test_command_ep_update_partial() {
    let mut state = Store::new();
    let ettle_id =
        ettle_ops::create_ettle(&mut state, "Test".to_string(), None, None, None, None).unwrap();
    let ep_id = ep_ops::create_ep(
        &mut state,
        &ettle_id,
        1,
        true,
        "Original why".to_string(),
        "Original what".to_string(),
        "Original how".to_string(),
    )
    .unwrap();

    // Update only WHAT field
    let cmd = Command::EpUpdate {
        ep_id: ep_id.clone(),
        why: None,
        what: Some("Updated what".to_string()),
        how: None,
        normative: None,
    };

    let policy = NeverAnchoredPolicy;
    let new_state = apply(state, cmd, &policy).unwrap();

    let ep = new_state.get_ep(&ep_id).unwrap();
    assert_eq!(ep.why, "Original why"); // Unchanged
    assert_eq!(ep.what, "Updated what"); // Changed
    assert_eq!(ep.how, "Original how"); // Unchanged
    assert!(ep.normative); // Unchanged
}

#[test]
fn test_command_refine_link_child() {
    let mut state = Store::new();
    let parent_id =
        ettle_ops::create_ettle(&mut state, "Parent".to_string(), None, None, None, None).unwrap();
    let child_id =
        ettle_ops::create_ettle(&mut state, "Child".to_string(), None, None, None, None).unwrap();

    let parent = state.get_ettle(&parent_id).unwrap();
    let parent_ep0_id = parent.ep_ids[0].clone();

    let cmd = Command::RefineLinkChild {
        parent_ep_id: parent_ep0_id.clone(),
        child_ettle_id: child_id.clone(),
    };

    let policy = NeverAnchoredPolicy;
    let new_state = apply(state, cmd, &policy).unwrap();

    // Verify EP points to child
    let ep = new_state.get_ep(&parent_ep0_id).unwrap();
    assert_eq!(ep.child_ettle_id, Some(child_id.clone()));

    // Verify child points to parent
    let child = new_state.get_ettle(&child_id).unwrap();
    assert_eq!(child.parent_id, Some(parent_id));
}

#[test]
fn test_command_refine_unlink_child() {
    let mut state = Store::new();
    let parent_id =
        ettle_ops::create_ettle(&mut state, "Parent".to_string(), None, None, None, None).unwrap();
    let child_id =
        ettle_ops::create_ettle(&mut state, "Child".to_string(), None, None, None, None).unwrap();

    let parent = state.get_ettle(&parent_id).unwrap();
    let parent_ep0_id = parent.ep_ids[0].clone();

    // First, link them
    refinement_ops::link_child(&mut state, &parent_ep0_id, &child_id).unwrap();

    // Verify they're linked
    let ep = state.get_ep(&parent_ep0_id).unwrap();
    assert_eq!(ep.child_ettle_id, Some(child_id.clone()));

    // Now unlink via command
    let cmd = Command::RefineUnlinkChild {
        parent_ep_id: parent_ep0_id.clone(),
    };

    let policy = NeverAnchoredPolicy;
    let new_state = apply(state, cmd, &policy).unwrap();

    // Verify EP no longer points to child
    let ep = new_state.get_ep(&parent_ep0_id).unwrap();
    assert_eq!(ep.child_ettle_id, None);

    // Verify child no longer points to parent
    let child = new_state.get_ettle(&child_id).unwrap();
    assert_eq!(child.parent_id, None);
}

#[test]
fn test_command_error_ettle_create_invalid_title() {
    let state = Store::new();

    let cmd = Command::EttleCreate {
        title: "".to_string(), // Invalid
        metadata: None,
        why: None,
        what: None,
        how: None,
    };

    let policy = NeverAnchoredPolicy;
    let result = apply(state, cmd, &policy);

    assert!(matches!(result, Err(EttleXError::InvalidTitle { .. })));
}

#[test]
fn test_command_error_ep_create_invalid_what() {
    let mut state = Store::new();
    let ettle_id =
        ettle_ops::create_ettle(&mut state, "Test".to_string(), None, None, None, None).unwrap();

    let cmd = Command::EpCreate {
        ettle_id,
        ordinal: 1,
        normative: true,
        why: String::new(),
        what: "  ".to_string(), // Invalid: whitespace-only
        how: String::new(),
    };

    let policy = NeverAnchoredPolicy;
    let result = apply(state, cmd, &policy);

    assert!(matches!(result, Err(EttleXError::InvalidWhat { .. })));
}

#[test]
fn test_command_error_refine_link_child_already_has_parent() {
    let mut state = Store::new();
    let parent1_id =
        ettle_ops::create_ettle(&mut state, "Parent 1".to_string(), None, None, None, None)
            .unwrap();
    let parent2_id =
        ettle_ops::create_ettle(&mut state, "Parent 2".to_string(), None, None, None, None)
            .unwrap();
    let child_id =
        ettle_ops::create_ettle(&mut state, "Child".to_string(), None, None, None, None).unwrap();

    // Link child to parent1
    let parent1 = state.get_ettle(&parent1_id).unwrap();
    let parent1_ep0_id = parent1.ep_ids[0].clone();
    refinement_ops::link_child(&mut state, &parent1_ep0_id, &child_id).unwrap();

    // Try to link child to parent2 (should fail)
    let parent2 = state.get_ettle(&parent2_id).unwrap();
    let parent2_ep0_id = parent2.ep_ids[0].clone();

    let cmd = Command::RefineLinkChild {
        parent_ep_id: parent2_ep0_id,
        child_ettle_id: child_id,
    };

    let policy = NeverAnchoredPolicy;
    let result = apply(state, cmd, &policy);

    assert!(matches!(
        result,
        Err(EttleXError::ChildAlreadyHasParent { .. })
    ));
}

// ---------------------------------------------------------------------------
// Decision command coverage
// ---------------------------------------------------------------------------

#[test]
fn test_command_decision_create_via_apply() {
    let state = Store::new();
    let policy = NeverAnchoredPolicy;

    let new_state = apply(
        state,
        Command::DecisionCreate {
            decision_id: Some("decision:001".to_string()),
            title: "Adopt event sourcing".to_string(),
            status: Some("proposed".to_string()),
            decision_text: "We adopt event sourcing for all state changes.".to_string(),
            rationale: "Enables auditability and replay.".to_string(),
            alternatives_text: None,
            consequences_text: None,
            evidence_kind: "none".to_string(),
            evidence_excerpt: None,
            evidence_capture_content: None,
            evidence_file_path: None,
        },
        &policy,
    )
    .unwrap();

    let decision = new_state.get_decision("decision:001").unwrap();
    assert_eq!(decision.title, "Adopt event sourcing");
}

#[test]
fn test_command_decision_update_via_apply() {
    let mut state = Store::new();
    let policy = NeverAnchoredPolicy;

    decision_ops::create_decision(
        &mut state,
        Some("decision:upd".to_string()),
        "Original".to_string(),
        None,
        "Body.".to_string(),
        "Rationale.".to_string(),
        None,
        None,
        "none".to_string(),
        None,
        None,
        None,
    )
    .unwrap();

    let new_state = apply(
        state,
        Command::DecisionUpdate {
            decision_id: "decision:upd".to_string(),
            title: Some("Updated".to_string()),
            status: None,
            decision_text: None,
            rationale: None,
            alternatives_text: None,
            consequences_text: None,
            evidence_kind: None,
            evidence_excerpt: None,
            evidence_capture_content: None,
            evidence_file_path: None,
        },
        &policy,
    )
    .unwrap();

    let decision = new_state.get_decision("decision:upd").unwrap();
    assert_eq!(decision.title, "Updated");
}

#[test]
fn test_command_decision_tombstone_via_apply() {
    let mut state = Store::new();
    let policy = NeverAnchoredPolicy;

    decision_ops::create_decision(
        &mut state,
        Some("decision:tomb2".to_string()),
        "To be tombstoned".to_string(),
        None,
        "Body.".to_string(),
        "Rationale.".to_string(),
        None,
        None,
        "none".to_string(),
        None,
        None,
        None,
    )
    .unwrap();

    let new_state = apply(
        state,
        Command::DecisionTombstone {
            decision_id: "decision:tomb2".to_string(),
        },
        &policy,
    )
    .unwrap();

    // After tombstone, get_decision should fail
    assert!(new_state.get_decision("decision:tomb2").is_err());
}

#[test]
fn test_command_decision_link_and_unlink_via_apply() {
    let mut state = Store::new();
    let policy = NeverAnchoredPolicy;

    let ettle_id =
        ettle_ops::create_ettle(&mut state, "Root".to_string(), None, None, None, None).unwrap();
    let ep_id = state.get_ettle(&ettle_id).unwrap().ep_ids[0].clone();

    decision_ops::create_decision(
        &mut state,
        Some("decision:lnk2".to_string()),
        "Link test".to_string(),
        None,
        "Body.".to_string(),
        "Rationale.".to_string(),
        None,
        None,
        "none".to_string(),
        None,
        None,
        None,
    )
    .unwrap();

    let new_state = apply(
        state,
        Command::DecisionLink {
            decision_id: "decision:lnk2".to_string(),
            target_kind: "ep".to_string(),
            target_id: ep_id.clone(),
            relation_kind: "grounds".to_string(),
            ordinal: 0,
        },
        &policy,
    )
    .unwrap();

    let linked = new_state.get_decision_link("decision:lnk2", "ep", &ep_id, "grounds");
    assert!(linked.is_some());

    let unlinked_state = apply(
        new_state,
        Command::DecisionUnlink {
            decision_id: "decision:lnk2".to_string(),
            target_kind: "ep".to_string(),
            target_id: ep_id.clone(),
            relation_kind: "grounds".to_string(),
        },
        &policy,
    )
    .unwrap();

    let still_linked = unlinked_state.get_decision_link("decision:lnk2", "ep", &ep_id, "grounds");
    assert!(still_linked.is_none());
}

#[test]
fn test_command_decision_supersede_via_apply() {
    let mut state = Store::new();
    let policy = NeverAnchoredPolicy;

    decision_ops::create_decision(
        &mut state,
        Some("decision:old2".to_string()),
        "Old decision".to_string(),
        None,
        "Body.".to_string(),
        "Rationale.".to_string(),
        None,
        None,
        "none".to_string(),
        None,
        None,
        None,
    )
    .unwrap();

    decision_ops::create_decision(
        &mut state,
        Some("decision:new2".to_string()),
        "New decision".to_string(),
        None,
        "Body.".to_string(),
        "Rationale.".to_string(),
        None,
        None,
        "none".to_string(),
        None,
        None,
        None,
    )
    .unwrap();

    let new_state = apply(
        state,
        Command::DecisionSupersede {
            old_decision_id: "decision:old2".to_string(),
            new_decision_id: "decision:new2".to_string(),
        },
        &policy,
    )
    .unwrap();

    // supersede_decision creates link: (old_id) --supersedes--> (new_id)
    let link =
        new_state.get_decision_link("decision:old2", "decision", "decision:new2", "supersedes");
    assert!(link.is_some());
}
