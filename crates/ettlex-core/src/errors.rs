use ettlex_core_types::{RequestId, TraceId};
use thiserror::Error;

/// Result type alias using EttleXError
pub type Result<T> = std::result::Result<T, EttleXError>;

// ========== Error Facility ==========

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

// ========== End Error Facility ==========

/// Comprehensive error taxonomy for EttleX operations
#[derive(Error, Debug, Clone, PartialEq)]
pub enum EttleXError {
    // ===== Structural Errors =====
    /// Parent ettle was not found
    #[error("Parent ettle not found: {ettle_id}")]
    ParentNotFound { ettle_id: String },

    /// Ettle has multiple parents (integrity violation)
    #[error("Ettle has multiple parents: {ettle_id}")]
    MultipleParents { ettle_id: String },

    /// Cycle detected in refinement tree
    #[error("Cycle detected: setting parent would create a cycle involving ettle {ettle_id}")]
    CycleDetected { ettle_id: String },

    /// Ettle not found in store
    #[error("Ettle not found: {ettle_id}")]
    EttleNotFound { ettle_id: String },

    /// EP not found in store
    #[error("EP not found: {ep_id}")]
    EpNotFound { ep_id: String },

    /// Ettle was previously deleted (tombstoned)
    #[error("Ettle was deleted: {ettle_id}")]
    EttleDeleted { ettle_id: String },

    /// EP was previously deleted (tombstoned)
    #[error("EP was deleted: {ep_id}")]
    EpDeleted { ep_id: String },

    // ===== Validation Errors =====
    /// Invalid title (empty or whitespace-only)
    #[error("Invalid title: {reason}")]
    InvalidTitle { reason: String },

    /// Child ettle exists but has no EP mapping from parent
    #[error("Child ettle {child_id} has no EP mapping from parent {parent_id}")]
    ChildWithoutEpMapping { child_id: String, parent_id: String },

    /// Duplicate EP ordinal within same ettle
    #[error("Duplicate EP ordinal {ordinal} in ettle {ettle_id}")]
    DuplicateEpOrdinal { ettle_id: String, ordinal: u32 },

    /// Child ettle is referenced by multiple EPs (should be one-to-one)
    #[error("Child ettle {child_id} is referenced by multiple EPs: {ep_ids:?}")]
    ChildReferencedByMultipleEps {
        child_id: String,
        ep_ids: Vec<String>,
    },

    /// EP references a child that doesn't exist
    #[error("EP {ep_id} references non-existent child: {child_id}")]
    EpReferencesNonExistentChild { ep_id: String, child_id: String },

    /// Orphaned ettle (has parent_id but parent doesn't exist)
    #[error("Orphaned ettle {ettle_id}: parent {parent_id} does not exist")]
    OrphanedEttle { ettle_id: String, parent_id: String },

    /// EP ordinal cannot be changed after creation
    #[error("Cannot change EP ordinal: ordinals are immutable")]
    OrdinalImmutable,

    /// Bidirectional membership inconsistency: EP's ettle_id doesn't match owning Ettle
    #[error("EP {ep_id} has ettle_id={ep_ettle_id} but is owned by ettle {owner_ettle_id}")]
    MembershipInconsistent {
        ep_id: String,
        ep_ettle_id: String,
        owner_ettle_id: String,
    },

    /// EP orphan: EP.ettle_id points to Ettle but Ettle.ep_ids doesn't include EP
    #[error("EP {ep_id} points to ettle {ettle_id} but is not listed in its ep_ids")]
    EpOrphaned { ep_id: String, ettle_id: String },

    /// Active EP ordering is non-deterministic (should never happen)
    #[error("Active EP ordering is non-deterministic for ettle {ettle_id}")]
    ActiveEpOrderNonDeterministic { ettle_id: String },

    /// EP list contains unknown EP ID
    #[error("Ettle {ettle_id} ep_ids contains unknown EP ID: {ep_id}")]
    EpListContainsUnknownId { ettle_id: String, ep_id: String },

