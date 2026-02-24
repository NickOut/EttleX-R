//! Decision domain model
//!
//! This module defines the domain model for decisions, which are first-class governance
//! artefacts capturing binding design decisions with portable evidence. Decisions can be
//! linked to EPs, Ettles, Constraints, and other Decisions with explicit relation kinds.
//!
//! Decisions are non-snapshot-semantic in Phase 1 - they do not affect snapshot manifests
//! or semantic digest computation.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::fmt;

/// A decision instance representing a binding design decision
///
/// Decisions capture architectural and design decisions with portable evidence.
/// The evidence_hash is computed from the evidence content to enable verification.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Decision {
    /// Unique identifier (UUIDv7 or explicit ID)
    pub decision_id: String,

    /// Decision title (short summary)
    pub title: String,

    /// Decision status (proposed, accepted, superseded, rejected, etc.)
    pub status: String,

    /// Decision text (the actual decision)
    pub decision_text: String,

    /// Rationale (why this decision was made)
    pub rationale: String,

    /// Alternatives considered (optional)
    pub alternatives_text: Option<String>,

    /// Consequences of this decision (optional)
    pub consequences_text: Option<String>,

    /// Evidence kind (none, excerpt, capture, file)
    pub evidence_kind: String,

    /// Evidence excerpt (portable short text)
    pub evidence_excerpt: Option<String>,

    /// Evidence capture ID (FK to decision_evidence_items)
    pub evidence_capture_id: Option<String>,

    /// Evidence file path (repo-relative path)
    pub evidence_file_path: Option<String>,

    /// SHA-256 hash of evidence content
    pub evidence_hash: String,

    /// Creation timestamp
    pub created_at: DateTime<Utc>,

    /// Last update timestamp
    pub updated_at: DateTime<Utc>,

    /// Deletion timestamp (tombstone pattern)
    pub tombstoned_at: Option<DateTime<Utc>>,
}

impl Decision {
    /// Create a new decision instance
    ///
    /// The evidence_hash is computed from the evidence content for verification.
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        decision_id: String,
        title: String,
        status: String,
        decision_text: String,
        rationale: String,
        alternatives_text: Option<String>,
        consequences_text: Option<String>,
        evidence_kind: String,
        evidence_excerpt: Option<String>,
        evidence_capture_id: Option<String>,
        evidence_file_path: Option<String>,
    ) -> Self {
        let now = Utc::now();
        let evidence_hash =
            Self::compute_evidence_hash(&evidence_kind, &evidence_excerpt, &evidence_file_path);

        Self {
            decision_id,
            title,
            status,
            decision_text,
            rationale,
            alternatives_text,
            consequences_text,
            evidence_kind,
            evidence_excerpt,
            evidence_capture_id,
            evidence_file_path,
            evidence_hash,
            created_at: now,
            updated_at: now,
            tombstoned_at: None,
        }
    }

    /// Compute SHA-256 hash of evidence content
    ///
    /// Hash is computed over the available evidence fields for deterministic verification.
    fn compute_evidence_hash(
        evidence_kind: &str,
        evidence_excerpt: &Option<String>,
        evidence_file_path: &Option<String>,
    ) -> String {
        let mut hasher = Sha256::new();
        hasher.update(evidence_kind.as_bytes());

        if let Some(excerpt) = evidence_excerpt {
            hasher.update(excerpt.as_bytes());
        }

        if let Some(path) = evidence_file_path {
            hasher.update(path.as_bytes());
        }

        format!("{:x}", hasher.finalize())
    }

    /// Check if decision is tombstoned (soft-deleted)
    pub fn is_tombstoned(&self) -> bool {
        self.tombstoned_at.is_some()
    }

    /// Mark decision as tombstoned (soft delete)
    pub fn tombstone(&mut self) {
        self.tombstoned_at = Some(Utc::now());
        self.updated_at = Utc::now();
    }

    /// Update decision fields
    #[allow(clippy::too_many_arguments)]
    pub fn update(
        &mut self,
        title: Option<String>,
        status: Option<String>,
        decision_text: Option<String>,
        rationale: Option<String>,
        alternatives_text: Option<Option<String>>,
        consequences_text: Option<Option<String>>,
        evidence_kind: Option<String>,
        evidence_excerpt: Option<Option<String>>,
        evidence_file_path: Option<Option<String>>,
    ) {
        if let Some(t) = title {
            self.title = t;
        }
        if let Some(s) = status {
            self.status = s;
        }
        if let Some(dt) = decision_text {
            self.decision_text = dt;
        }
        if let Some(r) = rationale {
            self.rationale = r;
        }
        if let Some(at) = alternatives_text {
            self.alternatives_text = at;
        }
        if let Some(ct) = consequences_text {
            self.consequences_text = ct;
        }
        if let Some(ek) = evidence_kind {
            self.evidence_kind = ek;
        }
        if let Some(ee) = evidence_excerpt {
            self.evidence_excerpt = ee;
        }
        if let Some(efp) = evidence_file_path {
            self.evidence_file_path = efp;
        }

        // Recompute hash if evidence changed
        self.evidence_hash = Self::compute_evidence_hash(
            &self.evidence_kind,
            &self.evidence_excerpt,
            &self.evidence_file_path,
        );
        self.updated_at = Utc::now();
    }
}

impl fmt::Display for Decision {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "Decision({}, title={}, status={})",
            self.decision_id, self.title, self.status
        )
    }
}

