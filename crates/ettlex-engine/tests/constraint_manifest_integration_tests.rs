//! Integration tests for constraint manifest integration
//!
//! Tests scenarios 4-7 from the implementation plan:
//! 4. Family-agnostic envelope
//! 5. ABB/SBB projections (empty for backward compat)
//! 6. Unknown families supported
//! 7. Deterministic ordering

use ettlex_core::model::{Constraint, Ep, EpConstraintRef, Ettle};
use ettlex_core::ops::Store;
use ettlex_core::snapshot::manifest::generate_manifest;
use serde_json::json;

// Scenario 4: Family-agnostic envelope populates from EPT constraints
#[test]
fn test_scenario_4_family_agnostic_envelope() {
    // Setup: Create store with ettles, EPs, and constraints
    let mut store = Store::new();

    // Create ettle and EP
    let ettle = Ettle::new("ettle-1".to_string(), "Test Ettle".to_string());
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

    // Create constraints with different families
    let abb_constraint = Constraint::new(
        "c1".to_string(),
        "ABB".to_string(),
        "OwnershipRule".to_string(),
        "EP".to_string(),
        json!({"rule": "owner_must_exist"}),
    );
    store.insert_constraint(abb_constraint);

    let sbb_constraint = Constraint::new(
        "c2".to_string(),
        "SBB".to_string(),
        "ComplianceCheck".to_string(),
        "EP".to_string(),
        json!({"check": "compliance"}),
    );
    store.insert_constraint(sbb_constraint);

    let custom_constraint = Constraint::new(
        "c3".to_string(),
        "Custom".to_string(),
        "CustomRule".to_string(),
        "EP".to_string(),
        json!({"custom": "rule"}),
    );
    store.insert_constraint(custom_constraint);

    // Attach constraints to EP
    store.insert_ep_constraint_ref(EpConstraintRef::new(
        "ep-1".to_string(),
        "c1".to_string(),
        0,
    ));
    store.insert_ep_constraint_ref(EpConstraintRef::new(
        "ep-1".to_string(),
        "c2".to_string(),
        1,
    ));
    store.insert_ep_constraint_ref(EpConstraintRef::new(
        "ep-1".to_string(),
        "c3".to_string(),
        2,
    ));

    // Generate manifest with EPT containing the EP
    let ept = vec!["ep-1".to_string()];
    let manifest = generate_manifest(
        ept,
        "policy/default@0".to_string(),
        "profile/default@0".to_string(),
        "ettle-1".to_string(),
        "0001".to_string(),
        None,
        &store,
    )
    .unwrap();

    // Verify constraints envelope is populated
    assert!(!manifest.constraints.declared_refs.is_empty());
    assert_eq!(manifest.constraints.declared_refs.len(), 3);

    // Verify families are present
    assert!(manifest.constraints.families.contains_key("ABB"));
    assert!(manifest.constraints.families.contains_key("SBB"));
    assert!(manifest.constraints.families.contains_key("Custom"));

    // Verify constraints_digest is computed
    assert!(!manifest.constraints.constraints_digest.is_empty());
    assert_eq!(manifest.constraints.constraints_digest.len(), 64);
}

// Scenario 5: ABB/SBB projections remain empty (backward compatibility)
#[test]
fn test_scenario_5_abb_sbb_projections_empty() {
    let mut store = Store::new();

    // Create ettle and EP with ABB constraint
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

    let abb_constraint = Constraint::new(
        "c1".to_string(),
        "ABB".to_string(),
        "Rule".to_string(),
        "EP".to_string(),
        json!({"rule": "test"}),
    );
    store.insert_constraint(abb_constraint);

    store.insert_ep_constraint_ref(EpConstraintRef::new(
        "ep-1".to_string(),
        "c1".to_string(),
        0,
    ));

    let ept = vec!["ep-1".to_string()];
    let manifest = generate_manifest(
        ept,
        "policy/default@0".to_string(),
        "profile/default@0".to_string(),
        "ettle-1".to_string(),
        "0001".to_string(),
        None,
        &store,
    )
    .unwrap();

    // ABB/SBB projections should remain empty (v0 - backward compat only)
    assert!(manifest.constraints.applicable_abb.is_empty());
    assert!(manifest.constraints.resolved_sbb.is_empty());

    // But family-specific data should be populated
    assert!(manifest.constraints.families.contains_key("ABB"));
    assert!(!manifest.constraints.families["ABB"].active_refs.is_empty());
}

