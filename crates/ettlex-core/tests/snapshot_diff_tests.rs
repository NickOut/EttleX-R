//! Pure diff unit tests for ep:snapshot_diff:0 — 26 scenarios.
//!
//! All tests operate exclusively on manifest bytes (no I/O, no DB).

use ettlex_core::diff::engine::compute_diff;
use ettlex_core::diff::model::{DiffClassification, DiffSeverity, InvariantViolationEntry};
use ettlex_core::errors::ExErrorKind;
use serde_json::{json, Value};

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

/// Build a minimal valid manifest JSON with the given overrides.
fn base_manifest() -> Value {
    json!({
        "manifest_schema_version": 1,
        "created_at": "2026-01-01T00:00:00Z",
        "policy_ref": "policy/default@0",
        "profile_ref": "profile/default@0",
        "ept": [
            {"ep_id": "ep:root:0", "ordinal": 0, "normative": true, "ep_digest": "aabbcc0000000000000000000000000000000000000000000000000000000000"}
        ],
        "constraints": {
            "declared_refs": [],
            "families": {},
            "applicable_abb": [],
            "resolved_sbb": [],
            "resolution_evidence": [],
            "constraints_digest": constraints_digest_for(&[], &std::collections::BTreeMap::new())
        },
        "coverage": {},
        "exceptions": [],
        "root_ettle_id": "ettle:root",
        "ept_digest": "0000000000000000000000000000000000000000000000000000000000000001",
        "manifest_digest": "0000000000000000000000000000000000000000000000000000000000000002",
        "semantic_manifest_digest": "0000000000000000000000000000000000000000000000000000000000000003",
        "store_schema_version": "0001",
        "seed_digest": null
    })
}

/// Re-compute constraints_digest from declared_refs + family digests, mirroring constraint_engine.
fn constraints_digest_for(
    declared_refs: &[&str],
    families: &std::collections::BTreeMap<&str, &str>,
) -> String {
    use sha2::{Digest, Sha256};
    let ref_ids: Vec<&str> = declared_refs.to_vec();
    let family_digests: Vec<(&str, &str)> = families.iter().map(|(k, v)| (*k, *v)).collect();
    let digest_input = vec![
        serde_json::to_value(&ref_ids).unwrap(),
        serde_json::to_value(&family_digests).unwrap(),
    ];
    let canonical = serde_json::to_string(&digest_input).unwrap();
    let mut hasher = Sha256::new();
    hasher.update(canonical.as_bytes());
    hex::encode(hasher.finalize())
}

/// Serialize a JSON value to bytes (the "manifest bytes").
fn to_bytes(v: &Value) -> Vec<u8> {
    serde_json::to_vec(v).unwrap()
}

