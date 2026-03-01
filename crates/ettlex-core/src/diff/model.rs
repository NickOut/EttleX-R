//! Snapshot diff output types.
//!
//! All types implement `Debug, Clone, Serialize, Deserialize, PartialEq`.
//! Collections use `BTreeMap` and sorted `Vec` for deterministic serialization.

use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

/// The top-level structured diff between two snapshot manifests.
///
/// `diff_schema_version` is always 1 for this implementation.
/// All change sub-structs are populated even when there is no change
/// (empty collections, `changed: false`) to allow uniform downstream processing.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct SnapshotDiff {
    /// Schema version of this diff structure (always 1)
    pub diff_schema_version: u32,
    /// Identity digests for both sides of the diff
    pub identity: DiffIdentity,
    /// High-level classification of the diff
    pub classification: DiffClassification,
    /// Severity of the most significant change
    pub severity: DiffSeverity,
    /// Changes to the EPT structure (added/removed/reordered EPs)
    pub ept_changes: EptChanges,
    /// Changes to EP content digests (same EPs, different content)
    pub ep_content_changes: EpContentChanges,
    /// Changes to the constraints envelope
    pub constraint_changes: ConstraintChanges,
    /// Changes to coverage metrics
    pub coverage_changes: CoverageChanges,
    /// Changes to the exceptions list
    pub exception_changes: ExceptionChanges,
    /// Changes to manifest metadata fields
    pub metadata_changes: MetadataChanges,
    /// Changes to unknown (forward-compatible) manifest fields
    pub unknown_changes: UnknownChanges,
    /// Non-fatal invariant violations detected during diffing
    pub invariant_violations: Vec<InvariantViolationEntry>,
}

/// Digest identity for both manifests being diffed.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct DiffIdentity {
    /// Full manifest digest of snapshot A (includes created_at)
    pub a_manifest_digest: String,
    /// Semantic manifest digest of snapshot A (excludes created_at)
    pub a_semantic_manifest_digest: String,
    /// EPT digest of snapshot A
    pub a_ept_digest: String,
    /// Full manifest digest of snapshot B (includes created_at)
    pub b_manifest_digest: String,
    /// Semantic manifest digest of snapshot B (excludes created_at)
    pub b_semantic_manifest_digest: String,
    /// EPT digest of snapshot B
    pub b_ept_digest: String,
}

/// High-level classification of the diff result.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum DiffClassification {
    /// Both manifests are byte-identical
    Identical,
    /// Manifests differ only in non-semantic fields (e.g. `created_at`)
    NoSemanticChange,
    /// Manifests have at least one semantic difference
    Changed,
}

/// Severity of the most significant change in the diff.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord)]
pub enum DiffSeverity {
    /// No changes at all
    None,
    /// Changes that are purely informational (e.g. metadata, coverage)
    Informational,
    /// Changes that affect semantic meaning (e.g. EP content, constraints)
    Semantic,
    /// Breaking changes (e.g. EPs added/removed from EPT)
    Breaking,
}

/// Changes to the EPT structure.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct EptChanges {
    /// True if any EPT change occurred
    pub changed: bool,
    /// EP IDs present in B but not A
    pub added_eps: Vec<String>,
    /// EP IDs present in A but not B
    pub removed_eps: Vec<String>,
    /// True if the same EP IDs appear in a different order
    pub ordering_changed: bool,
}

/// Changes to EP content digests for EPs present in both manifests.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct EpContentChanges {
    /// EP IDs whose `ep_digest` changed between A and B
    pub changed_eps: Vec<String>,
}

/// Changes to the constraints envelope.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ConstraintChanges {
    /// Changes to the top-level declared refs list
    pub declared_ref_changes: DeclaredRefChanges,
    /// Per-family diff entries (keyed by family name)
    pub family_changes: BTreeMap<String, FamilyDiffEntry>,
    /// Changes to the frozen ABB/SBB projection lists
    pub abb_sbb_projection_changes: AbbSbbProjectionChanges,
    /// Change to the top-level `constraints_digest` field, if any
    pub constraints_digest_change: Option<DigestChange>,
}

/// Set-delta for the declared constraint refs list.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct DeclaredRefChanges {
    /// Constraint IDs in B but not A
    pub added: Vec<String>,
    /// Constraint IDs in A but not B
    pub removed: Vec<String>,
}

/// Diff entry for a single constraint family.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct FamilyDiffEntry {
    /// True if this family was added in B
    pub added: bool,
    /// True if this family was removed in B
    pub removed: bool,
    /// True if the family's digest changed
    pub digest_changed: bool,
    /// Previous digest (None if family was added)
    pub old_digest: Option<String>,
    /// New digest (None if family was removed)
    pub new_digest: Option<String>,
}

/// Changes to the frozen ABB/SBB projection lists.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct AbbSbbProjectionChanges {
    /// ABB entries in B but not A
    pub abb_added: Vec<String>,
    /// ABB entries in A but not B
    pub abb_removed: Vec<String>,
    /// SBB entries in B but not A
    pub sbb_added: Vec<String>,
    /// SBB entries in A but not B
    pub sbb_removed: Vec<String>,
}

/// A change to a single digest field.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct DigestChange {
    /// Value in A
    pub old: String,
    /// Value in B
    pub new: String,
}

/// Changes to the coverage metrics.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct CoverageChanges {
    /// True if the coverage value changed
    pub changed: bool,
    /// Coverage value in A
    pub old_value: serde_json::Value,
    /// Coverage value in B
    pub new_value: serde_json::Value,
}

/// Changes to the exceptions list.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ExceptionChanges {
    /// Exception entries in B but not A
    pub added: Vec<String>,
    /// Exception entries in A but not B
    pub removed: Vec<String>,
}

/// Changes to manifest metadata fields.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct MetadataChanges {
    /// Map from field name to its old/new values for any field that changed
    pub changed_fields: BTreeMap<String, MetadataFieldChange>,
}

/// Old/new values for a changed metadata field.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct MetadataFieldChange {
    /// Value in A
    pub old: serde_json::Value,
    /// Value in B
    pub new: serde_json::Value,
}

/// Changes to unknown (unrecognised) manifest fields.
///
/// Forward-compatible: fields unknown to this version of the diff engine are
/// tracked but not treated as errors.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct UnknownChanges {
    /// Unknown field keys present in B but not A
    pub added_fields: Vec<String>,
    /// Unknown field keys present in A but not B
    pub removed_fields: Vec<String>,
    /// Unknown field keys present in both A and B with different values
    pub changed_fields: Vec<String>,
}

/// A non-fatal invariant violation detected during diffing.
///
/// These are appended to `SnapshotDiff::invariant_violations` and do not
/// prevent the diff from being returned.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(tag = "kind")]
pub enum InvariantViolationEntry {
    /// The `constraints_digest` recorded in the manifest does not match the
    /// digest recomputed from the envelope's `declared_refs` and `families`.
    ConstraintsEnvelopeDigestMismatch {
        /// Which side ("a" or "b") has the mismatch
        which: String,
        /// Digest recomputed from envelope data
        computed: String,
        /// Digest recorded in the manifest
        recorded: String,
    },
}
