//! Snapshot diff computation engine.
//!
//! The core entry point is [`compute_diff`], which accepts raw manifest bytes
//! for two snapshots and produces a [`SnapshotDiff`].

#![allow(clippy::result_large_err)]

use crate::diff::model::{
    AbbSbbProjectionChanges, ConstraintChanges, CoverageChanges, DeclaredRefChanges,
    DiffClassification, DiffIdentity, DiffSeverity, DigestChange, EpContentChanges, EptChanges,
    ExceptionChanges, FamilyDiffEntry, InvariantViolationEntry, MetadataChanges,
    MetadataFieldChange, SnapshotDiff, UnknownChanges,
};
use crate::errors::{ExError, ExErrorKind};
use crate::snapshot::manifest::SnapshotManifest;
use serde_json::Value;
use sha2::{Digest as _, Sha256};
use std::collections::{BTreeMap, BTreeSet};

/// Known top-level manifest fields used for unknown-field detection.
const KNOWN_FIELDS: &[&str] = &[
    "manifest_schema_version",
    "created_at",
    "policy_ref",
    "profile_ref",
    "ept",
    "constraints",
    "coverage",
    "exceptions",
    "root_ettle_id",
    "ept_digest",
    "manifest_digest",
    "semantic_manifest_digest",
    "store_schema_version",
    "seed_digest",
];

/// Parse raw manifest bytes into a typed `SnapshotManifest` and the raw JSON `Value`.
///
/// The raw `Value` is returned alongside the typed manifest so that the caller can
/// detect unknown top-level fields without a second parse.
///
/// # Errors
///
/// - `InvalidManifest` — bytes are not valid UTF-8, not valid JSON, or
///   `manifest_schema_version` is not an integer
/// - `MissingField` — `semantic_manifest_digest` or `constraints` key absent
pub fn parse_manifest_bytes(bytes: &[u8]) -> Result<(SnapshotManifest, Value), ExError> {
    // 1. UTF-8 decode
    let text = std::str::from_utf8(bytes).map_err(|e| {
        ExError::new(ExErrorKind::InvalidManifest)
            .with_op("parse_manifest_bytes")
            .with_message(format!("manifest is not valid UTF-8: {}", e))
    })?;

    // 2. JSON parse to generic Value
    let raw: Value = serde_json::from_str(text).map_err(|e| {
        ExError::new(ExErrorKind::InvalidManifest)
            .with_op("parse_manifest_bytes")
            .with_message(format!("manifest is not valid JSON: {}", e))
    })?;

    let obj = raw.as_object().ok_or_else(|| {
        ExError::new(ExErrorKind::InvalidManifest)
            .with_op("parse_manifest_bytes")
            .with_message("manifest JSON root must be an object")
    })?;

    // 3. schema_version must be an integer (accept both manifest_schema_version key names)
    let schema_version_key = "manifest_schema_version";
    if let Some(sv) = obj.get(schema_version_key) {
        if !sv.is_number() || sv.as_u64().is_none() {
            return Err(ExError::new(ExErrorKind::InvalidManifest)
                .with_op("parse_manifest_bytes")
                .with_message(format!(
                    "`{}` must be an unsigned integer, got: {}",
                    schema_version_key, sv
                )));
        }
    }
    // If absent we still allow serde to decide (it will error in step 6 if required)

    // 4. semantic_manifest_digest must be present
    if !obj.contains_key("semantic_manifest_digest") {
        return Err(ExError::new(ExErrorKind::MissingField)
            .with_op("parse_manifest_bytes")
            .with_message("required field `semantic_manifest_digest` is absent"));
    }

    // 5. constraints must be present
    if !obj.contains_key("constraints") {
        return Err(ExError::new(ExErrorKind::MissingField)
            .with_op("parse_manifest_bytes")
            .with_message("required field `constraints` is absent"));
    }

    // 6. Full typed deserialisation
    let manifest: SnapshotManifest = serde_json::from_value(raw.clone()).map_err(|e| {
        ExError::new(ExErrorKind::InvalidManifest)
            .with_op("parse_manifest_bytes")
            .with_message(format!("failed to deserialize manifest: {}", e))
    })?;

    Ok((manifest, raw))
}

