//! Human-readable summary renderer for snapshot diffs.

use crate::diff::model::{DiffClassification, DiffSeverity, SnapshotDiff};

/// Render a human-readable Markdown/text summary of a [`SnapshotDiff`].
///
/// The summary is intended for review workflows and approval displays.
/// It is informational only and does not affect the structured diff.
pub fn render_human_summary(diff: &SnapshotDiff) -> String {
    let mut out = String::new();

    // Header
    out.push_str("## Snapshot Diff\n\n");

    // Classification and severity
    let class_label = match &diff.classification {
        DiffClassification::Identical => "Identical",
        DiffClassification::NoSemanticChange => "No Semantic Change",
        DiffClassification::Changed => "Changed",
    };
    let severity_label = match &diff.severity {
        DiffSeverity::None => "None",
        DiffSeverity::Informational => "Informational",
        DiffSeverity::Semantic => "Semantic",
        DiffSeverity::Breaking => "Breaking",
    };
    out.push_str(&format!(
        "**Classification**: {class_label}  \n**Severity**: {severity_label}\n\n"
    ));

    // Identity
    out.push_str("### Identity\n\n");
    out.push_str(&format!(
        "| | Manifest Digest | Semantic Digest | EPT Digest |\n\
         |---|---|---|---|\n\
         | A | `{}` | `{}` | `{}` |\n\
         | B | `{}` | `{}` | `{}` |\n\n",
        short(&diff.identity.a_manifest_digest),
        short(&diff.identity.a_semantic_manifest_digest),
        short(&diff.identity.a_ept_digest),
        short(&diff.identity.b_manifest_digest),
        short(&diff.identity.b_semantic_manifest_digest),
        short(&diff.identity.b_ept_digest),
    ));

    if matches!(
        diff.classification,
        DiffClassification::Identical | DiffClassification::NoSemanticChange
    ) {
        out.push_str("_No semantic changes detected._\n");
        return out;
    }

    // EPT changes
    if diff.ept_changes.changed {
        out.push_str("### EPT Changes\n\n");
        if !diff.ept_changes.added_eps.is_empty() {
            out.push_str(&format!(
                "- **Added EPs** ({}): {}\n",
                diff.ept_changes.added_eps.len(),
                diff.ept_changes.added_eps.join(", ")
            ));
        }
        if !diff.ept_changes.removed_eps.is_empty() {
            out.push_str(&format!(
                "- **Removed EPs** ({}): {}\n",
                diff.ept_changes.removed_eps.len(),
                diff.ept_changes.removed_eps.join(", ")
            ));
        }
        if diff.ept_changes.ordering_changed {
            out.push_str("- **Ordering changed**\n");
        }
        out.push('\n');
    }

    // EP content changes
    if !diff.ep_content_changes.changed_eps.is_empty() {
        out.push_str("### EP Content Changes\n\n");
        for ep_id in &diff.ep_content_changes.changed_eps {
            out.push_str(&format!("- `{}` (digest changed)\n", ep_id));
        }
        out.push('\n');
    }

    // Constraint changes
    let cc = &diff.constraint_changes;
    let has_cc = !cc.declared_ref_changes.added.is_empty()
        || !cc.declared_ref_changes.removed.is_empty()
        || !cc.family_changes.is_empty()
        || cc.constraints_digest_change.is_some();
    if has_cc {
        out.push_str("### Constraint Changes\n\n");
        if !cc.declared_ref_changes.added.is_empty() {
            out.push_str(&format!(
                "- **Added refs**: {}\n",
                cc.declared_ref_changes.added.join(", ")
            ));
        }
        if !cc.declared_ref_changes.removed.is_empty() {
            out.push_str(&format!(
                "- **Removed refs**: {}\n",
                cc.declared_ref_changes.removed.join(", ")
            ));
        }
        for (family, entry) in &cc.family_changes {
            if entry.added {
                out.push_str(&format!("- **Family added**: `{}`\n", family));
            } else if entry.removed {
                out.push_str(&format!("- **Family removed**: `{}`\n", family));
            } else if entry.digest_changed {
                out.push_str(&format!(
                    "- **Family changed**: `{}` (digest changed)\n",
                    family
                ));
            }
        }
        out.push('\n');
    }

    // Coverage changes
    if diff.coverage_changes.changed {
        out.push_str("### Coverage Changes\n\n");
        out.push_str("- Coverage metrics changed\n\n");
    }

    // Exception changes
    if !diff.exception_changes.added.is_empty() || !diff.exception_changes.removed.is_empty() {
        out.push_str("### Exception Changes\n\n");
        if !diff.exception_changes.added.is_empty() {
            out.push_str(&format!(
                "- **Added**: {}\n",
                diff.exception_changes.added.join(", ")
            ));
        }
        if !diff.exception_changes.removed.is_empty() {
            out.push_str(&format!(
                "- **Removed**: {}\n",
                diff.exception_changes.removed.join(", ")
            ));
        }
        out.push('\n');
    }

    // Metadata changes
    if !diff.metadata_changes.changed_fields.is_empty() {
        out.push_str("### Metadata Changes\n\n");
        for (field, change) in &diff.metadata_changes.changed_fields {
            out.push_str(&format!(
                "- **{}**: `{}` → `{}`\n",
                field, change.old, change.new
            ));
        }
        out.push('\n');
    }

    // Unknown changes
    if !diff.unknown_changes.added_fields.is_empty()
        || !diff.unknown_changes.removed_fields.is_empty()
        || !diff.unknown_changes.changed_fields.is_empty()
    {
        out.push_str("### Unknown Field Changes\n\n");
        if !diff.unknown_changes.added_fields.is_empty() {
            out.push_str(&format!(
                "- **Added fields**: {}\n",
                diff.unknown_changes.added_fields.join(", ")
            ));
        }
        if !diff.unknown_changes.removed_fields.is_empty() {
            out.push_str(&format!(
                "- **Removed fields**: {}\n",
                diff.unknown_changes.removed_fields.join(", ")
            ));
        }
        if !diff.unknown_changes.changed_fields.is_empty() {
            out.push_str(&format!(
                "- **Changed fields**: {}\n",
                diff.unknown_changes.changed_fields.join(", ")
            ));
        }
        out.push('\n');
    }

    // Invariant violations
    if !diff.invariant_violations.is_empty() {
        out.push_str("### ⚠ Invariant Violations\n\n");
        for v in &diff.invariant_violations {
            match v {
                crate::diff::model::InvariantViolationEntry::ConstraintsEnvelopeDigestMismatch {
                    which,
                    computed,
                    recorded,
                } => {
                    out.push_str(&format!(
                        "- Manifest {which}: constraints_digest mismatch \
                         (recorded `{}`, computed `{}`)\n",
                        short(recorded),
                        short(computed)
                    ));
                }
            }
        }
        out.push('\n');
    }

    out
}

