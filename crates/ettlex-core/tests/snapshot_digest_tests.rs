// Test suite for snapshot digest computation
// Tests determinism, semantic digest properties, and digest algorithms

use ettlex_core::ops::Store;
use ettlex_core::snapshot::digest::{
    compute_ept_digest, compute_manifest_digest, compute_semantic_digest,
};
use ettlex_core::snapshot::manifest::generate_manifest;

#[test]
fn test_ept_digest_deterministic() {
    let ept1 = vec!["ep:a".into(), "ep:b".into(), "ep:c".into()];
    let ept2 = vec!["ep:a".into(), "ep:b".into(), "ep:c".into()];

    let digest1 = compute_ept_digest(&ept1).unwrap();
    let digest2 = compute_ept_digest(&ept2).unwrap();

    assert_eq!(digest1, digest2);
    assert_eq!(digest1.len(), 64); // SHA256 hex length
}

#[test]
fn test_ept_digest_order_sensitive() {
    let ept1 = vec!["ep:a".into(), "ep:b".into()];
    let ept2 = vec!["ep:b".into(), "ep:a".into()];

    let digest1 = compute_ept_digest(&ept1).unwrap();
    let digest2 = compute_ept_digest(&ept2).unwrap();

    assert_ne!(digest1, digest2); // Different order → different digest
}

#[test]
fn test_semantic_digest_excludes_created_at() {
    let ept = vec!["ep:root:0".into()];

    // Generate two manifests at different times
    let manifest1 = generate_manifest(
        ept.clone(),
        "policy/default@0".into(),
        "profile/default@0".into(),
        "ettle:root".into(),
        "0001".into(),
        None,
        &Store::new(),
    )
    .unwrap();

    std::thread::sleep(std::time::Duration::from_millis(10));

    let manifest2 = generate_manifest(
        ept,
        "policy/default@0".into(),
        "profile/default@0".into(),
        "ettle:root".into(),
        "0001".into(),
        None,
        &Store::new(),
    )
    .unwrap();

    // Timestamps should differ
    assert_ne!(manifest1.created_at, manifest2.created_at);

    // Semantic digests should be SAME (timestamp excluded)
    let semantic1 = compute_semantic_digest(&manifest1).unwrap();
    let semantic2 = compute_semantic_digest(&manifest2).unwrap();
    assert_eq!(semantic1, semantic2);

    // Manifest digests should DIFFER (timestamp included)
    let full1 = compute_manifest_digest(&manifest1).unwrap();
    let full2 = compute_manifest_digest(&manifest2).unwrap();
    assert_ne!(full1, full2);
}

#[test]
fn test_manifest_digest_includes_created_at() {
    let ept = vec!["ep:root:0".into()];

    let manifest1 = generate_manifest(
        ept.clone(),
        "policy/default@0".into(),
        "profile/default@0".into(),
        "ettle:root".into(),
        "0001".into(),
        None,
        &Store::new(),
    )
    .unwrap();

    std::thread::sleep(std::time::Duration::from_millis(10));

    let manifest2 = generate_manifest(
        ept,
        "policy/default@0".into(),
        "profile/default@0".into(),
        "ettle:root".into(),
        "0001".into(),
        None,
        &Store::new(),
    )
    .unwrap();

    let digest1 = compute_manifest_digest(&manifest1).unwrap();
    let digest2 = compute_manifest_digest(&manifest2).unwrap();

    // Different timestamps → different manifest digests
    assert_ne!(digest1, digest2);
}

#[test]
fn test_digest_format_is_hex_sha256() {
    let ept = vec!["ep:test".into()];
    let digest = compute_ept_digest(&ept).unwrap();

    // SHA256 hex digest is 64 characters
    assert_eq!(digest.len(), 64);

    // All characters should be valid hex
    assert!(digest.chars().all(|c| c.is_ascii_hexdigit()));
}
