//! EttleX canonical error facility
//!
//! This crate provides the foundational error types used across the EttleX workspace:
//! - `ExErrorKind` — stable, matchable error taxonomy
//! - `ExError` — structured error type with rich context fields
//! - `assert_err_kind!` and `assert_err_field!` — test assertion macros

pub mod macros;

use ettlex_core_types::{RequestId, TraceId};

/// Canonical error kind taxonomy
///
/// This taxonomy provides a stable, structured classification of all errors
/// in the EttleX system. Each kind maps to a stable error code that can be
/// used for programmatic error handling, testing, and external API responses.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ExErrorKind {
    // Structural/Validation
    InvalidInput,
    InvalidTitle,
    InvalidOrdinal,
    NotFound,
    Deleted,
    ConstraintViolation,
    IllegalReparent,
    CycleDetected,
    MultipleParents,
    DuplicateMapping,
    MissingMapping,
    AmbiguousSelection,

    // Traversal/Export
    TraversalBroken,
    DeletedNodeInTraversal,
    AmbiguousLeafSelection,
    DeterminismViolation,

    // Mutation
    CannotDelete,
    StrandsChild,

    // Decision Errors
    InvalidDecision,
    InvalidEvidence,
    InvalidEvidencePath,
    DecisionTombstoned,
    DuplicateLink,
    InvalidTargetKind,

    // Profile Errors
    ProfileNotFound,
    ProfileDefaultMissing,
    /// A profile create was attempted with a ref that already exists with different content
    ProfileConflict,

    // Approval Errors
    ApprovalNotFound,
    ApprovalRoutingUnavailable,
    /// CAS blob referenced by an approval_requests row is missing
    ApprovalStorageCorrupt,

    // Constraint Errors
    InvalidConstraintFamily,
    AlreadyExists,
    ConstraintTombstoned,
    DuplicateAttachment,

    // Commit policy
    HeadMismatch,
    NotALeaf,
    PolicyDenied,
    RootEttleAmbiguous,
    RootEttleInvalid,
    EptAmbiguous,

    // Structural/Validation (extended)
    /// An EP has more than one parent EP in the refinement graph (integrity violation)
    RefinementIntegrityViolation,
    /// A valid query surface that is not yet implemented in this build
    NotImplemented,

    // Policy
    /// The referenced policy document does not exist in the policy provider
    PolicyNotFound,
    /// Policy export failed (malformed/unterminated HANDOFF markers, or unknown export_kind)
    PolicyExportFailed,
    /// A snapshot commit was attempted with an empty policy_ref
    PolicyRefMissing,
    /// Policy export result exceeded the configured maximum byte limit
    PolicyExportTooLarge,
    /// Policy file contains invalid UTF-8 or cannot be decoded
    PolicyParseError,
    /// A PolicyCreate was attempted with a policy_ref that already exists
    PolicyConflict,

    // Diff / manifest parsing
    /// Manifest bytes are not valid UTF-8 JSON, or `manifest_schema_version` is the wrong type
    InvalidManifest,
    /// A required manifest field (e.g. `semantic_manifest_digest`, `constraints`) is absent
    MissingField,
    /// A CAS digest referenced in the manifest was not found in the CAS store
    MissingBlob,
    /// The constraints envelope disagrees with its own recorded digest (non-fatal: diff still returns)
    InvariantViolation,

    /// Update command supplied no fields to update
    EmptyUpdate,

    // New variants (Slice 00)
    /// Entity was tombstoned and cannot be used
    AlreadyTombstoned,
    /// A decision link points to the same entity as its source
    SelfReferentialLink,
    /// Entity cannot be removed because it has active dependants
    HasActiveDependants,
    /// A link record is missing its type discriminator field
    MissingLinkType,

    // Integration/IO (future)
    Io,
    Serialization,
    Persistence,
    ExternalService,
    Timeout,
    Concurrency,

    // Auth (future)
    Unauthorised,
    Forbidden,

    // Internal
    Internal,
}