/// Return the first 12 characters of a digest for display purposes.
fn short(digest: &str) -> &str {
    let end = digest.len().min(12);
    &digest[..end]
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::diff::engine::compute_diff;
    use serde_json::{json, Value};

    fn base() -> Value {
        json!({
            "manifest_schema_version": 1,
            "created_at": "2026-01-01T00:00:00Z",
            "policy_ref": "policy/default@0",
            "profile_ref": "profile/default@0",
            "ept": [
                {"ep_id": "ep:root:0", "ordinal": 0, "normative": true,
                 "ep_digest": "aa00000000000000000000000000000000000000000000000000000000000000"}
            ],
            "constraints": {
                "declared_refs": [],
                "families": {},
                "applicable_abb": [],
                "resolved_sbb": [],
                "resolution_evidence": [],
                "constraints_digest": "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855"
            },
            "coverage": {},
            "exceptions": [],
            "root_ettle_id": "ettle:root",
            "ept_digest": "0001",
            "manifest_digest": "0002",
            "semantic_manifest_digest": "0003",
            "store_schema_version": "0001",
            "seed_digest": null
        })
    }

    fn bytes(v: &Value) -> Vec<u8> {
        serde_json::to_vec(v).unwrap()
    }

    #[test]
    fn test_summary_identical() {
        let a = base();
        let diff = compute_diff(&bytes(&a), &bytes(&a)).unwrap();
        let s = render_human_summary(&diff);
        assert!(s.contains("Identical"));
        assert!(s.contains("_No semantic changes detected._"));
    }

    #[test]
    fn test_summary_no_semantic_change() {
        let mut a = base();
        let mut b = base();
        let sem = "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa";
        a["semantic_manifest_digest"] = json!(sem);
        b["semantic_manifest_digest"] = json!(sem);
        a["manifest_digest"] =
            json!("1111111111111111111111111111111111111111111111111111111111111111");
        b["manifest_digest"] =
            json!("2222222222222222222222222222222222222222222222222222222222222222");
        b["created_at"] = json!("2026-06-01T00:00:00Z");
        let diff = compute_diff(&bytes(&a), &bytes(&b)).unwrap();
        let s = render_human_summary(&diff);
        assert!(s.contains("No Semantic Change"));
    }

    #[test]
    fn test_summary_ept_changes() {
        let mut a = base();
        let mut b = base();
        a["semantic_manifest_digest"] =
            json!("aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa");
        b["semantic_manifest_digest"] =
            json!("bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb");
        a["manifest_digest"] =
            json!("cccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccc");
        b["manifest_digest"] =
            json!("dddddddddddddddddddddddddddddddddddddddddddddddddddddddddddddddd");
        b["ept"] = json!([
            {"ep_id": "ep:root:0", "ordinal": 0, "normative": true,
             "ep_digest": "aa00000000000000000000000000000000000000000000000000000000000000"},
            {"ep_id": "ep:root:1", "ordinal": 1, "normative": true,
             "ep_digest": "bb00000000000000000000000000000000000000000000000000000000000000"}
        ]);
        let diff = compute_diff(&bytes(&a), &bytes(&b)).unwrap();
        let s = render_human_summary(&diff);
        assert!(s.contains("EPT Changes"));
        assert!(s.contains("ep:root:1"));
    }

    #[test]
    fn test_summary_ep_content_changes() {
        let mut a = base();
        let mut b = base();
        a["semantic_manifest_digest"] =
            json!("aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa");
        b["semantic_manifest_digest"] =
            json!("bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb");
        a["manifest_digest"] =
            json!("cccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccc");
        b["manifest_digest"] =
            json!("dddddddddddddddddddddddddddddddddddddddddddddddddddddddddddddddd");
        b["ept"] = json!([
            {"ep_id": "ep:root:0", "ordinal": 0, "normative": true,
             "ep_digest": "ff00000000000000000000000000000000000000000000000000000000000000"}
        ]);
        let diff = compute_diff(&bytes(&a), &bytes(&b)).unwrap();
        let s = render_human_summary(&diff);
        assert!(s.contains("EP Content Changes"));
    }

    #[test]
    fn test_summary_constraint_changes() {
        let mut a = base();
        let mut b = base();
        a["semantic_manifest_digest"] =
            json!("aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa");
        b["semantic_manifest_digest"] =
            json!("bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb");
        a["manifest_digest"] =
            json!("cccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccc");
        b["manifest_digest"] =
            json!("dddddddddddddddddddddddddddddddddddddddddddddddddddddddddddddddd");
        b["constraints"] = json!({
            "declared_refs": ["c1"],
            "families": {
                "ABB": {"status": "UNCOMPUTED", "active_refs": ["c1"], "outcomes": [], "evidence": [],
                        "digest": "abcd000000000000000000000000000000000000000000000000000000000000"}
            },
            "applicable_abb": [],
            "resolved_sbb": [],
            "resolution_evidence": [],
            "constraints_digest": "abcd000000000000000000000000000000000000000000000000000000000001"
        });
        let diff = compute_diff(&bytes(&a), &bytes(&b)).unwrap();
        let s = render_human_summary(&diff);
        assert!(s.contains("Constraint Changes"));
    }

    #[test]
    fn test_summary_coverage_changes() {
        let mut a = base();
        let mut b = base();
        a["semantic_manifest_digest"] =
            json!("aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa");
        b["semantic_manifest_digest"] =
            json!("bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb");
        a["manifest_digest"] =
            json!("cccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccc");
        b["manifest_digest"] =
            json!("dddddddddddddddddddddddddddddddddddddddddddddddddddddddddddddddd");
        a["coverage"] = json!({"percent": 75});
        b["coverage"] = json!({"percent": 90});
        let diff = compute_diff(&bytes(&a), &bytes(&b)).unwrap();
        let s = render_human_summary(&diff);
        assert!(s.contains("Coverage Changes"));
    }

    #[test]
    fn test_summary_exception_changes() {
        let mut a = base();
        let mut b = base();
        a["semantic_manifest_digest"] =
            json!("aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa");
        b["semantic_manifest_digest"] =
            json!("bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb");
        a["manifest_digest"] =
            json!("cccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccc");
        b["manifest_digest"] =
            json!("dddddddddddddddddddddddddddddddddddddddddddddddddddddddddddddddd");
        a["exceptions"] = json!(["exc-1"]);
        b["exceptions"] = json!(["exc-2"]);
        let diff = compute_diff(&bytes(&a), &bytes(&b)).unwrap();
        let s = render_human_summary(&diff);
        assert!(s.contains("Exception Changes"));
    }

    #[test]
    fn test_summary_metadata_changes() {
        let mut a = base();
        let mut b = base();
        a["semantic_manifest_digest"] =
            json!("aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa");
        b["semantic_manifest_digest"] =
            json!("bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb");
        a["manifest_digest"] =
            json!("cccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccc");
        b["manifest_digest"] =
            json!("dddddddddddddddddddddddddddddddddddddddddddddddddddddddddddddddd");
        a["policy_ref"] = json!("policy/v1@0");
        b["policy_ref"] = json!("policy/v2@0");
        let diff = compute_diff(&bytes(&a), &bytes(&b)).unwrap();
        let s = render_human_summary(&diff);
        assert!(s.contains("Metadata Changes"));
        assert!(s.contains("policy_ref"));
    }

    #[test]
    fn test_summary_unknown_field_changes() {
        let mut a = base();
        let mut b = base();
        a["semantic_manifest_digest"] =
            json!("aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa");
        b["semantic_manifest_digest"] =
            json!("bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb");
        a["manifest_digest"] =
            json!("cccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccc");
        b["manifest_digest"] =
            json!("dddddddddddddddddddddddddddddddddddddddddddddddddddddddddddddddd");
        a["future_field"] = json!("old_value");
        b["future_field"] = json!("new_value");
        let diff = compute_diff(&bytes(&a), &bytes(&b)).unwrap();
        let s = render_human_summary(&diff);
        assert!(s.contains("Unknown Field Changes"));
        assert!(s.contains("future_field"));
    }

    #[test]
    fn test_summary_invariant_violation() {
        let mut a = base();
        let mut b = base();
        a["semantic_manifest_digest"] =
            json!("aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa");
        b["semantic_manifest_digest"] =
            json!("bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb");
        a["manifest_digest"] =
            json!("cccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccc");
        b["manifest_digest"] =
            json!("dddddddddddddddddddddddddddddddddddddddddddddddddddddddddddddddd");
        a["constraints"]["constraints_digest"] =
            json!("0000000000000000000000000000000000000000000000000000000000000000");
        let diff = compute_diff(&bytes(&a), &bytes(&b)).unwrap();
        let s = render_human_summary(&diff);
        assert!(s.contains("Invariant Violations"));
        assert!(s.contains("constraints_digest mismatch"));
    }

    #[test]
    fn test_summary_ordering_changed() {
        let mut a = base();
        let mut b = base();
        a["semantic_manifest_digest"] =
            json!("aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa");
        b["semantic_manifest_digest"] =
            json!("bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb");
        a["manifest_digest"] =
            json!("cccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccc");
        b["manifest_digest"] =
            json!("dddddddddddddddddddddddddddddddddddddddddddddddddddddddddddddddd");
        a["ept"] = json!([
            {"ep_id": "ep:root:0", "ordinal": 0, "normative": true,
             "ep_digest": "aa00000000000000000000000000000000000000000000000000000000000000"},
            {"ep_id": "ep:root:1", "ordinal": 1, "normative": true,
             "ep_digest": "bb00000000000000000000000000000000000000000000000000000000000000"}
        ]);
        b["ept"] = json!([
            {"ep_id": "ep:root:1", "ordinal": 0, "normative": true,
             "ep_digest": "bb00000000000000000000000000000000000000000000000000000000000000"},
            {"ep_id": "ep:root:0", "ordinal": 1, "normative": true,
             "ep_digest": "aa00000000000000000000000000000000000000000000000000000000000000"}
        ]);
        let diff = compute_diff(&bytes(&a), &bytes(&b)).unwrap();
        let s = render_human_summary(&diff);
        assert!(s.contains("EPT Changes"));
        assert!(s.contains("Ordering changed"));
    }

    #[test]
    fn test_summary_family_removed() {
        let mut a = base();
        let mut b = base();
        a["semantic_manifest_digest"] =
            json!("aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa");
        b["semantic_manifest_digest"] =
            json!("bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb");
        a["manifest_digest"] =
            json!("cccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccc");
        b["manifest_digest"] =
            json!("dddddddddddddddddddddddddddddddddddddddddddddddddddddddddddddddd");
        a["constraints"] = json!({
            "declared_refs": ["c1"],
            "families": {
                "ABB": {"status": "UNCOMPUTED", "active_refs": ["c1"], "outcomes": [], "evidence": [],
                        "digest": "abcd000000000000000000000000000000000000000000000000000000000000"}
            },
            "applicable_abb": [],
            "resolved_sbb": [],
            "resolution_evidence": [],
            "constraints_digest": "abcd000000000000000000000000000000000000000000000000000000000001"
        });
        let diff = compute_diff(&bytes(&a), &bytes(&b)).unwrap();
        let s = render_human_summary(&diff);
        assert!(s.contains("Constraint Changes"));
        assert!(s.contains("ABB"));
    }

    #[test]
    fn test_summary_unknown_fields_removed() {
        let mut a = base();
        let mut b = base();
        a["semantic_manifest_digest"] =
            json!("aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa");
        b["semantic_manifest_digest"] =
            json!("bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb");
        a["manifest_digest"] =
            json!("cccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccc");
        b["manifest_digest"] =
            json!("dddddddddddddddddddddddddddddddddddddddddddddddddddddddddddddddd");
        a["gone_field"] = json!("was_here");
        let diff = compute_diff(&bytes(&a), &bytes(&b)).unwrap();
        let s = render_human_summary(&diff);
        assert!(s.contains("Unknown Field Changes"));
        assert!(s.contains("gone_field"));
    }
}