    /// EP ownership points to unknown Ettle
    #[error("EP {ep_id} has ettle_id pointing to unknown ettle: {ettle_id}")]
    EpOwnershipPointsToUnknownEttle { ep_id: String, ettle_id: String },

    /// Invalid parent pointer (structural integrity violation)
    #[error("Invalid parent pointer in ettle {ettle_id}: {reason}")]
    InvalidParentPointer { ettle_id: String, reason: String },

    /// Invalid WHAT content (empty string not allowed)
    #[error("Invalid WHAT content in EP {ep_id}: cannot be empty string")]
    InvalidWhat { ep_id: String },

    /// Invalid HOW content (empty string not allowed)
    #[error("Invalid HOW content in EP {ep_id}: cannot be empty string")]
    InvalidHow { ep_id: String },

    /// EP references deleted EP in child mapping
    #[error("EP {ep_id} has child mapping but EP is deleted (tombstoned)")]
    MappingReferencesDeletedEp { ep_id: String },

    /// EP references deleted child Ettle
    #[error("EP {ep_id} maps to deleted child ettle {child_id}")]
    MappingReferencesDeletedChild { ep_id: String, child_id: String },

    // ===== Traversal Errors =====
    /// RT parent chain is broken (parent_id points to non-existent ettle)
    #[error("RT computation failed: parent chain broken at ettle {ettle_id}")]
    RtParentChainBroken { ettle_id: String },

    /// EPT computation failed: missing EP mapping in parent
    #[error("EPT computation failed: no EP in parent {parent_id} maps to child {child_id}")]
    EptMissingMapping { parent_id: String, child_id: String },

    /// EPT computation failed: multiple EPs map to same child
    #[error("EPT computation failed: multiple EPs in parent {parent_id} map to child {child_id}")]
    EptDuplicateMapping { parent_id: String, child_id: String },

    /// EPT computation failed: leaf ettle has multiple EPs and no ordinal specified
    #[error("EPT computation failed: leaf ettle {leaf_id} has multiple EPs, must specify ordinal")]
    EptAmbiguousLeafEp { leaf_id: String },

    /// EPT computation failed: specified leaf EP not found
    #[error("EPT computation failed: leaf EP with ordinal {ordinal} not found in ettle {leaf_id}")]
    EptLeafEpNotFound { leaf_id: String, ordinal: u32 },

    // ===== Mutation Errors =====
    /// Cannot delete ettle that has children
    #[error("Cannot delete ettle {ettle_id}: has {child_count} children")]
    DeleteWithChildren {
        ettle_id: String,
        child_count: usize,
    },

    /// Cannot delete EP that is referenced by a child
    #[error("Cannot delete EP {ep_id}: child ettle {child_id} still references it")]
    DeleteReferencedEp { ep_id: String, child_id: String },

    /// Illegal reparent operation
    #[error("Illegal reparent: {reason}")]
    IllegalReparent { reason: String },

    /// Cannot link child: child already has a different parent
    #[error(
        "Cannot link child {child_id} to EP {ep_id}: child already has parent {current_parent_id}"
    )]
    ChildAlreadyHasParent {
        child_id: String,
        ep_id: String,
        current_parent_id: String,
    },

    /// Cannot link child: EP already has a different child
    #[error("Cannot link child to EP {ep_id}: EP already maps to {current_child_id}")]
    EpAlreadyHasChild {
        ep_id: String,
        current_child_id: String,
    },

    /// Cannot create EP with ordinal that already exists
    #[error("Cannot create EP with ordinal {ordinal} in ettle {ettle_id}: ordinal already exists")]
    OrdinalAlreadyExists { ettle_id: String, ordinal: u32 },

    /// Cannot reuse ordinal of a tombstoned EP
    #[error("Cannot create EP with ordinal {ordinal} in ettle {ettle_id}: ordinal is used by tombstoned EP {tombstoned_ep_id}")]
    EpOrdinalReuseForbidden {
        ettle_id: String,
        ordinal: u32,
        tombstoned_ep_id: String,
    },

    /// Cannot delete EP0 (ordinal 0)
    #[error("Cannot delete EP0 (ordinal 0) in ettle {ettle_id}")]
    CannotDeleteEp0 { ettle_id: String },

    /// Tombstoning EP would strand its child (last mapping)
    #[error("Cannot delete EP {ep_id}: it's the only active mapping to child {child_id}")]
    TombstoneStrandsChild { ep_id: String, child_id: String },

    // ===== Apply/Command Errors =====
    /// Apply function atomicity breach (internal assertion failure)
    #[error("Apply atomicity breach: {message}")]
    ApplyAtomicityBreach { message: String },

    /// Attempted to hard delete an anchored EP
    #[error("Cannot hard delete anchored EP {ep_id}")]
    HardDeleteForbiddenAnchoredEp { ep_id: String },

    /// Hard delete hit inconsistent membership (EP not in owning Ettle's ep_ids)
    #[error("Hard delete failed: EP {ep_id} not found in owning Ettle {ettle_id} ep_ids list")]
    DeleteReferencesMissingEpInOwningEttle { ep_id: String, ettle_id: String },

    // ===== Generic Errors =====
    /// Generic internal error
    #[error("Internal error: {message}")]
    Internal { message: String },
}