impl ExErrorKind {
    /// Get the stable error code for this kind
    pub fn code(&self) -> &'static str {
        match self {
            ExErrorKind::InvalidInput => "ERR_INVALID_INPUT",
            ExErrorKind::InvalidTitle => "ERR_INVALID_TITLE",
            ExErrorKind::InvalidOrdinal => "ERR_INVALID_ORDINAL",
            ExErrorKind::NotFound => "ERR_NOT_FOUND",
            ExErrorKind::Deleted => "ERR_DELETED",
            ExErrorKind::ConstraintViolation => "ERR_CONSTRAINT_VIOLATION",
            ExErrorKind::IllegalReparent => "ERR_ILLEGAL_REPARENT",
            ExErrorKind::CycleDetected => "ERR_CYCLE_DETECTED",
            ExErrorKind::MultipleParents => "ERR_MULTIPLE_PARENTS",
            ExErrorKind::DuplicateMapping => "ERR_DUPLICATE_MAPPING",
            ExErrorKind::MissingMapping => "ERR_MISSING_MAPPING",
            ExErrorKind::AmbiguousSelection => "ERR_AMBIGUOUS_SELECTION",
            ExErrorKind::TraversalBroken => "ERR_TRAVERSAL_BROKEN",
            ExErrorKind::DeletedNodeInTraversal => "ERR_DELETED_NODE_IN_TRAVERSAL",
            ExErrorKind::AmbiguousLeafSelection => "ERR_AMBIGUOUS_LEAF_SELECTION",
            ExErrorKind::DeterminismViolation => "ERR_DETERMINISM_VIOLATION",
            ExErrorKind::CannotDelete => "ERR_CANNOT_DELETE",
            ExErrorKind::StrandsChild => "ERR_STRANDS_CHILD",
            ExErrorKind::InvalidDecision => "ERR_INVALID_DECISION",
            ExErrorKind::InvalidEvidence => "ERR_INVALID_EVIDENCE",
            ExErrorKind::InvalidEvidencePath => "ERR_INVALID_EVIDENCE_PATH",
            ExErrorKind::DecisionTombstoned => "ERR_DECISION_TOMBSTONED",
            ExErrorKind::DuplicateLink => "ERR_DUPLICATE_LINK",
            ExErrorKind::InvalidTargetKind => "ERR_INVALID_TARGET_KIND",
            ExErrorKind::ProfileNotFound => "ERR_PROFILE_NOT_FOUND",
            ExErrorKind::ProfileDefaultMissing => "ERR_PROFILE_DEFAULT_MISSING",
            ExErrorKind::ProfileConflict => "ERR_PROFILE_CONFLICT",
            ExErrorKind::ApprovalNotFound => "ERR_APPROVAL_NOT_FOUND",
            ExErrorKind::ApprovalRoutingUnavailable => "ERR_APPROVAL_ROUTING_UNAVAILABLE",
            ExErrorKind::ApprovalStorageCorrupt => "ERR_APPROVAL_STORAGE_CORRUPT",
            ExErrorKind::RefinementIntegrityViolation => "ERR_REFINEMENT_INTEGRITY_VIOLATION",
            ExErrorKind::NotImplemented => "ERR_NOT_IMPLEMENTED",
            ExErrorKind::InvalidConstraintFamily => "ERR_INVALID_CONSTRAINT_FAMILY",
            ExErrorKind::AlreadyExists => "ERR_ALREADY_EXISTS",
            ExErrorKind::ConstraintTombstoned => "ERR_CONSTRAINT_TOMBSTONED",
            ExErrorKind::DuplicateAttachment => "ERR_DUPLICATE_ATTACHMENT",
            ExErrorKind::HeadMismatch => "ERR_HEAD_MISMATCH",
            ExErrorKind::NotALeaf => "ERR_NOT_A_LEAF",
            ExErrorKind::PolicyDenied => "ERR_POLICY_DENIED",
            ExErrorKind::RootEttleAmbiguous => "ERR_ROOT_ETTLE_AMBIGUOUS",
            ExErrorKind::RootEttleInvalid => "ERR_ROOT_ETTLE_INVALID",
            ExErrorKind::EptAmbiguous => "ERR_EPT_AMBIGUOUS",
            ExErrorKind::PolicyNotFound => "ERR_POLICY_NOT_FOUND",
            ExErrorKind::PolicyExportFailed => "ERR_POLICY_EXPORT_FAILED",
            ExErrorKind::PolicyRefMissing => "ERR_POLICY_REF_MISSING",
            ExErrorKind::PolicyExportTooLarge => "ERR_POLICY_EXPORT_TOO_LARGE",
            ExErrorKind::PolicyParseError => "ERR_POLICY_PARSE_ERROR",
            ExErrorKind::PolicyConflict => "ERR_POLICY_CONFLICT",
            ExErrorKind::InvalidManifest => "ERR_INVALID_MANIFEST",
            ExErrorKind::MissingField => "ERR_MISSING_FIELD",
            ExErrorKind::MissingBlob => "ERR_MISSING_BLOB",
            ExErrorKind::InvariantViolation => "ERR_INVARIANT_VIOLATION",
            ExErrorKind::EmptyUpdate => "ERR_EMPTY_UPDATE",
            ExErrorKind::AlreadyTombstoned => "ERR_ALREADY_TOMBSTONED",
            ExErrorKind::SelfReferentialLink => "ERR_SELF_REFERENTIAL_LINK",
            ExErrorKind::HasActiveDependants => "ERR_HAS_ACTIVE_DEPENDANTS",
            ExErrorKind::MissingLinkType => "ERR_MISSING_LINK_TYPE",
            ExErrorKind::Io => "ERR_IO",
            ExErrorKind::Serialization => "ERR_SERIALIZATION",
            ExErrorKind::Persistence => "ERR_PERSISTENCE",
            ExErrorKind::ExternalService => "ERR_EXTERNAL_SERVICE",
            ExErrorKind::Timeout => "ERR_TIMEOUT",
            ExErrorKind::Concurrency => "ERR_CONCURRENCY",
            ExErrorKind::Unauthorised => "ERR_UNAUTHORISED",
            ExErrorKind::Forbidden => "ERR_FORBIDDEN",
            ExErrorKind::Internal => "ERR_INTERNAL",
        }
    }
}