/// Recompute the `constraints_digest` from the envelope data stored in the manifest.
///
/// Mirrors the algorithm in `constraint_engine::evaluate` so that the diff engine
/// can detect envelope/digest disagreements without re-running the constraint engine.
fn recompute_constraints_digest(manifest: &SnapshotManifest) -> String {
    let envelope = &manifest.constraints;
    let ref_ids: Vec<&str> = envelope.declared_refs.iter().map(|s| s.as_str()).collect();
    let family_digests: Vec<(&str, &str)> = envelope
        .families
        .iter()
        .map(|(k, v)| (k.as_str(), v.digest.as_str()))
        .collect();

    let digest_input = vec![
        serde_json::to_value(&ref_ids).unwrap_or(Value::Null),
        serde_json::to_value(&family_digests).unwrap_or(Value::Null),
    ];
    let canonical = serde_json::to_string(&digest_input).unwrap_or_default();
    let mut hasher = Sha256::new();
    hasher.update(canonical.as_bytes());
    hex::encode(hasher.finalize())
}

/// Check constraints envelope integrity and return any violations (non-fatal).
fn check_envelope_invariants(
    which: &str,
    manifest: &SnapshotManifest,
    violations: &mut Vec<InvariantViolationEntry>,
) {
    let computed = recompute_constraints_digest(manifest);
    let recorded = &manifest.constraints.constraints_digest;
    if &computed != recorded {
        violations.push(InvariantViolationEntry::ConstraintsEnvelopeDigestMismatch {
            which: which.to_string(),
            computed,
            recorded: recorded.clone(),
        });
    }
}

/// Compute a set-delta between two ordered lists.
///
/// Returns `(added, removed)` where added = in b but not a, removed = in a but not b.
fn set_delta(a: &[String], b: &[String]) -> (Vec<String>, Vec<String>) {
    let set_a: BTreeSet<&str> = a.iter().map(|s| s.as_str()).collect();
    let set_b: BTreeSet<&str> = b.iter().map(|s| s.as_str()).collect();
    let added: Vec<String> = b
        .iter()
        .filter(|s| !set_a.contains(s.as_str()))
        .cloned()
        .collect();
    let removed: Vec<String> = a
        .iter()
        .filter(|s| !set_b.contains(s.as_str()))
        .cloned()
        .collect();
    (added, removed)
}

/// Compute the maximum severity across a slice of severities.
fn max_severity(severities: &[DiffSeverity]) -> DiffSeverity {
    severities
        .iter()
        .max()
        .cloned()
        .unwrap_or(DiffSeverity::None)
}

/// Compare two `serde_json::Value`s as metadata field values.
fn metadata_field_change(old: &Value, new: &Value) -> Option<MetadataFieldChange> {
    if old != new {
        Some(MetadataFieldChange {
            old: old.clone(),
            new: new.clone(),
        })
    } else {
        None
    }
}