/// Create two manifests with different `semantic_manifest_digest` values.
fn two_different_manifests() -> (Value, Value) {
    let mut a = base_manifest();
    let mut b = base_manifest();
    a["semantic_manifest_digest"] =
        json!("aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa");
    b["semantic_manifest_digest"] =
        json!("bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb");
    a["manifest_digest"] =
        json!("cccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccc");
    b["manifest_digest"] =
        json!("dddddddddddddddddddddddddddddddddddddddddddddddddddddddddddddddd");
    (a, b)
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

// S1: Diff output is deterministic
#[test]
fn test_diff_is_deterministic() {
    let (a, b) = two_different_manifests();
    let a_bytes = to_bytes(&a);
    let b_bytes = to_bytes(&b);
    let diff1 = compute_diff(&a_bytes, &b_bytes).unwrap();
    let diff2 = compute_diff(&a_bytes, &b_bytes).unwrap();
    assert_eq!(diff1, diff2);
    // Serialized form must also be identical
    let s1 = serde_json::to_string(&diff1).unwrap();
    let s2 = serde_json::to_string(&diff2).unwrap();
    assert_eq!(s1, s2);
}

// S2: Diffing against itself → no changes
#[test]
fn test_diff_self_yields_no_changes() {
    let a = base_manifest();
    let a_bytes = to_bytes(&a);
    let diff = compute_diff(&a_bytes, &a_bytes).unwrap();
    assert_eq!(diff.classification, DiffClassification::Identical);
    assert_eq!(diff.severity, DiffSeverity::None);
    assert!(diff.ept_changes.added_eps.is_empty());
    assert!(diff.ept_changes.removed_eps.is_empty());
    assert!(!diff.ept_changes.ordering_changed);
}

// S3: created_at is non-semantic
#[test]
fn test_diff_treats_created_at_as_non_semantic() {
    let mut a = base_manifest();
    let mut b = base_manifest();
    // Same semantic digest, different created_at
    a["created_at"] = json!("2026-01-01T00:00:00Z");
    b["created_at"] = json!("2026-06-01T00:00:00Z");
    // Ensure same semantic digest
    let sem = "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa";
    a["semantic_manifest_digest"] = json!(sem);
    b["semantic_manifest_digest"] = json!(sem);
    a["manifest_digest"] =
        json!("1111111111111111111111111111111111111111111111111111111111111111");
    b["manifest_digest"] =
        json!("2222222222222222222222222222222222222222222222222222222222222222");

    let diff = compute_diff(&to_bytes(&a), &to_bytes(&b)).unwrap();
    assert_eq!(diff.classification, DiffClassification::NoSemanticChange);
    assert_eq!(diff.severity, DiffSeverity::None);
}

// S4: Unknown fields → no breakage
#[test]
fn test_diff_handles_unknown_fields_additive() {
    let mut a = base_manifest();
    let mut b = base_manifest();
    b["some_future_field"] = json!("hello");
    // Different semantic digests to trigger full diff path
    a["semantic_manifest_digest"] =
        json!("aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa");
    b["semantic_manifest_digest"] =
        json!("bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb");
    a["manifest_digest"] =
        json!("cccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccc");
    b["manifest_digest"] =
        json!("dddddddddddddddddddddddddddddddddddddddddddddddddddddddddddddddd");

    let diff = compute_diff(&to_bytes(&a), &to_bytes(&b)).unwrap();
    // Should succeed and flag the unknown field
    assert_eq!(diff.classification, DiffClassification::Changed);
    assert!(diff
        .unknown_changes
        .added_fields
        .contains(&"some_future_field".to_string()));
}

// S5: EPT change detected
#[test]
fn test_diff_detects_ept_change() {
    let mut a = base_manifest();
    let mut b = base_manifest();
    b["ept"] = json!([
        {"ep_id": "ep:root:0", "ordinal": 0, "normative": true,
         "ep_digest": "aabbcc0000000000000000000000000000000000000000000000000000000000"},
        {"ep_id": "ep:root:1", "ordinal": 1, "normative": true,
         "ep_digest": "ddeeff0000000000000000000000000000000000000000000000000000000000"}
    ]);
    a["semantic_manifest_digest"] =
        json!("aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa");
    b["semantic_manifest_digest"] =
        json!("bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb");
    a["manifest_digest"] =
        json!("cccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccc");
    b["manifest_digest"] =
        json!("dddddddddddddddddddddddddddddddddddddddddddddddddddddddddddddddd");

    let diff = compute_diff(&to_bytes(&a), &to_bytes(&b)).unwrap();
    assert!(diff.ept_changes.changed);
    assert!(diff
        .ept_changes
        .added_eps
        .contains(&"ep:root:1".to_string()));
    assert_eq!(diff.severity, DiffSeverity::Breaking);
}

// S6: EP digest changes within same EPT
#[test]
fn test_diff_detects_ep_content_change() {
    let mut a = base_manifest();
    let mut b = base_manifest();
    b["ept"] = json!([
        {"ep_id": "ep:root:0", "ordinal": 0, "normative": true,
         "ep_digest": "ffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffff"}
    ]);
    a["semantic_manifest_digest"] =
        json!("aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa");
    b["semantic_manifest_digest"] =
        json!("bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb");
    a["manifest_digest"] =
        json!("cccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccc");
    b["manifest_digest"] =
        json!("dddddddddddddddddddddddddddddddddddddddddddddddddddddddddddddddd");

    let diff = compute_diff(&to_bytes(&a), &to_bytes(&b)).unwrap();
    assert!(!diff.ept_changes.changed, "EPT set should not have changed");
    assert!(diff
        .ep_content_changes
        .changed_eps
        .contains(&"ep:root:0".to_string()));
    assert_eq!(diff.severity, DiffSeverity::Semantic);
}

// S7: Ordinal reordering as EPT change
#[test]
fn test_diff_detects_ordinal_reordering() {
    let mut a = base_manifest();
    let mut b = base_manifest();
    a["ept"] = json!([
        {"ep_id": "ep:root:0", "ordinal": 0, "normative": true, "ep_digest": "aa00000000000000000000000000000000000000000000000000000000000000"},
        {"ep_id": "ep:root:1", "ordinal": 1, "normative": true, "ep_digest": "bb00000000000000000000000000000000000000000000000000000000000000"}
    ]);
    b["ept"] = json!([
        {"ep_id": "ep:root:1", "ordinal": 0, "normative": true, "ep_digest": "bb00000000000000000000000000000000000000000000000000000000000000"},
        {"ep_id": "ep:root:0", "ordinal": 1, "normative": true, "ep_digest": "aa00000000000000000000000000000000000000000000000000000000000000"}
    ]);
    a["semantic_manifest_digest"] =
        json!("aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa");
    b["semantic_manifest_digest"] =
        json!("bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb");
    a["manifest_digest"] =
        json!("cccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccc");
    b["manifest_digest"] =
        json!("dddddddddddddddddddddddddddddddddddddddddddddddddddddddddddddddd");

    let diff = compute_diff(&to_bytes(&a), &to_bytes(&b)).unwrap();
    assert!(diff.ept_changes.ordering_changed);
    assert!(diff.ept_changes.added_eps.is_empty());
    assert!(diff.ept_changes.removed_eps.is_empty());
}

// S8: Addition of unknown family constraint
#[test]
fn test_diff_detects_constraint_addition_unknown_family() {
    let (a, mut b) = two_different_manifests();
    let family_digest = "abcdef0000000000000000000000000000000000000000000000000000000000";
    b["constraints"] = json!({
        "declared_refs": ["c1"],
        "families": {
            "CUSTOM": {
                "status": "UNCOMPUTED",
                "active_refs": ["c1"],
                "outcomes": [],
                "evidence": [],
                "digest": family_digest
            }
        },
        "applicable_abb": [],
        "resolved_sbb": [],
        "resolution_evidence": [],
        "constraints_digest": constraints_digest_for(&["c1"], &{
            let mut m = std::collections::BTreeMap::new();
            m.insert("CUSTOM", family_digest);
            m
        })
    });

    let diff = compute_diff(&to_bytes(&a), &to_bytes(&b)).unwrap();
    assert!(!diff
        .constraint_changes
        .declared_ref_changes
        .added
        .is_empty());
    assert!(diff
        .constraint_changes
        .family_changes
        .contains_key("CUSTOM"));
    let family_entry = diff
        .constraint_changes
        .family_changes
        .get("CUSTOM")
        .unwrap();
    assert!(family_entry.added);
}

// S9: Constraint payload change via digest
#[test]
fn test_diff_detects_constraint_payload_change() {
    let (mut a, mut b) = two_different_manifests();
    let digest_a = "aaaa000000000000000000000000000000000000000000000000000000000000";
    let digest_b = "bbbb000000000000000000000000000000000000000000000000000000000000";
    let families_a = {
        let mut m = std::collections::BTreeMap::new();
        m.insert("ABB", digest_a);
        m
    };
    let families_b = {
        let mut m = std::collections::BTreeMap::new();
        m.insert("ABB", digest_b);
        m
    };
    a["constraints"] = json!({
        "declared_refs": ["c1"],
        "families": {
            "ABB": {"status": "UNCOMPUTED", "active_refs": ["c1"], "outcomes": [], "evidence": [], "digest": digest_a}
        },
        "applicable_abb": [],
        "resolved_sbb": [],
        "resolution_evidence": [],
        "constraints_digest": constraints_digest_for(&["c1"], &families_a)
    });
    b["constraints"] = json!({
        "declared_refs": ["c1"],
        "families": {
            "ABB": {"status": "UNCOMPUTED", "active_refs": ["c1"], "outcomes": [], "evidence": [], "digest": digest_b}
        },
        "applicable_abb": [],
        "resolved_sbb": [],
        "resolution_evidence": [],
        "constraints_digest": constraints_digest_for(&["c1"], &families_b)
    });

    let diff = compute_diff(&to_bytes(&a), &to_bytes(&b)).unwrap();
    assert!(diff.constraint_changes.family_changes.contains_key("ABB"));
    let entry = diff.constraint_changes.family_changes.get("ABB").unwrap();
    assert!(entry.digest_changed);
    assert!(!entry.added);
    assert!(!entry.removed);
}

// S10: ABB/SBB projection parity
#[test]
fn test_diff_abb_sbb_projection_parity() {
    let (mut a, mut b) = two_different_manifests();
    a["constraints"]["applicable_abb"] = json!(["c1", "c2"]);
    a["constraints"]["resolved_sbb"] = json!(["c2"]);
    b["constraints"]["applicable_abb"] = json!(["c2", "c3"]);
    b["constraints"]["resolved_sbb"] = json!(["c2", "c3"]);

    let diff = compute_diff(&to_bytes(&a), &to_bytes(&b)).unwrap();
    let abb = &diff.constraint_changes.abb_sbb_projection_changes;
    assert!(abb.abb_added.contains(&"c3".to_string()));
    assert!(abb.abb_removed.contains(&"c1".to_string()));
    assert!(abb.sbb_added.contains(&"c3".to_string()));
    assert!(abb.sbb_removed.is_empty());
}

// S11: Coverage metric changes
#[test]
fn test_diff_coverage_metric_changes() {
    let (mut a, mut b) = two_different_manifests();
    a["coverage"] = json!({"percent": 75});
    b["coverage"] = json!({"percent": 90});

    let diff = compute_diff(&to_bytes(&a), &to_bytes(&b)).unwrap();
    assert!(diff.coverage_changes.changed);
    assert_eq!(diff.coverage_changes.old_value, json!({"percent": 75}));
    assert_eq!(diff.coverage_changes.new_value, json!({"percent": 90}));
}

// S12: Exception list changes
#[test]
fn test_diff_exception_list_changes() {
    let (mut a, mut b) = two_different_manifests();
    a["exceptions"] = json!(["exc-1"]);
    b["exceptions"] = json!(["exc-1", "exc-2"]);

    let diff = compute_diff(&to_bytes(&a), &to_bytes(&b)).unwrap();
    assert!(diff.exception_changes.added.contains(&"exc-2".to_string()));
    assert!(diff.exception_changes.removed.is_empty());
}

// S13: policy_ref/profile_ref change
#[test]
fn test_diff_metadata_policy_ref_change() {
    let (mut a, mut b) = two_different_manifests();
    a["policy_ref"] = json!("policy/v1@0");
    b["policy_ref"] = json!("policy/v2@0");
    a["profile_ref"] = json!("profile/default@0");
    b["profile_ref"] = json!("profile/strict@1");

    let diff = compute_diff(&to_bytes(&a), &to_bytes(&b)).unwrap();
    assert!(diff
        .metadata_changes
        .changed_fields
        .contains_key("policy_ref"));
    assert!(diff
        .metadata_changes
        .changed_fields
        .contains_key("profile_ref"));
}

// S14: store_schema_version change
#[test]
fn test_diff_metadata_store_schema_version_change() {
    let (mut a, mut b) = two_different_manifests();
    a["store_schema_version"] = json!("0001");
    b["store_schema_version"] = json!("0002");

    let diff = compute_diff(&to_bytes(&a), &to_bytes(&b)).unwrap();
    assert!(diff
        .metadata_changes
        .changed_fields
        .contains_key("store_schema_version"));
}

// S15: Invalid schema_version → InvalidManifest
#[test]
fn test_diff_rejects_invalid_schema_version() {
    let mut a = base_manifest();
    a["manifest_schema_version"] = json!("not-a-number");
    let b = base_manifest();
    let err = compute_diff(&to_bytes(&a), &to_bytes(&b)).unwrap_err();
    assert_eq!(err.kind(), ExErrorKind::InvalidManifest);
}

// S16: Determinism guard — two identical inputs produce identical JSON
#[test]
fn test_diff_determinism_guard_detects_violation() {
    // Verify the round-trip guard succeeds for valid diffs (no false positives)
    let (a, b) = two_different_manifests();
    let diff = compute_diff(&to_bytes(&a), &to_bytes(&b)).unwrap();
    // Round-trip must be stable
    let s1 = serde_json::to_string(&diff).unwrap();
    let reparsed: ettlex_core::diff::model::SnapshotDiff = serde_json::from_str(&s1).unwrap();
    let s2 = serde_json::to_string(&reparsed).unwrap();
    assert_eq!(s1, s2, "diff JSON must be stable across round-trips");
}

// S17: Missing semantic_manifest_digest → MissingField
#[test]
fn test_diff_missing_semantic_manifest_digest() {
    let mut a = base_manifest();
    let b = base_manifest();
    let obj = a.as_object_mut().unwrap();
    obj.remove("semantic_manifest_digest");
    let err = compute_diff(&to_bytes(&a), &to_bytes(&b)).unwrap_err();
    assert_eq!(err.kind(), ExErrorKind::MissingField);
}

// S18: Large manifests complete within budget (basic smoke test, no strict timing)
#[test]
fn test_diff_large_manifests_efficiency() {
    let mut a = base_manifest();
    let mut b = base_manifest();
    // 200 EPs
    let ept_a: Vec<Value> = (0..200)
        .map(|i| {
            json!({
                "ep_id": format!("ep:root:{}", i),
                "ordinal": i,
                "normative": true,
                "ep_digest": format!("{:064x}", i)
            })
        })
        .collect();
    let ept_b: Vec<Value> = (0..200)
        .map(|i| {
            json!({
                "ep_id": format!("ep:root:{}", i),
                "ordinal": i,
                "normative": true,
                "ep_digest": format!("{:064x}", i + 1)
            })
        })
        .collect();
    a["ept"] = Value::Array(ept_a);
    b["ept"] = Value::Array(ept_b);
    a["semantic_manifest_digest"] =
        json!("aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa");
    b["semantic_manifest_digest"] =
        json!("bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb");
    a["manifest_digest"] =
        json!("cccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccc");
    b["manifest_digest"] =
        json!("dddddddddddddddddddddddddddddddddddddddddddddddddddddddddddddddd");

    let diff = compute_diff(&to_bytes(&a), &to_bytes(&b)).unwrap();
    assert_eq!(diff.ep_content_changes.changed_eps.len(), 200);
}

// S19: Byte-identical manifests → identical classification
#[test]
fn test_diff_byte_identical_manifests() {
    let a = base_manifest();
    let a_bytes = to_bytes(&a);
    let diff = compute_diff(&a_bytes, &a_bytes).unwrap();
    assert_eq!(diff.classification, DiffClassification::Identical);
    assert_eq!(diff.severity, DiffSeverity::None);
}

// S20: Constraint ref additions with empty ABB/SBB
#[test]
fn test_diff_constraint_ref_additions_empty_abb_sbb() {
    let (a, mut b) = two_different_manifests();
    let digest_b = "cccc000000000000000000000000000000000000000000000000000000000000";
    let families_b = {
        let mut m = std::collections::BTreeMap::new();
        m.insert("ABB", digest_b);
        m
    };
    b["constraints"] = json!({
        "declared_refs": ["c1"],
        "families": {
            "ABB": {"status": "UNCOMPUTED", "active_refs": ["c1"], "outcomes": [], "evidence": [], "digest": digest_b}
        },
        "applicable_abb": [],
        "resolved_sbb": [],
        "resolution_evidence": [],
        "constraints_digest": constraints_digest_for(&["c1"], &families_b)
    });

    let diff = compute_diff(&to_bytes(&a), &to_bytes(&b)).unwrap();
    assert!(diff
        .constraint_changes
        .declared_ref_changes
        .added
        .contains(&"c1".to_string()));
    let abb = &diff.constraint_changes.abb_sbb_projection_changes;
    assert!(abb.abb_added.is_empty());
    assert!(abb.sbb_added.is_empty());
}

// S21: Unknown constraint family is opaque (no error)
#[test]
fn test_diff_unknown_constraint_family_opaque() {
    let (a, mut b) = two_different_manifests();
    let digest = "eeee000000000000000000000000000000000000000000000000000000000000";
    let families = {
        let mut m = std::collections::BTreeMap::new();
        m.insert("FUTURE_FAMILY", digest);
        m
    };
    b["constraints"] = json!({
        "declared_refs": ["c1"],
        "families": {
            "FUTURE_FAMILY": {"status": "UNCOMPUTED", "active_refs": ["c1"], "outcomes": [], "evidence": [], "digest": digest}
        },
        "applicable_abb": [],
        "resolved_sbb": [],
        "resolution_evidence": [],
        "constraints_digest": constraints_digest_for(&["c1"], &families)
    });

    // Must not error; family is reported opaquely
    let diff = compute_diff(&to_bytes(&a), &to_bytes(&b)).unwrap();
    assert!(diff
        .constraint_changes
        .family_changes
        .contains_key("FUTURE_FAMILY"));
}

// S22: Unknown manifest field present in both — known diffs still correct
#[test]
fn test_diff_unknown_manifest_field_known_diffs_correct() {
    let (mut a, mut b) = two_different_manifests();
    // Both have the same unknown field → not in unknown_changes.changed_fields
    a["extra_field"] = json!("same_value");
    b["extra_field"] = json!("same_value");
    // But change policy_ref so there's a known diff
    a["policy_ref"] = json!("policy/v1@0");
    b["policy_ref"] = json!("policy/v2@0");

    let diff = compute_diff(&to_bytes(&a), &to_bytes(&b)).unwrap();
    assert!(diff
        .metadata_changes
        .changed_fields
        .contains_key("policy_ref"));
    assert!(!diff
        .unknown_changes
        .changed_fields
        .contains(&"extra_field".to_string()));
}

// S23: Missing constraints field → MissingField
#[test]
fn test_diff_missing_constraints_field() {
    let mut a = base_manifest();
    let b = base_manifest();
    let obj = a.as_object_mut().unwrap();
    obj.remove("constraints");
    let err = compute_diff(&to_bytes(&a), &to_bytes(&b)).unwrap_err();
    assert_eq!(err.kind(), ExErrorKind::MissingField);
}

// S24: Deterministic under randomized JSON input order
#[test]
fn test_diff_deterministic_under_randomized_input_order() {
    // Build manifests with same data but different JSON key ordering
    // serde_json preserves insertion order for Value::Object, so we can vary it
    let a1 = base_manifest();
    // Re-build with reversed key insertion (manually)
    let mut a2_obj = serde_json::Map::new();
    let a1_obj = a1.as_object().unwrap();
    // Insert keys in reverse order
    for (k, v) in a1_obj.iter().rev() {
        a2_obj.insert(k.clone(), v.clone());
    }
    let a2 = Value::Object(a2_obj);

    let (_, b) = two_different_manifests();
    let diff1 = compute_diff(&to_bytes(&a1), &to_bytes(&b)).unwrap();
    let diff2 = compute_diff(&to_bytes(&a2), &to_bytes(&b)).unwrap();
    // Both parse to the same typed struct → same diff
    assert_eq!(
        diff1.identity.a_manifest_digest,
        diff2.identity.a_manifest_digest
    );
    assert_eq!(diff1.classification, diff2.classification);
    assert_eq!(diff1.severity, diff2.severity);
}

// S25: InvariantViolation when envelope digest disagrees
#[test]
fn test_diff_invariant_violation_constraints_digest() {
    let (mut a, b) = two_different_manifests();
    // Set an obviously wrong constraints_digest on manifest A
    a["constraints"]["constraints_digest"] =
        json!("0000000000000000000000000000000000000000000000000000000000000000");

    let diff = compute_diff(&to_bytes(&a), &to_bytes(&b)).unwrap();
    // Diff still succeeds (non-fatal), but violation is recorded
    let violations: Vec<&InvariantViolationEntry> = diff
        .invariant_violations
        .iter()
        .filter(|v| {
            matches!(
                v,
                InvariantViolationEntry::ConstraintsEnvelopeDigestMismatch { which, .. }
                if which == "a"
            )
        })
        .collect();
    assert!(
        !violations.is_empty(),
        "expected invariant violation for manifest A"
    );
}

// S26: Same semantic_manifest_digest → no_semantic_change
#[test]
fn test_diff_no_semantic_change_different_manifest_digest() {
    let mut a = base_manifest();
    let mut b = base_manifest();
    let sem = "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa";
    a["semantic_manifest_digest"] = json!(sem);
    b["semantic_manifest_digest"] = json!(sem);
    a["manifest_digest"] =
        json!("1111111111111111111111111111111111111111111111111111111111111111");
    b["manifest_digest"] =
        json!("2222222222222222222222222222222222222222222222222222222222222222");
    // Different created_at too
    a["created_at"] = json!("2026-01-01T00:00:00Z");
    b["created_at"] = json!("2026-03-01T00:00:00Z");

    let diff = compute_diff(&to_bytes(&a), &to_bytes(&b)).unwrap();
    assert_eq!(diff.classification, DiffClassification::NoSemanticChange);
    assert_eq!(diff.severity, DiffSeverity::None);
    // Identity digests must reflect both sides
    assert_ne!(
        diff.identity.a_manifest_digest,
        diff.identity.b_manifest_digest
    );
}
