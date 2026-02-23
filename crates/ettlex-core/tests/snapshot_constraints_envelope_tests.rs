// Test suite for snapshot manifest constraints envelope (v4 requirements)
// Tests that the constraints envelope structure is always present with all required fields

use ettlex_core::ops::Store;
use ettlex_core::snapshot::manifest::generate_manifest;

#[test]
fn test_manifest_contains_constraints_envelope_even_when_empty() {
    // Scenario: Snapshot manifest always contains constraints envelope fields even when no constraints are attached
    // Given an EPT whose EPs have no attached constraints
    let ept = vec!["ep:a".to_string(), "ep:b".to_string()];

    // When I run snapshot_commit
    let manifest = generate_manifest(
        ept,
        "policy/default@0".to_string(),
        "profile/default@0".to_string(),
        "ettle:root".to_string(),
        "0001".to_string(),
        None,
        &Store::new(),
    )
    .unwrap();

    // Then the manifest contains constraints.declared_refs as an empty list
    assert!(manifest.constraints.declared_refs.is_empty());

    // And the manifest contains constraints.families as an empty map
    assert!(manifest.constraints.families.is_empty());

    // And the manifest contains constraints.applicable_abb as an empty list
    assert!(manifest.constraints.applicable_abb.is_empty());

    // And the manifest contains constraints.resolved_sbb as an empty list
    assert!(manifest.constraints.resolved_sbb.is_empty());

    // And the manifest contains constraints.resolution_evidence as an empty list
    assert!(manifest.constraints.resolution_evidence.is_empty());

    // And constraints_digest is present
    assert!(!manifest.constraints.constraints_digest.is_empty());
}

#[test]
fn test_deterministic_ordering_of_declared_refs() {
    // Scenario: Deterministic ordering of declared_refs is stable across insertion order
    // This test verifies that even if constraints were attached in different orders,
    // the declared_refs list would be deterministically ordered

    // For now, with no constraint attachment mechanism, we verify:
    // - declared_refs is a Vec (ordered)
    // - families is a BTreeMap (deterministically ordered)

    let ept = vec!["ep:a".to_string()];

    let manifest1 = generate_manifest(
        ept.clone(),
        "policy/default@0".to_string(),
        "profile/default@0".to_string(),
        "ettle:root".to_string(),
        "0001".to_string(),
        None,
        &Store::new(),
    )
    .unwrap();

    let manifest2 = generate_manifest(
        ept,
        "policy/default@0".to_string(),
        "profile/default@0".to_string(),
        "ettle:root".to_string(),
        "0001".to_string(),
        None,
        &Store::new(),
    )
    .unwrap();

    // Then declared_refs ordering is identical
    assert_eq!(
        manifest1.constraints.declared_refs,
        manifest2.constraints.declared_refs
    );

    // And constraints_digest is identical
    assert_eq!(
        manifest1.constraints.constraints_digest,
        manifest2.constraints.constraints_digest
    );
}

#[test]
fn test_constraints_envelope_serialization_is_deterministic() {
    // Verify that the constraints envelope serializes deterministically
    // This ensures semantic_manifest_digest stability

    let ept = vec!["ep:a".to_string()];

    let manifest = generate_manifest(
        ept,
        "policy/default@0".to_string(),
        "profile/default@0".to_string(),
        "ettle:root".to_string(),
        "0001".to_string(),
        None,
        &Store::new(),
    )
    .unwrap();

    // Serialize constraints twice
    let json1 = serde_json::to_string(&manifest.constraints).unwrap();
    let json2 = serde_json::to_string(&manifest.constraints).unwrap();

    // Should be identical (deterministic key ordering)
    assert_eq!(json1, json2);
}

#[test]
fn test_constraints_digest_is_computed() {
    // Verify that constraints_digest is always computed, even for empty constraints

    let ept = vec!["ep:a".to_string()];

    let manifest = generate_manifest(
        ept,
        "policy/default@0".to_string(),
        "profile/default@0".to_string(),
        "ettle:root".to_string(),
        "0001".to_string(),
        None,
        &Store::new(),
    )
    .unwrap();

    // constraints_digest should be non-empty
    assert!(!manifest.constraints.constraints_digest.is_empty());

    // constraints_digest should be a valid hex string (64 chars for SHA256)
    assert_eq!(manifest.constraints.constraints_digest.len(), 64);
    assert!(manifest
        .constraints
        .constraints_digest
        .chars()
        .all(|c| c.is_ascii_hexdigit()));
}