/// Compute a structured, deterministic diff between two snapshot manifests.
///
/// Accepts raw manifest bytes for both sides. Returns a [`SnapshotDiff`]
/// describing all detected changes.
///
/// # Errors
///
/// - `InvalidManifest` — either manifest fails UTF-8/JSON/schema validation
/// - `MissingField` — a required field is absent from either manifest
/// - `DeterminismViolation` — the computed diff fails its internal round-trip
///   sanity check (should never occur in correct builds)
pub fn compute_diff(a_bytes: &[u8], b_bytes: &[u8]) -> Result<SnapshotDiff, ExError> {
    // Parse both manifests
    let (a_manifest, a_raw) = parse_manifest_bytes(a_bytes)?;
    let (b_manifest, b_raw) = parse_manifest_bytes(b_bytes)?;

    // Identity block
    let identity = DiffIdentity {
        a_manifest_digest: a_manifest.manifest_digest.clone(),
        a_semantic_manifest_digest: a_manifest.semantic_manifest_digest.clone(),
        a_ept_digest: a_manifest.ept_digest.clone(),
        b_manifest_digest: b_manifest.manifest_digest.clone(),
        b_semantic_manifest_digest: b_manifest.semantic_manifest_digest.clone(),
        b_ept_digest: b_manifest.ept_digest.clone(),
    };

    // Fast-path: byte-identical manifests
    if a_bytes == b_bytes {
        return Ok(SnapshotDiff {
            diff_schema_version: 1,
            identity,
            classification: DiffClassification::Identical,
            severity: DiffSeverity::None,
            ept_changes: EptChanges {
                changed: false,
                added_eps: Vec::new(),
                removed_eps: Vec::new(),
                ordering_changed: false,
            },
            ep_content_changes: EpContentChanges {
                changed_eps: Vec::new(),
            },
            constraint_changes: ConstraintChanges {
                declared_ref_changes: DeclaredRefChanges {
                    added: Vec::new(),
                    removed: Vec::new(),
                },
                family_changes: BTreeMap::new(),
                abb_sbb_projection_changes: AbbSbbProjectionChanges {
                    abb_added: Vec::new(),
                    abb_removed: Vec::new(),
                    sbb_added: Vec::new(),
                    sbb_removed: Vec::new(),
                },
                constraints_digest_change: None,
            },
            coverage_changes: CoverageChanges {
                changed: false,
                old_value: a_manifest.coverage.clone(),
                new_value: a_manifest.coverage.clone(),
            },
            exception_changes: ExceptionChanges {
                added: Vec::new(),
                removed: Vec::new(),
            },
            metadata_changes: MetadataChanges {
                changed_fields: BTreeMap::new(),
            },
            unknown_changes: UnknownChanges {
                added_fields: Vec::new(),
                removed_fields: Vec::new(),
                changed_fields: Vec::new(),
            },
            invariant_violations: Vec::new(),
        });
    }

    // Semantic identity: same semantic digest → no semantic change
    if a_manifest.semantic_manifest_digest == b_manifest.semantic_manifest_digest {
        return Ok(SnapshotDiff {
            diff_schema_version: 1,
            identity,
            classification: DiffClassification::NoSemanticChange,
            severity: DiffSeverity::None,
            ept_changes: EptChanges {
                changed: false,
                added_eps: Vec::new(),
                removed_eps: Vec::new(),
                ordering_changed: false,
            },
            ep_content_changes: EpContentChanges {
                changed_eps: Vec::new(),
            },
            constraint_changes: ConstraintChanges {
                declared_ref_changes: DeclaredRefChanges {
                    added: Vec::new(),
                    removed: Vec::new(),
                },
                family_changes: BTreeMap::new(),
                abb_sbb_projection_changes: AbbSbbProjectionChanges {
                    abb_added: Vec::new(),
                    abb_removed: Vec::new(),
                    sbb_added: Vec::new(),
                    sbb_removed: Vec::new(),
                },
                constraints_digest_change: None,
            },
            coverage_changes: CoverageChanges {
                changed: false,
                old_value: a_manifest.coverage.clone(),
                new_value: b_manifest.coverage.clone(),
            },
            exception_changes: ExceptionChanges {
                added: Vec::new(),
                removed: Vec::new(),
            },
            metadata_changes: MetadataChanges {
                changed_fields: BTreeMap::new(),
            },
            unknown_changes: UnknownChanges {
                added_fields: Vec::new(),
                removed_fields: Vec::new(),
                changed_fields: Vec::new(),
            },
            invariant_violations: Vec::new(),
        });
    }

    // Invariant violations (non-fatal)
    let mut invariant_violations: Vec<InvariantViolationEntry> = Vec::new();
    check_envelope_invariants("a", &a_manifest, &mut invariant_violations);
    check_envelope_invariants("b", &b_manifest, &mut invariant_violations);

    // EPT changes
    let a_ep_ids: Vec<String> = a_manifest.ept.iter().map(|e| e.ep_id.clone()).collect();
    let b_ep_ids: Vec<String> = b_manifest.ept.iter().map(|e| e.ep_id.clone()).collect();
    let (ept_added, ept_removed) = set_delta(&a_ep_ids, &b_ep_ids);

    // Ordering changed: same set of EP IDs but in a different order
    let a_set: BTreeSet<&str> = a_ep_ids.iter().map(|s| s.as_str()).collect();
    let b_set: BTreeSet<&str> = b_ep_ids.iter().map(|s| s.as_str()).collect();
    let same_set = a_set == b_set;
    let ordering_changed = same_set && a_ep_ids != b_ep_ids;
    let ept_structurally_changed = !ept_added.is_empty() || !ept_removed.is_empty();

    let ept_changes = EptChanges {
        changed: ept_structurally_changed || ordering_changed,
        added_eps: ept_added,
        removed_eps: ept_removed,
        ordering_changed,
    };

    // EP content changes (EPs present in both, digest differs)
    let a_ep_digests: BTreeMap<&str, &str> = a_manifest
        .ept
        .iter()
        .map(|e| (e.ep_id.as_str(), e.ep_digest.as_str()))
        .collect();
    let b_ep_digests: BTreeMap<&str, &str> = b_manifest
        .ept
        .iter()
        .map(|e| (e.ep_id.as_str(), e.ep_digest.as_str()))
        .collect();

    let mut changed_eps: Vec<String> = a_ep_digests
        .iter()
        .filter_map(|(ep_id, a_digest)| {
            b_ep_digests
                .get(ep_id)
                .filter(|b_digest| b_digest != &a_digest)
                .map(|_| ep_id.to_string())
        })
        .collect();
    changed_eps.sort();
    let ep_content_changes = EpContentChanges { changed_eps };

    // Constraint changes
    let a_env = &a_manifest.constraints;
    let b_env = &b_manifest.constraints;

    let (decl_added, decl_removed) = set_delta(&a_env.declared_refs, &b_env.declared_refs);
    let declared_ref_changes = DeclaredRefChanges {
        added: decl_added,
        removed: decl_removed,
    };

    // Per-family diff
    let a_families: BTreeSet<&str> = a_env.families.keys().map(|s| s.as_str()).collect();
    let b_families: BTreeSet<&str> = b_env.families.keys().map(|s| s.as_str()).collect();
    let all_families: BTreeSet<&str> = a_families.union(&b_families).copied().collect();

    let mut family_changes: BTreeMap<String, FamilyDiffEntry> = BTreeMap::new();
    for family in &all_families {
        let a_fc = a_env.families.get(*family);
        let b_fc = b_env.families.get(*family);
        match (a_fc, b_fc) {
            (None, Some(b)) => {
                family_changes.insert(
                    family.to_string(),
                    FamilyDiffEntry {
                        added: true,
                        removed: false,
                        digest_changed: true,
                        old_digest: None,
                        new_digest: Some(b.digest.clone()),
                    },
                );
            }
            (Some(a), None) => {
                family_changes.insert(
                    family.to_string(),
                    FamilyDiffEntry {
                        added: false,
                        removed: true,
                        digest_changed: true,
                        old_digest: Some(a.digest.clone()),
                        new_digest: None,
                    },
                );
            }
            (Some(a), Some(b)) => {
                if a.digest != b.digest {
                    family_changes.insert(
                        family.to_string(),
                        FamilyDiffEntry {
                            added: false,
                            removed: false,
                            digest_changed: true,
                            old_digest: Some(a.digest.clone()),
                            new_digest: Some(b.digest.clone()),
                        },
                    );
                }
            }
            (None, None) => {}
        }
    }

    let (abb_added, abb_removed) = set_delta(&a_env.applicable_abb, &b_env.applicable_abb);
    let (sbb_added, sbb_removed) = set_delta(&a_env.resolved_sbb, &b_env.resolved_sbb);
    let abb_sbb_projection_changes = AbbSbbProjectionChanges {
        abb_added,
        abb_removed,
        sbb_added,
        sbb_removed,
    };

    let constraints_digest_change = if a_env.constraints_digest != b_env.constraints_digest {
        Some(DigestChange {
            old: a_env.constraints_digest.clone(),
            new: b_env.constraints_digest.clone(),
        })
    } else {
        None
    };

    let constraint_changes = ConstraintChanges {
        declared_ref_changes,
        family_changes,
        abb_sbb_projection_changes,
        constraints_digest_change,
    };

    // Coverage changes
    let coverage_changed = a_manifest.coverage != b_manifest.coverage;
    let coverage_changes = CoverageChanges {
        changed: coverage_changed,
        old_value: a_manifest.coverage.clone(),
        new_value: b_manifest.coverage.clone(),
    };

    // Exception changes
    let (exc_added, exc_removed) = set_delta(&a_manifest.exceptions, &b_manifest.exceptions);
    let exception_changes = ExceptionChanges {
        added: exc_added,
        removed: exc_removed,
    };

    // Metadata changes: policy_ref, profile_ref, store_schema_version, manifest_schema_version
    let mut changed_fields: BTreeMap<String, MetadataFieldChange> = BTreeMap::new();

    let meta_fields: &[(&str, &Value, &Value)] = &[
        (
            "policy_ref",
            &Value::String(a_manifest.policy_ref.clone()),
            &Value::String(b_manifest.policy_ref.clone()),
        ),
        (
            "profile_ref",
            &Value::String(a_manifest.profile_ref.clone()),
            &Value::String(b_manifest.profile_ref.clone()),
        ),
        (
            "store_schema_version",
            &Value::String(a_manifest.store_schema_version.clone()),
            &Value::String(b_manifest.store_schema_version.clone()),
        ),
        (
            "manifest_schema_version",
            &Value::Number(a_manifest.manifest_schema_version.into()),
            &Value::Number(b_manifest.manifest_schema_version.into()),
        ),
    ];

    for (name, old_val, new_val) in meta_fields {
        if let Some(change) = metadata_field_change(old_val, new_val) {
            changed_fields.insert(name.to_string(), change);
        }
    }

    let metadata_changes = MetadataChanges { changed_fields };

    // Unknown changes: keys not in KNOWN_FIELDS
    let known_set: BTreeSet<&str> = KNOWN_FIELDS.iter().copied().collect();

    let a_unknown: BTreeSet<&str> = if let Some(obj) = a_raw.as_object() {
        obj.keys()
            .map(|k| k.as_str())
            .filter(|k| !known_set.contains(k))
            .collect()
    } else {
        BTreeSet::new()
    };
    let b_unknown: BTreeSet<&str> = if let Some(obj) = b_raw.as_object() {
        obj.keys()
            .map(|k| k.as_str())
            .filter(|k| !known_set.contains(k))
            .collect()
    } else {
        BTreeSet::new()
    };

    let unk_added: Vec<String> = b_unknown
        .difference(&a_unknown)
        .map(|s| s.to_string())
        .collect();
    let unk_removed: Vec<String> = a_unknown
        .difference(&b_unknown)
        .map(|s| s.to_string())
        .collect();

    // Changed unknown fields: present in both but different value
    let unk_both: BTreeSet<&str> = a_unknown.intersection(&b_unknown).copied().collect();
    let mut unk_changed: Vec<String> = Vec::new();
    if let (Some(a_obj), Some(b_obj)) = (a_raw.as_object(), b_raw.as_object()) {
        for key in &unk_both {
            if a_obj.get(*key) != b_obj.get(*key) {
                unk_changed.push(key.to_string());
            }
        }
    }
    unk_changed.sort();

    let unknown_changes = UnknownChanges {
        added_fields: unk_added,
        removed_fields: unk_removed,
        changed_fields: unk_changed,
    };

    // Severity roll-up
    let mut severities: Vec<DiffSeverity> = Vec::new();

    if ept_changes.added_eps.is_empty() && ept_changes.removed_eps.is_empty() {
        // no structural EPT change
    } else {
        severities.push(DiffSeverity::Breaking);
    }
    if ept_changes.ordering_changed {
        severities.push(DiffSeverity::Semantic);
    }
    if !ep_content_changes.changed_eps.is_empty() {
        severities.push(DiffSeverity::Semantic);
    }
    let has_constraint_changes = !constraint_changes.declared_ref_changes.added.is_empty()
        || !constraint_changes.declared_ref_changes.removed.is_empty()
        || !constraint_changes.family_changes.is_empty()
        || !constraint_changes
            .abb_sbb_projection_changes
            .abb_added
            .is_empty()
        || !constraint_changes
            .abb_sbb_projection_changes
            .abb_removed
            .is_empty()
        || !constraint_changes
            .abb_sbb_projection_changes
            .sbb_added
            .is_empty()
        || !constraint_changes
            .abb_sbb_projection_changes
            .sbb_removed
            .is_empty()
        || constraint_changes.constraints_digest_change.is_some();
    if has_constraint_changes {
        severities.push(DiffSeverity::Semantic);
    }
    if coverage_changes.changed {
        severities.push(DiffSeverity::Informational);
    }
    if !exception_changes.added.is_empty() || !exception_changes.removed.is_empty() {
        severities.push(DiffSeverity::Informational);
    }
    if !metadata_changes.changed_fields.is_empty() {
        severities.push(DiffSeverity::Informational);
    }
    if !unknown_changes.added_fields.is_empty()
        || !unknown_changes.removed_fields.is_empty()
        || !unknown_changes.changed_fields.is_empty()
    {
        severities.push(DiffSeverity::Informational);
    }

    let severity = max_severity(&severities);

    let diff = SnapshotDiff {
        diff_schema_version: 1,
        identity,
        classification: DiffClassification::Changed,
        severity,
        ept_changes,
        ep_content_changes,
        constraint_changes,
        coverage_changes,
        exception_changes,
        metadata_changes,
        unknown_changes,
        invariant_violations,
    };

    // Determinism guard: round-trip through JSON must produce an equal struct
    let serialized = serde_json::to_string(&diff).map_err(|e| {
        ExError::new(ExErrorKind::DeterminismViolation)
            .with_op("compute_diff")
            .with_message(format!("failed to serialize diff: {}", e))
    })?;
    let reparsed: SnapshotDiff = serde_json::from_str(&serialized).map_err(|e| {
        ExError::new(ExErrorKind::DeterminismViolation)
            .with_op("compute_diff")
            .with_message(format!("failed to re-parse diff: {}", e))
    })?;
    if reparsed != diff {
        return Err(ExError::new(ExErrorKind::DeterminismViolation)
            .with_op("compute_diff")
            .with_message("diff is not deterministic: round-trip produced different struct"));
    }

    Ok(diff)
}
