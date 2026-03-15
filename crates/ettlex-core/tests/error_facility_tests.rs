use ettlex_core::errors::{ExError, ExErrorKind};

#[test]
fn test_not_found_verifiable_by_kind() {
    let ex_err = ExError::new(ExErrorKind::NotFound)
        .with_entity_id("unknown")
        .with_message("Ettle not found");

    assert_eq!(ex_err.kind(), ExErrorKind::NotFound);
    assert_eq!(ex_err.code(), "ERR_NOT_FOUND");
    assert_eq!(ex_err.entity_id(), Some("unknown"));
}

#[test]
fn test_deleted_distinct_from_not_found() {
    let ex_err = ExError::new(ExErrorKind::Deleted)
        .with_entity_id("deleted-ettle")
        .with_message("Ettle was deleted");

    assert_eq!(ex_err.kind(), ExErrorKind::Deleted);
    assert_eq!(ex_err.code(), "ERR_DELETED");
    assert_ne!(ex_err.kind(), ExErrorKind::NotFound);
    assert_eq!(ex_err.entity_id(), Some("deleted-ettle"));
}

#[test]
fn test_invalid_title_structured_fields() {
    let ex_err = ExError::new(ExErrorKind::InvalidTitle)
        .with_message("Invalid title: Title cannot be empty");

    assert_eq!(ex_err.kind(), ExErrorKind::InvalidTitle);
    assert_eq!(ex_err.code(), "ERR_INVALID_TITLE");
    assert!(ex_err.message().contains("Invalid title"));
}

#[test]
fn test_error_kind_code_mapping() {
    // Test that each kind has a stable, unique code
    let kinds = vec![
        (ExErrorKind::NotFound, "ERR_NOT_FOUND"),
        (ExErrorKind::Deleted, "ERR_DELETED"),
        (ExErrorKind::InvalidTitle, "ERR_INVALID_TITLE"),
        (ExErrorKind::InvalidOrdinal, "ERR_INVALID_ORDINAL"),
        (ExErrorKind::CycleDetected, "ERR_CYCLE_DETECTED"),
        (ExErrorKind::CannotDelete, "ERR_CANNOT_DELETE"),
        (ExErrorKind::StrandsChild, "ERR_STRANDS_CHILD"),
    ];

    for (kind, expected_code) in kinds {
        assert_eq!(kind.code(), expected_code);
    }
}

#[test]
fn test_ep_not_found_conversion() {
    let ex_err = ExError::new(ExErrorKind::NotFound)
        .with_ep_id("ep123")
        .with_message("EP not found");

    assert_eq!(ex_err.kind(), ExErrorKind::NotFound);
    assert_eq!(ex_err.code(), "ERR_NOT_FOUND");
    assert_eq!(ex_err.ep_id(), Some("ep123"));
}

#[test]
fn test_cycle_detected_conversion() {
    let ex_err = ExError::new(ExErrorKind::CycleDetected)
        .with_entity_id("e1")
        .with_message("Cycle detected");

    assert_eq!(ex_err.kind(), ExErrorKind::CycleDetected);
    assert_eq!(ex_err.code(), "ERR_CYCLE_DETECTED");
    assert_eq!(ex_err.entity_id(), Some("e1"));
}

#[test]
fn test_ordinal_already_exists_conversion() {
    let ex_err = ExError::new(ExErrorKind::InvalidOrdinal)
        .with_entity_id("e1")
        .with_ordinal(5)
        .with_message("Ordinal already exists");

    assert_eq!(ex_err.kind(), ExErrorKind::InvalidOrdinal);
    assert_eq!(ex_err.code(), "ERR_INVALID_ORDINAL");
    assert_eq!(ex_err.entity_id(), Some("e1"));
    assert_eq!(ex_err.ordinal(), Some(5));
}

#[test]
fn test_cannot_delete_with_children_conversion() {
    let ex_err = ExError::new(ExErrorKind::CannotDelete)
        .with_entity_id("e1")
        .with_message("Cannot delete: has 3 children");

    assert_eq!(ex_err.kind(), ExErrorKind::CannotDelete);
    assert_eq!(ex_err.code(), "ERR_CANNOT_DELETE");
    assert_eq!(ex_err.entity_id(), Some("e1"));
    assert!(ex_err.message().contains("3 children"));
}

#[test]
fn test_strands_child_conversion() {
    let ex_err = ExError::new(ExErrorKind::StrandsChild)
        .with_ep_id("ep1")
        .with_message("Tombstoning ep1 would strand child child1");

    assert_eq!(ex_err.kind(), ExErrorKind::StrandsChild);
    assert_eq!(ex_err.code(), "ERR_STRANDS_CHILD");
    assert_eq!(ex_err.ep_id(), Some("ep1"));
    assert!(ex_err.message().contains("child1"));
}