/// Conversion from EttleXError to ExError
///
/// This allows existing code using EttleXError to be gradually migrated
/// to the canonical error facility while maintaining backward compatibility.
impl From<EttleXError> for ExError {
    fn from(err: EttleXError) -> Self {
        match err {
            // Structural Errors -> NotFound
            EttleXError::ParentNotFound { ettle_id } => ExError::new(ExErrorKind::NotFound)
                .with_entity_id(ettle_id)
                .with_op("find_parent")
                .with_message("Parent ettle not found"),

            EttleXError::EttleNotFound { ettle_id } => ExError::new(ExErrorKind::NotFound)
                .with_entity_id(ettle_id)
                .with_message("Ettle not found"),

            EttleXError::EpNotFound { ep_id } => ExError::new(ExErrorKind::NotFound)
                .with_ep_id(ep_id)
                .with_message("EP not found"),

            EttleXError::OrphanedEttle {
                ettle_id,
                parent_id,
            } => ExError::new(ExErrorKind::NotFound)
                .with_entity_id(ettle_id)
                .with_message(format!("Parent {} does not exist", parent_id)),

            // Structural Errors -> Deleted
            EttleXError::EttleDeleted { ettle_id } => ExError::new(ExErrorKind::Deleted)
                .with_entity_id(ettle_id)
                .with_message("Ettle was deleted"),

            EttleXError::EpDeleted { ep_id } => ExError::new(ExErrorKind::Deleted)
                .with_ep_id(ep_id)
                .with_message("EP was deleted"),

            EttleXError::MappingReferencesDeletedEp { ep_id } => ExError::new(ExErrorKind::Deleted)
                .with_ep_id(ep_id)
                .with_message("EP has child mapping but EP is deleted"),

            EttleXError::MappingReferencesDeletedChild { ep_id, child_id } => {
                ExError::new(ExErrorKind::Deleted)
                    .with_ep_id(ep_id)
                    .with_message(format!("EP maps to deleted child ettle {}", child_id))
            }

            // Validation Errors -> InvalidTitle
            EttleXError::InvalidTitle { reason } => ExError::new(ExErrorKind::InvalidTitle)
                .with_message(format!("Invalid title: {}", reason)),

            // Validation Errors -> InvalidInput
            EttleXError::InvalidWhat { ep_id } => ExError::new(ExErrorKind::InvalidInput)
                .with_ep_id(ep_id)
                .with_message("Invalid WHAT content: cannot be empty string"),

            EttleXError::InvalidHow { ep_id } => ExError::new(ExErrorKind::InvalidInput)
                .with_ep_id(ep_id)
                .with_message("Invalid HOW content: cannot be empty string"),

            // Validation Errors -> InvalidOrdinal
            EttleXError::OrdinalAlreadyExists { ettle_id, ordinal } => {
                ExError::new(ExErrorKind::InvalidOrdinal)
                    .with_entity_id(ettle_id)
                    .with_ordinal(ordinal)
                    .with_message("Ordinal already exists")
            }

            EttleXError::EpOrdinalReuseForbidden {
                ettle_id,
                ordinal,
                tombstoned_ep_id,
            } => ExError::new(ExErrorKind::InvalidOrdinal)
                .with_entity_id(ettle_id)
                .with_ordinal(ordinal)
                .with_message(format!(
                    "Ordinal is used by tombstoned EP {}",
                    tombstoned_ep_id
                )),

            EttleXError::OrdinalImmutable => ExError::new(ExErrorKind::InvalidOrdinal)
                .with_message("Cannot change EP ordinal: ordinals are immutable"),

            // Structural Errors -> MultipleParents
            EttleXError::MultipleParents { ettle_id } => ExError::new(ExErrorKind::MultipleParents)
                .with_entity_id(ettle_id)
                .with_message("Ettle has multiple parents"),

            // Structural Errors -> CycleDetected
            EttleXError::CycleDetected { ettle_id } => ExError::new(ExErrorKind::CycleDetected)
                .with_entity_id(ettle_id)
                .with_message("Setting parent would create a cycle"),

            // Constraint Violations
            EttleXError::ChildWithoutEpMapping {
                child_id,
                parent_id,
            } => ExError::new(ExErrorKind::ConstraintViolation)
                .with_entity_id(child_id)
                .with_message(format!("Child has no EP mapping from parent {}", parent_id)),

            EttleXError::DuplicateEpOrdinal { ettle_id, ordinal } => {
                ExError::new(ExErrorKind::ConstraintViolation)
                    .with_entity_id(ettle_id)
                    .with_ordinal(ordinal)
                    .with_message("Duplicate EP ordinal")
            }

            EttleXError::ChildReferencedByMultipleEps { child_id, ep_ids } => {
                ExError::new(ExErrorKind::ConstraintViolation)
                    .with_entity_id(child_id)
                    .with_message(format!("Referenced by multiple EPs: {:?}", ep_ids))
            }

            EttleXError::EpReferencesNonExistentChild { ep_id, child_id } => {
                ExError::new(ExErrorKind::ConstraintViolation)
                    .with_ep_id(ep_id)
                    .with_message(format!("References non-existent child: {}", child_id))
            }

            EttleXError::MembershipInconsistent {
                ep_id,
                ep_ettle_id,
                owner_ettle_id,
            } => ExError::new(ExErrorKind::ConstraintViolation)
                .with_ep_id(ep_id)
                .with_message(format!(
                    "EP has ettle_id={} but is owned by ettle {}",
                    ep_ettle_id, owner_ettle_id
                )),

            EttleXError::EpOrphaned { ep_id, ettle_id } => {
                ExError::new(ExErrorKind::ConstraintViolation)
                    .with_ep_id(ep_id)
                    .with_message(format!(
                        "EP points to ettle {} but is not listed in its ep_ids",
                        ettle_id
                    ))
            }

            EttleXError::EpListContainsUnknownId { ettle_id, ep_id } => {
                ExError::new(ExErrorKind::ConstraintViolation)
                    .with_entity_id(ettle_id)
                    .with_message(format!("ep_ids contains unknown EP ID: {}", ep_id))
            }

            EttleXError::EpOwnershipPointsToUnknownEttle { ep_id, ettle_id } => {
                ExError::new(ExErrorKind::ConstraintViolation)
                    .with_ep_id(ep_id)
                    .with_message(format!(
                        "EP has ettle_id pointing to unknown ettle: {}",
                        ettle_id
                    ))
            }

            EttleXError::InvalidParentPointer { ettle_id, reason } => {
                ExError::new(ExErrorKind::ConstraintViolation)
                    .with_entity_id(ettle_id)
                    .with_message(format!("Invalid parent pointer: {}", reason))
            }

            // Determinism Violation
            EttleXError::ActiveEpOrderNonDeterministic { ettle_id } => {
                ExError::new(ExErrorKind::DeterminismViolation)
                    .with_entity_id(ettle_id)
                    .with_message("Active EP ordering is non-deterministic")
            }

            // Traversal Errors
            EttleXError::RtParentChainBroken { ettle_id } => {
                ExError::new(ExErrorKind::TraversalBroken)
                    .with_entity_id(ettle_id)
                    .with_message("RT computation failed: parent chain broken")
            }

            EttleXError::EptMissingMapping {
                parent_id,
                child_id,
            } => ExError::new(ExErrorKind::MissingMapping)
                .with_entity_id(child_id)
                .with_message(format!("No EP in parent {} maps to child", parent_id)),

            EttleXError::EptDuplicateMapping {
                parent_id,
                child_id,
            } => ExError::new(ExErrorKind::DuplicateMapping)
                .with_entity_id(child_id)
                .with_message(format!("Multiple EPs in parent {} map to child", parent_id)),

            EttleXError::EptAmbiguousLeafEp { leaf_id } => {
                ExError::new(ExErrorKind::AmbiguousLeafSelection)
                    .with_entity_id(leaf_id)
                    .with_message("Leaf ettle has multiple EPs, must specify ordinal")
            }

            EttleXError::EptLeafEpNotFound { leaf_id, ordinal } => {
                ExError::new(ExErrorKind::NotFound)
                    .with_entity_id(leaf_id)
                    .with_ordinal(ordinal)
                    .with_message("Leaf EP with ordinal not found")
            }

            // Mutation Errors -> CannotDelete
            EttleXError::DeleteWithChildren {
                ettle_id,
                child_count,
            } => ExError::new(ExErrorKind::CannotDelete)
                .with_entity_id(ettle_id)
                .with_message(format!("Has {} children", child_count)),

            EttleXError::DeleteReferencedEp { ep_id, child_id } => {
                ExError::new(ExErrorKind::CannotDelete)
                    .with_ep_id(ep_id)
                    .with_message(format!("Child ettle {} still references it", child_id))
            }

            EttleXError::CannotDeleteEp0 { ettle_id } => ExError::new(ExErrorKind::CannotDelete)
                .with_entity_id(ettle_id)
                .with_ep_id("ep0")
                .with_message("Cannot delete EP0 (ordinal 0)"),

            EttleXError::HardDeleteForbiddenAnchoredEp { ep_id } => {
                ExError::new(ExErrorKind::CannotDelete)
                    .with_ep_id(ep_id)
                    .with_message("Cannot hard delete anchored EP")
            }

            EttleXError::DeleteReferencesMissingEpInOwningEttle { ep_id, ettle_id } => {
                ExError::new(ExErrorKind::CannotDelete)
                    .with_ep_id(ep_id)
                    .with_entity_id(ettle_id)
                    .with_message("EP not found in owning Ettle's ep_ids list")
            }

            // Mutation Errors -> StrandsChild
            EttleXError::TombstoneStrandsChild { ep_id, child_id } => {
                ExError::new(ExErrorKind::StrandsChild)
                    .with_ep_id(ep_id)
                    .with_message(format!(
                        "It's the only active mapping to child {}",
                        child_id
                    ))
            }

            // Mutation Errors -> IllegalReparent
            EttleXError::IllegalReparent { reason } => ExError::new(ExErrorKind::IllegalReparent)
                .with_message(format!("Illegal reparent: {}", reason)),

            EttleXError::ChildAlreadyHasParent {
                child_id,
                ep_id,
                current_parent_id,
            } => ExError::new(ExErrorKind::IllegalReparent)
                .with_entity_id(child_id)
                .with_ep_id(ep_id)
                .with_message(format!("Child already has parent {}", current_parent_id)),

            // Duplicate Mapping
            EttleXError::EpAlreadyHasChild {
                ep_id,
                current_child_id,
            } => ExError::new(ExErrorKind::DuplicateMapping)
                .with_ep_id(ep_id)
                .with_message(format!("EP already maps to {}", current_child_id)),

            // Internal Errors
            EttleXError::Internal { message } => {
                ExError::new(ExErrorKind::Internal).with_message(message)
            }

            EttleXError::ApplyAtomicityBreach { message } => ExError::new(ExErrorKind::Internal)
                .with_op("apply")
                .with_message(format!("Apply atomicity breach: {}", message)),
        }
    }
}