/// Canonical structured error type
///
/// This error type provides a structured representation of errors with
/// classification fields for programmatic handling and rich context for debugging.
#[derive(Debug, Clone)]
pub struct ExError {
    kind: ExErrorKind,
    op: Option<String>,
    entity_id: Option<String>,
    ep_id: Option<String>,
    ordinal: Option<u32>,
    request_id: Option<RequestId>,
    trace_id: Option<TraceId>,
    message: String,
    source: Option<Box<ExError>>,
    candidates: Option<Vec<String>>,
}

impl ExError {
    /// Create a new error with the specified kind
    pub fn new(kind: ExErrorKind) -> Self {
        Self {
            kind,
            op: None,
            entity_id: None,
            ep_id: None,
            ordinal: None,
            request_id: None,
            trace_id: None,
            message: String::new(),
            source: None,
            candidates: None,
        }
    }

    /// Add operation context
    pub fn with_op(mut self, op: impl Into<String>) -> Self {
        self.op = Some(op.into());
        self
    }

    /// Add entity ID context
    pub fn with_entity_id(mut self, id: impl Into<String>) -> Self {
        self.entity_id = Some(id.into());
        self
    }

    /// Add EP ID context
    pub fn with_ep_id(mut self, id: impl Into<String>) -> Self {
        self.ep_id = Some(id.into());
        self
    }

    /// Add ordinal context
    pub fn with_ordinal(mut self, ordinal: u32) -> Self {
        self.ordinal = Some(ordinal);
        self
    }

    /// Add request ID context
    pub fn with_request_id(mut self, request_id: RequestId) -> Self {
        self.request_id = Some(request_id);
        self
    }

    /// Add trace ID context
    pub fn with_trace_id(mut self, trace_id: TraceId) -> Self {
        self.trace_id = Some(trace_id);
        self
    }

    /// Add custom message
    pub fn with_message(mut self, message: impl Into<String>) -> Self {
        self.message = message.into();
        self
    }

    /// Add source error
    pub fn with_source(mut self, source: ExError) -> Self {
        self.source = Some(Box::new(source));
        self
    }

    /// Add candidate entity ids (used for RootEttleAmbiguous to carry candidate leaf EP ids)
    pub fn with_candidates(mut self, ids: Vec<String>) -> Self {
        self.candidates = Some(ids);
        self
    }

    /// Get the error kind
    pub fn kind(&self) -> ExErrorKind {
        self.kind
    }

    /// Get the stable error code
    pub fn code(&self) -> &'static str {
        self.kind.code()
    }

    /// Get the operation context, if any
    pub fn op(&self) -> Option<&str> {
        self.op.as_deref()
    }

    /// Get the entity ID context, if any
    pub fn entity_id(&self) -> Option<&str> {
        self.entity_id.as_deref()
    }

    /// Get the EP ID context, if any
    pub fn ep_id(&self) -> Option<&str> {
        self.ep_id.as_deref()
    }

    /// Get the ordinal context, if any
    pub fn ordinal(&self) -> Option<u32> {
        self.ordinal
    }

    /// Get the request ID context, if any
    pub fn request_id(&self) -> Option<&RequestId> {
        self.request_id.as_ref()
    }

    /// Get the trace ID context, if any
    pub fn trace_id(&self) -> Option<&TraceId> {
        self.trace_id.as_ref()
    }

    /// Get the error message
    pub fn message(&self) -> &str {
        &self.message
    }

    /// Get the source error, if any
    pub fn source_error(&self) -> Option<&ExError> {
        self.source.as_deref()
    }

    /// Get candidate entity ids, if any (populated on RootEttleAmbiguous)
    pub fn candidates(&self) -> Option<&[String]> {
        self.candidates.as_deref()
    }
}

impl std::fmt::Display for ExError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "[{}] {}", self.code(), self.kind.code())?;
        if let Some(op) = &self.op {
            write!(f, " in operation '{}'", op)?;
        }
        if !self.message.is_empty() {
            write!(f, ": {}", self.message)?;
        }
        if let Some(entity_id) = &self.entity_id {
            write!(f, " (entity_id: {})", entity_id)?;
        }
        if let Some(ep_id) = &self.ep_id {
            write!(f, " (ep_id: {})", ep_id)?;
        }
        if let Some(ordinal) = self.ordinal {
            write!(f, " (ordinal: {})", ordinal)?;
        }
        Ok(())
    }
}

impl std::error::Error for ExError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        None
    }
}

/// Result type alias
pub type Result<T> = std::result::Result<T, ExError>;

/// Conversion from serde_json::Error to ExError
impl From<serde_json::Error> for ExError {
    fn from(err: serde_json::Error) -> Self {
        ExError::new(ExErrorKind::Serialization).with_message(err.to_string())
    }
}

/// Conversion from std::io::Error to ExError
impl From<std::io::Error> for ExError {
    fn from(err: std::io::Error) -> Self {
        ExError::new(ExErrorKind::Io).with_message(err.to_string())
    }
}
