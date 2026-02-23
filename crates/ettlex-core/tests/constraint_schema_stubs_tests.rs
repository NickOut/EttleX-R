//! Comprehensive test suite for constraint schema stubs
//!
//! Tests scenarios 8-17 from seed_constraint_schema_stubs_v9.yaml:
//! - Contract violations (closed enum, field removal, non-deterministic iteration)
//! - Negative cases (malformed payload, large payloads, empty lists)
//! - Decision isolation

use ettlex_core::model::{Constraint, Ep, EpConstraintRef, Ettle};
use ettlex_core::ops::constraint_ops;
use ettlex_core::ops::Store;
use serde_json::json;

// Scenario 8: No closed enum - arbitrary families work without code changes
#[test]
fn test_scenario_8_no_closed_enum_arbitrary_families() {
    let mut store = Store::new();

    // Create constraints with completely arbitrary family names
    // No enum means no code change needed for new families
    let arbitrary_families = [
        "FutureFramework2026",
        "LegacySystem",
        "CustomOrg",
        "ThirdPartyTool",
        "研究", // Unicode family name
    ];

    for (i, family) in arbitrary_families.iter().enumerate() {
        let result = constraint_ops::create_constraint(
            &mut store,
            format!("c{}", i),
            family.to_string(),
            "Rule".to_string(),
            "EP".to_string(),
            json!({"family": family}),
        );

        // Should succeed without any schema changes
        assert!(result.is_ok(), "Family '{}' should be accepted", family);
    }

    // Verify all were stored
    let all_constraints = store.list_constraints();
    assert_eq!(all_constraints.len(), 5);
}

// Scenario 9: Additive-only schema evolution
#[test]
fn test_scenario_9_additive_only_no_field_removal() {
    // This scenario is enforced by Rust's type system
    // If we try to remove a field from Constraint, compilation fails
    // This test documents the requirement

    let constraint = Constraint::new(
        "c1".to_string(),
        "ABB".to_string(),
        "Rule".to_string(),
        "EP".to_string(),
        json!({"rule": "test"}),
    );

    // All fields must be present (enforced by struct definition)
    assert!(!constraint.constraint_id.is_empty());
    assert!(!constraint.family.is_empty());
    assert!(!constraint.kind.is_empty());
    assert!(!constraint.scope.is_empty());
    assert!(!constraint.payload_digest.is_empty());

    // created_at and updated_at are always set
    assert!(constraint.created_at.timestamp() > 0);
    assert!(constraint.updated_at.timestamp() > 0);

    // deleted_at is optional but the field exists
    assert!(constraint.deleted_at.is_none());
}

// Scenario 10: Non-deterministic iteration prevention via BTreeMap
#[test]
fn test_scenario_10_deterministic_iteration() {
    use ettlex_core::snapshot::manifest::ConstraintsEnvelope;

    let mut store = Store::new();

    // Create ettle and EP
    let ettle = Ettle::new("ettle-1".to_string(), "Test".to_string());
    store.insert_ettle(ettle);

    let ep = Ep::new(
        "ep-1".to_string(),
        "ettle-1".to_string(),
        0,
        true,
        "why".to_string(),
        "what".to_string(),
        "how".to_string(),
    );
    store.insert_ep(ep);

    // Create constraints with families that would have different HashMap iteration order
    let families = ["Zulu", "Alpha", "Mike", "Charlie", "Echo"];

    for (i, family) in families.iter().enumerate() {
        let constraint = Constraint::new(
            format!("c{}", i),
            family.to_string(),
            "Rule".to_string(),
            "EP".to_string(),
            json!({}),
        );
        store.insert_constraint(constraint);
        store.insert_ep_constraint_ref(EpConstraintRef::new(
            "ep-1".to_string(),
            format!("c{}", i),
            i as i32,
        ));
    }

    let ept = vec!["ep-1".to_string()];

    // Generate envelope multiple times
    let envelope1 = ConstraintsEnvelope::from_ept(&ept, &store).unwrap();
    let envelope2 = ConstraintsEnvelope::from_ept(&ept, &store).unwrap();

    // Serialization should be deterministic
    let json1 = serde_json::to_string(&envelope1).unwrap();
    let json2 = serde_json::to_string(&envelope2).unwrap();

    assert_eq!(json1, json2, "Serialization must be deterministic");

    // Verify families are in sorted order (BTreeMap guarantees this)
    let family_keys: Vec<_> = envelope1.families.keys().cloned().collect();
    let mut sorted_keys = family_keys.clone();
    sorted_keys.sort();
    assert_eq!(family_keys, sorted_keys, "Family keys must be sorted");
}

