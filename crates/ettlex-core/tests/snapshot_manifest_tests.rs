// Test suite for snapshot manifest generation
// Tests basic manifest structure, field population, and schema compliance

use ettlex_core::ops::Store;
use ettlex_core::snapshot::manifest::generate_manifest;

#[test]
fn test_generate_manifest_basic() {
    let ept = vec!["ep:root:0".into(), "ep:root:1".into()];

    let manifest = generate_manifest(
        ept.clone(),
        "policy/default@0".into(),
        "profile/default@0".into(),
        "ettle:root".into(),
        "0001".into(),
        None,
        &Store::new(),
    )
    .unwrap();

    assert_eq!(manifest.manifest_schema_version, 1);
    assert_eq!(manifest.policy_ref, "policy/default@0");
    assert_eq!(manifest.profile_ref, "profile/default@0");
    assert_eq!(manifest.root_ettle_id, "ettle:root");
    assert_eq!(manifest.store_schema_version, "0001");
    assert_eq!(manifest.ept.len(), 2);
    assert!(!manifest.created_at.is_empty()); // Timestamp present
    assert!(manifest.seed_digest.is_none());
}

#[test]
fn test_generate_manifest_with_seed_digest() {
    let ept = vec!["ep:root:0".into()];

    let manifest = generate_manifest(
        ept,
        "policy/default@0".into(),
        "profile/default@0".into(),
        "ettle:root".into(),
        "0001".into(),
        Some("abc123".into()),
        &Store::new(),
    )
    .unwrap();

    assert_eq!(manifest.seed_digest, Some("abc123".into()));
}

#[test]
fn test_generate_manifest_ep_entries_have_ordinals() {
    let ept = vec!["ep:a".into(), "ep:b".into(), "ep:c".into()];

    let manifest = generate_manifest(
        ept,
        "policy/default@0".into(),
        "profile/default@0".into(),
        "ettle:root".into(),
        "0001".into(),
        None,
        &Store::new(),
    )
    .unwrap();

    assert_eq!(manifest.ept[0].ordinal, 0);
    assert_eq!(manifest.ept[1].ordinal, 1);
    assert_eq!(manifest.ept[2].ordinal, 2);
    assert_eq!(manifest.ept[0].ep_id, "ep:a");
    assert_eq!(manifest.ept[1].ep_id, "ep:b");
    assert_eq!(manifest.ept[2].ep_id, "ep:c");
}

#[test]
fn test_generate_manifest_all_eps_normative() {
    let ept = vec!["ep:root:0".into()];

    let manifest = generate_manifest(
        ept,
        "policy/default@0".into(),
        "profile/default@0".into(),
        "ettle:root".into(),
        "0001".into(),
        None,
        &Store::new(),
    )
    .unwrap();

    // All EPs are normative in v0
    assert!(manifest.ept[0].normative);
}

#[test]
fn test_generate_manifest_v0_fields_empty() {
    let ept = vec!["ep:root:0".into()];

    let manifest = generate_manifest(
        ept,
        "policy/default@0".into(),
        "profile/default@0".into(),
        "ettle:root".into(),
        "0001".into(),
        None,
        &Store::new(),
    )
    .unwrap();

    // V0 fields should be empty
    assert!(manifest.constraints.declared_refs.is_empty());
    assert!(manifest.constraints.families.is_empty());
    assert!(manifest.constraints.applicable_abb.is_empty());
    assert!(manifest.constraints.resolved_sbb.is_empty());
    assert!(manifest.constraints.resolution_evidence.is_empty());
    assert!(!manifest.constraints.constraints_digest.is_empty());
    assert!(manifest.exceptions.is_empty());
    assert_eq!(
        manifest.coverage,
        serde_json::Value::Object(serde_json::Map::new())
    );
}