#[test]
fn test_multiple_parents_conversion() {
    let ex_err = ExError::new(ExErrorKind::MultipleParents)
        .with_entity_id("e1")
        .with_message("Multiple parents detected");

    assert_eq!(ex_err.kind(), ExErrorKind::MultipleParents);
    assert_eq!(ex_err.code(), "ERR_MULTIPLE_PARENTS");
}

#[test]
fn test_traversal_broken_conversion() {
    let ex_err = ExError::new(ExErrorKind::TraversalBroken)
        .with_entity_id("e1")
        .with_message("Parent chain broken");

    assert_eq!(ex_err.kind(), ExErrorKind::TraversalBroken);
    assert_eq!(ex_err.code(), "ERR_TRAVERSAL_BROKEN");
}

#[test]
fn test_ambiguous_leaf_selection_conversion() {
    let ex_err = ExError::new(ExErrorKind::AmbiguousLeafSelection)
        .with_entity_id("leaf1")
        .with_message("Leaf has multiple EPs");

    assert_eq!(ex_err.kind(), ExErrorKind::AmbiguousLeafSelection);
    assert_eq!(ex_err.code(), "ERR_AMBIGUOUS_LEAF_SELECTION");
}

#[test]
fn test_determinism_violation_conversion() {
    let ex_err = ExError::new(ExErrorKind::DeterminismViolation)
        .with_entity_id("e1")
        .with_message("Active EP order is non-deterministic");

    assert_eq!(ex_err.kind(), ExErrorKind::DeterminismViolation);
    assert_eq!(ex_err.code(), "ERR_DETERMINISM_VIOLATION");
}

#[test]
fn test_ex_error_builder_pattern() {
    use ettlex_core_types::RequestId;

    let request_id = RequestId::new();
    let ex_err = ExError::new(ExErrorKind::NotFound)
        .with_op("read_ettle")
        .with_entity_id("e123")
        .with_message("Ettle not found in store")
        .with_request_id(request_id.clone());

    assert_eq!(ex_err.kind(), ExErrorKind::NotFound);
    assert_eq!(ex_err.op(), Some("read_ettle"));
    assert_eq!(ex_err.entity_id(), Some("e123"));
    assert!(ex_err.message().contains("not found"));
    assert!(ex_err.request_id().is_some());
}

#[test]
fn test_ex_error_display() {
    let ex_err = ExError::new(ExErrorKind::NotFound)
        .with_op("read_ettle")
        .with_entity_id("e123")
        .with_message("Ettle not found");

    let display_str = format!("{}", ex_err);

    assert!(display_str.contains("ERR_NOT_FOUND"));
    assert!(display_str.contains("read_ettle"));
    assert!(display_str.contains("e123"));
}

#[test]
fn test_constraint_violation_conversion() {
    let ex_err = ExError::new(ExErrorKind::ConstraintViolation)
        .with_entity_id("e1")
        .with_ordinal(2)
        .with_message("Duplicate EP ordinal");

    assert_eq!(ex_err.kind(), ExErrorKind::ConstraintViolation);
    assert_eq!(ex_err.code(), "ERR_CONSTRAINT_VIOLATION");
}

#[test]
fn test_illegal_reparent_conversion() {
    let ex_err = ExError::new(ExErrorKind::IllegalReparent).with_message("Would create orphan");

    assert_eq!(ex_err.kind(), ExErrorKind::IllegalReparent);
    assert_eq!(ex_err.code(), "ERR_ILLEGAL_REPARENT");
}

#[test]
fn test_internal_error_conversion() {
    let ex_err = ExError::new(ExErrorKind::Internal).with_message("Unexpected state");

    assert_eq!(ex_err.kind(), ExErrorKind::Internal);
    assert_eq!(ex_err.code(), "ERR_INTERNAL");
}

#[test]
fn test_all_error_kinds_have_unique_codes() {
    use std::collections::HashSet;

    let kinds = vec![
        ExErrorKind::InvalidInput,
        ExErrorKind::InvalidTitle,
        ExErrorKind::InvalidOrdinal,
        ExErrorKind::NotFound,
        ExErrorKind::Deleted,
        ExErrorKind::ConstraintViolation,
        ExErrorKind::IllegalReparent,
        ExErrorKind::CycleDetected,
        ExErrorKind::MultipleParents,
        ExErrorKind::DuplicateMapping,
        ExErrorKind::MissingMapping,
        ExErrorKind::AmbiguousSelection,
        ExErrorKind::TraversalBroken,
        ExErrorKind::DeletedNodeInTraversal,
        ExErrorKind::AmbiguousLeafSelection,
        ExErrorKind::DeterminismViolation,
        ExErrorKind::CannotDelete,
        ExErrorKind::StrandsChild,
        ExErrorKind::Internal,
    ];

    let codes: HashSet<_> = kinds.iter().map(|k| k.code()).collect();

    // All codes should be unique
    assert_eq!(codes.len(), kinds.len());

    // All codes should start with "ERR_"
    for code in codes {
        assert!(code.starts_with("ERR_"));
    }
}