// Scenario 11: Negative case - malformed payload (JSON accepts anything)
#[test]
fn test_scenario_11_malformed_payload_accepted() {
    let mut store = Store::new();

    // JSON Value accepts any structure - this is intentional for flexibility
    let weird_payloads = [
        json!(null),
        json!([1, 2, 3]),
        json!("just a string"),
        json!(42),
        json!({"nested": {"deeply": {"very": {"deep": "value"}}}}),
    ];

    for (i, payload) in weird_payloads.iter().enumerate() {
        let result = constraint_ops::create_constraint(
            &mut store,
            format!("c{}", i),
            "Family".to_string(),
            "Kind".to_string(),
            "EP".to_string(),
            payload.clone(),
        );

        // All valid JSON is accepted
        assert!(
            result.is_ok(),
            "Valid JSON should be accepted: {:?}",
            payload
        );
    }
}

// Scenario 12: Large payloads are accepted (no artificial size limits)
#[test]
fn test_scenario_12_large_payloads() {
    let mut store = Store::new();

    // Create a large payload (simulating complex constraint configuration)
    let large_data: Vec<_> = (0..1000)
        .map(|i| json!({"rule": format!("rule_{}", i), "value": i}))
        .collect();
    let large_payload = json!({"rules": large_data});

    let result = constraint_ops::create_constraint(
        &mut store,
        "c1".to_string(),
        "Complex".to_string(),
        "MultiRule".to_string(),
        "EP".to_string(),
        large_payload,
    );

    assert!(result.is_ok(), "Large payloads should be accepted");

    // Verify it was stored and can be retrieved
    let constraint = store.get_constraint("c1").unwrap();
    assert_eq!(
        constraint.payload_json["rules"].as_array().unwrap().len(),
        1000
    );
}

// Scenario 13: Empty constraint lists are valid
#[test]
fn test_scenario_13_empty_constraint_lists() {
    use ettlex_core::snapshot::manifest::ConstraintsEnvelope;

    let store = Store::new(); // Empty store

    // Create ettle and EP but no constraints
    let ept = vec!["ep-1".to_string()];

    let envelope = ConstraintsEnvelope::from_ept(&ept, &store).unwrap();

    // Empty lists should be valid
    assert!(envelope.declared_refs.is_empty());
    assert!(envelope.families.is_empty());
    assert!(envelope.applicable_abb.is_empty());
    assert!(envelope.resolved_sbb.is_empty());

    // Digest should still be computed
    assert!(!envelope.constraints_digest.is_empty());
}

// Scenario 14: Decision isolation - constraints don't depend on Decision artefacts
#[test]
fn test_scenario_14_constraints_independent_of_decisions() {
    // Constraints are completely independent domain model
    // They don't reference Decision types at all

    let mut store = Store::new();

    // Create constraint without any Decision-related data
    let result = constraint_ops::create_constraint(
        &mut store,
        "c1".to_string(),
        "ABB".to_string(),
        "Rule".to_string(),
        "EP".to_string(),
        json!({"rule": "standalone"}),
    );

    assert!(result.is_ok());

    // Constraint operations work without Decision module
    let constraint = store.get_constraint("c1").unwrap();
    assert_eq!(constraint.family, "ABB");

    // Update constraint
    constraint_ops::update_constraint(&mut store, "c1", json!({"rule": "updated"})).unwrap();

    // Tombstone constraint
    constraint_ops::tombstone_constraint(&mut store, "c1").unwrap();

    // All operations succeed without Decision dependency
}