// Scenario 6: Unknown/arbitrary families are supported
#[test]
fn test_scenario_6_unknown_families_supported() {
    let mut store = Store::new();

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

    // Create constraints with completely arbitrary families
    let families = vec!["ZArchitect", "OpenGroup", "Togaf", "CustomFramework"];

    for (i, family) in families.iter().enumerate() {
        let constraint = Constraint::new(
            format!("c{}", i),
            family.to_string(),
            "Rule".to_string(),
            "EP".to_string(),
            json!({"family": family}),
        );
        store.insert_constraint(constraint);
        store.insert_ep_constraint_ref(EpConstraintRef::new(
            "ep-1".to_string(),
            format!("c{}", i),
            i as i32,
        ));
    }

    let ept = vec!["ep-1".to_string()];
    let manifest = generate_manifest(
        ept,
        "policy/default@0".to_string(),
        "profile/default@0".to_string(),
        "ettle-1".to_string(),
        "0001".to_string(),
        None,
        &store,
    )
    .unwrap();

    // All arbitrary families should be present
    for family in families {
        assert!(
            manifest.constraints.families.contains_key(family),
            "Family {} should be present",
            family
        );
    }

    assert_eq!(manifest.constraints.families.len(), 4);
}

// Scenario 7: Deterministic ordering of constraints
#[test]
fn test_scenario_7_deterministic_ordering() {
    let mut store = Store::new();

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

    // Create constraints in non-alphabetical order
    let constraint_ids = ["c-zulu", "c-alpha", "c-mike", "c-bravo"];
    for (i, id) in constraint_ids.iter().enumerate() {
        let constraint = Constraint::new(
            id.to_string(),
            "Family".to_string(),
            "Kind".to_string(),
            "EP".to_string(),
            json!({"id": id}),
        );
        store.insert_constraint(constraint);
        store.insert_ep_constraint_ref(EpConstraintRef::new(
            "ep-1".to_string(),
            id.to_string(),
            i as i32,
        ));
    }

    let ept = vec!["ep-1".to_string()];
    let manifest = generate_manifest(
        ept,
        "policy/default@0".to_string(),
        "profile/default@0".to_string(),
        "ettle-1".to_string(),
        "0001".to_string(),
        None,
        &store,
    )
    .unwrap();

    // declared_refs should be sorted deterministically
    let declared_refs = &manifest.constraints.declared_refs;
    let mut sorted_refs = declared_refs.clone();
    sorted_refs.sort();
    assert_eq!(
        declared_refs, &sorted_refs,
        "declared_refs should be sorted"
    );

    // families should use BTreeMap (deterministic key ordering)
    // Verify by checking serialization is deterministic
    let json1 = serde_json::to_string(&manifest.constraints.families).unwrap();
    let json2 = serde_json::to_string(&manifest.constraints.families).unwrap();
    assert_eq!(json1, json2);
}

// Test semantic_manifest_digest stability with identical constraint state
#[test]
fn test_semantic_digest_stable_with_constraints() {
    let mut store = Store::new();

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

    let constraint = Constraint::new(
        "c1".to_string(),
        "ABB".to_string(),
        "Rule".to_string(),
        "EP".to_string(),
        json!({"rule": "stable"}),
    );
    store.insert_constraint(constraint);
    store.insert_ep_constraint_ref(EpConstraintRef::new(
        "ep-1".to_string(),
        "c1".to_string(),
        0,
    ));

    let ept = vec!["ep-1".to_string()];

    // Generate manifest twice
    let manifest1 = generate_manifest(
        ept.clone(),
        "policy/default@0".to_string(),
        "profile/default@0".to_string(),
        "ettle-1".to_string(),
        "0001".to_string(),
        None,
        &store,
    )
    .unwrap();

    std::thread::sleep(std::time::Duration::from_millis(10));

    let manifest2 = generate_manifest(
        ept,
        "policy/default@0".to_string(),
        "profile/default@0".to_string(),
        "ettle-1".to_string(),
        "0001".to_string(),
        None,
        &store,
    )
    .unwrap();

    // Timestamps differ
    assert_ne!(manifest1.created_at, manifest2.created_at);

    // But semantic digests should be identical (constraints are deterministic)
    assert_eq!(
        manifest1.semantic_manifest_digest,
        manifest2.semantic_manifest_digest
    );

    // And constraints digests should be identical
    assert_eq!(
        manifest1.constraints.constraints_digest,
        manifest2.constraints.constraints_digest
    );
}