/// Evidence item capturing portable conversation or document content
///
/// Used to store full evidence content (e.g., conversation captures, meeting notes)
/// separately from the decision record for better normalization.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct DecisionEvidenceItem {
    /// Unique identifier for this evidence capture
    pub evidence_capture_id: String,

    /// Evidence source (mcp_chat_capture, manual_copy, meeting_notes, export)
    pub source: String,

    /// Evidence content (portable text/markdown blob)
    pub content: String,

    /// SHA-256 hash of content bytes
    pub content_hash: String,

    /// Creation timestamp
    pub created_at: DateTime<Utc>,
}

impl DecisionEvidenceItem {
    /// Create a new evidence item
    pub fn new(evidence_capture_id: String, source: String, content: String) -> Self {
        let content_hash = Self::compute_content_hash(&content);

        Self {
            evidence_capture_id,
            source,
            content,
            content_hash,
            created_at: Utc::now(),
        }
    }

    /// Compute SHA-256 hash of content
    fn compute_content_hash(content: &str) -> String {
        let mut hasher = Sha256::new();
        hasher.update(content.as_bytes());
        format!("{:x}", hasher.finalize())
    }
}

impl fmt::Display for DecisionEvidenceItem {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "DecisionEvidenceItem({}, source={})",
            self.evidence_capture_id, self.source
        )
    }
}

/// Decision link relating a decision to a target entity
///
/// Links can have different relation kinds: grounds, constrains, motivates, supersedes, etc.
/// The ordinal field enables deterministic ordering for queries.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct DecisionLink {
    /// Decision identifier
    pub decision_id: String,

    /// Target kind (ep, ettle, constraint, decision)
    pub target_kind: String,

    /// Target identifier
    pub target_id: String,

    /// Relation kind (grounds, constrains, motivates, supersedes)
    pub relation_kind: String,

    /// Ordinal position for deterministic ordering
    pub ordinal: i32,

    /// Creation timestamp
    pub created_at: DateTime<Utc>,

    /// Tombstone timestamp (soft delete)
    pub tombstoned_at: Option<DateTime<Utc>>,
}

impl DecisionLink {
    /// Create a new decision link
    pub fn new(
        decision_id: String,
        target_kind: String,
        target_id: String,
        relation_kind: String,
        ordinal: i32,
    ) -> Self {
        Self {
            decision_id,
            target_kind,
            target_id,
            relation_kind,
            ordinal,
            created_at: Utc::now(),
            tombstoned_at: None,
        }
    }

    /// Check if link is tombstoned
    pub fn is_tombstoned(&self) -> bool {
        self.tombstoned_at.is_some()
    }

    /// Mark link as tombstoned (soft delete)
    pub fn tombstone(&mut self) {
        self.tombstoned_at = Some(Utc::now());
    }
}

impl fmt::Display for DecisionLink {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "DecisionLink(decision={}, target={}:{}, relation={}, ordinal={})",
            self.decision_id, self.target_kind, self.target_id, self.relation_kind, self.ordinal
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_decision_new() {
        let decision = Decision::new(
            "d1".to_string(),
            "Test Decision".to_string(),
            "proposed".to_string(),
            "We will do X".to_string(),
            "Because Y".to_string(),
            None,
            None,
            "none".to_string(),
            None,
            None,
            None,
        );

        assert_eq!(decision.decision_id, "d1");
        assert_eq!(decision.title, "Test Decision");
        assert_eq!(decision.status, "proposed");
        assert!(!decision.evidence_hash.is_empty());
        assert!(!decision.is_tombstoned());
    }

    #[test]
    fn test_decision_tombstone() {
        let mut decision = Decision::new(
            "d1".to_string(),
            "Test".to_string(),
            "proposed".to_string(),
            "text".to_string(),
            "rationale".to_string(),
            None,
            None,
            "none".to_string(),
            None,
            None,
            None,
        );

        assert!(!decision.is_tombstoned());
        decision.tombstone();
        assert!(decision.is_tombstoned());
    }

    #[test]
    fn test_decision_update() {
        let mut decision = Decision::new(
            "d1".to_string(),
            "Test".to_string(),
            "proposed".to_string(),
            "text".to_string(),
            "rationale".to_string(),
            None,
            None,
            "none".to_string(),
            None,
            None,
            None,
        );

        let created_at = decision.created_at;
        std::thread::sleep(std::time::Duration::from_millis(10));

        decision.update(
            None,
            Some("accepted".to_string()),
            None,
            None,
            None,
            None,
            None,
            None,
            None,
        );

        assert_eq!(decision.status, "accepted");
        assert_eq!(decision.created_at, created_at);
        assert!(decision.updated_at > created_at);
    }

    #[test]
    fn test_evidence_item_new() {
        let item = DecisionEvidenceItem::new(
            "e1".to_string(),
            "mcp_chat_capture".to_string(),
            "Evidence content".to_string(),
        );

        assert_eq!(item.evidence_capture_id, "e1");
        assert_eq!(item.source, "mcp_chat_capture");
        assert!(!item.content_hash.is_empty());
    }

    #[test]
    fn test_decision_link_new() {
        let link = DecisionLink::new(
            "d1".to_string(),
            "ep".to_string(),
            "ep1".to_string(),
            "grounds".to_string(),
            0,
        );

        assert_eq!(link.decision_id, "d1");
        assert_eq!(link.target_kind, "ep");
        assert_eq!(link.target_id, "ep1");
        assert_eq!(link.relation_kind, "grounds");
        assert!(!link.is_tombstoned());
    }
}