// Scenario 15: Decision isolation - Decision artefacts don't depend on constraints
#[test]
fn test_scenario_15_decisions_independent_of_constraints() {
    // This test documents that Decision module (when implemented)
    // will not depend on Constraint module

    // Constraint types are in ettlex_core::model::constraint
    // Decision types will be in ettlex_core::model::decision (future)
    // No circular dependency

    // For now, we verify constraints don't pollute core APIs
    let mut store = Store::new();

    // Core operations (create_ettle, create_ep) don't require constraints
    use ettlex_core::ops::{ep_ops, ettle_ops};

    let ettle_id = ettle_ops::create_ettle(
        &mut store,
        "Test".to_string(),
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

    // Core operations succeed without any constraint involvement
    assert!(!ettle_id.is_empty());
    assert!(!ep_id.is_empty());
}

// Scenario 16: Constraint attachment is EP-specific
#[test]
fn test_scenario_16_ep_level_attachment() {
    let mut store = Store::new();

    // Create two EPs
    let ettle_id = ettlex_core::ops::ettle_ops::create_ettle(
        &mut store,
        "Test".to_string(),
        None,
        None,
        Some("what".to_string()),
        Some("how".to_string()),
    )
    .unwrap();

    let ep1 = ettlex_core::ops::ep_ops::create_ep(
        &mut store,
        &ettle_id,
        1,
        false,
        "why1".to_string(),
        "what1".to_string(),
        "how1".to_string(),
    )
    .unwrap();

    let ep2 = ettlex_core::ops::ep_ops::create_ep(
        &mut store,
        &ettle_id,
        2,
        false,
        "why2".to_string(),
        "what2".to_string(),
        "how2".to_string(),
    )
    .unwrap();

    // Create constraint
    constraint_ops::create_constraint(
        &mut store,
        "c1".to_string(),
        "ABB".to_string(),
        "Rule".to_string(),
        "EP".to_string(),
        json!({"rule": "test"}),
    )
    .unwrap();

    // Attach to EP1 only
    constraint_ops::attach_constraint_to_ep(&mut store, ep1.clone(), "c1".to_string(), 0).unwrap();

    // Verify attachment is EP-specific
    assert!(store.is_constraint_attached_to_ep(&ep1, "c1"));
    assert!(!store.is_constraint_attached_to_ep(&ep2, "c1"));

    // List constraints for each EP
    let ep1_constraints = constraint_ops::list_constraints_for_ep(&store, &ep1).unwrap();
    let ep2_constraints = constraint_ops::list_constraints_for_ep(&store, &ep2).unwrap();

    assert_eq!(ep1_constraints.len(), 1);
    assert_eq!(ep2_constraints.len(), 0);
}

// Scenario 17: Payload digest enables content deduplication
#[test]
fn test_scenario_17_payload_digest_deduplication() {
    let payload = json!({"rule": "identical", "value": 42});

    // Create two constraints with identical payloads
    let c1 = Constraint::new(
        "c1".to_string(),
        "Family".to_string(),
        "Kind".to_string(),
        "EP".to_string(),
        payload.clone(),
    );

    let c2 = Constraint::new(
        "c2".to_string(),
        "Family".to_string(),
        "Kind".to_string(),
        "EP".to_string(),
        payload,
    );

    // Digests should be identical (enabling CAS deduplication)
    assert_eq!(c1.payload_digest, c2.payload_digest);

    // Different IDs but same content
    assert_ne!(c1.constraint_id, c2.constraint_id);
    assert_eq!(c1.payload_json, c2.payload_json);
}

// Edge case: Constraint operations with non-existent EPs
#[test]
fn test_edge_case_attach_to_nonexistent_ep() {
    let mut store = Store::new();

    constraint_ops::create_constraint(
        &mut store,
        "c1".to_string(),
        "ABB".to_string(),
        "Rule".to_string(),
        "EP".to_string(),
        json!({}),
    )
    .unwrap();

    let result = constraint_ops::attach_constraint_to_ep(
        &mut store,
        "nonexistent".to_string(),
        "c1".to_string(),
        0,
    );

    assert!(result.is_err());
    assert!(matches!(
        result,
        Err(ettlex_core::errors::EttleXError::EpNotFound { .. })
    ));
}

// Edge case: Constraint operations with non-existent constraints
#[test]
fn test_edge_case_nonexistent_constraint() {
    let store = Store::new();

    let result = constraint_ops::get_constraint(&store, "nonexistent");

    assert!(result.is_err());
    assert!(matches!(
        result,
        Err(ettlex_core::errors::EttleXError::ConstraintNotFound { .. })
    ));
}

// Edge case: Update deleted constraint
#[test]
fn test_edge_case_update_deleted_constraint() {
    let mut store = Store::new();

    constraint_ops::create_constraint(
        &mut store,
        "c1".to_string(),
        "ABB".to_string(),
        "Rule".to_string(),
        "EP".to_string(),
        json!({}),
    )
    .unwrap();

    constraint_ops::tombstone_constraint(&mut store, "c1").unwrap();

    let result = constraint_ops::update_constraint(&mut store, "c1", json!({"new": "data"}));

    assert!(result.is_err());
    assert!(matches!(
        result,
        Err(ettlex_core::errors::EttleXError::ConstraintDeleted { .. })
    ));
}

// Edge case: Detach non-attached constraint
#[test]
fn test_edge_case_detach_non_attached() {
    let mut store = Store::new();

    let ettle_id = ettlex_core::ops::ettle_ops::create_ettle(
        &mut store,
        "Test".to_string(),
        None,
        None,
        Some("what".to_string()),
        Some("how".to_string()),
    )
    .unwrap();

    let ep_id = ettlex_core::ops::ep_ops::create_ep(
        &mut store,
        &ettle_id,
        1,
        false,
        "why".to_string(),
        "what".to_string(),
        "how".to_string(),
    )
    .unwrap();

    let result = constraint_ops::detach_constraint_from_ep(&mut store, &ep_id, "c1");

    assert!(result.is_err());
    assert!(matches!(
        result,
        Err(ettlex_core::errors::EttleXError::ConstraintNotAttached { .. })
    ));
}

// Performance: Multiple constraints on single EP
#[test]
fn test_performance_many_constraints_single_ep() {
    let mut store = Store::new();

    let ettle_id = ettlex_core::ops::ettle_ops::create_ettle(
        &mut store,
        "Test".to_string(),
        None,
        None,
        Some("what".to_string()),
        Some("how".to_string()),
    )
    .unwrap();

    let ep_id = ettlex_core::ops::ep_ops::create_ep(
        &mut store,
        &ettle_id,
        1,
        false,
        "why".to_string(),
        "what".to_string(),
        "how".to_string(),
    )
    .unwrap();

    // Attach 100 constraints to single EP
    for i in 0..100 {
        constraint_ops::create_constraint(
            &mut store,
            format!("c{}", i),
            "Family".to_string(),
            "Kind".to_string(),
            "EP".to_string(),
            json!({"index": i}),
        )
        .unwrap();

        constraint_ops::attach_constraint_to_ep(&mut store, ep_id.clone(), format!("c{}", i), i)
            .unwrap();
    }

    // List should maintain order
    let constraints = constraint_ops::list_constraints_for_ep(&store, &ep_id).unwrap();
    assert_eq!(constraints.len(), 100);

    // Verify ordering
    for (i, constraint) in constraints.iter().enumerate() {
        assert_eq!(constraint.constraint_id, format!("c{}", i));
    }
}
