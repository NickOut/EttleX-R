use ettlex_errors::{assert_err_kind, ExError, ExErrorKind};

#[test]
fn test_ex_error_builder_all_fields() {
    use ettlex_core_types::{RequestId, TraceId};
    let rid = RequestId::new();
    let tid = TraceId::new();
    let err = ExError::new(ExErrorKind::NotFound)
        .with_op("test_op")
        .with_entity_id("e-123")
        .with_ep_id("ep-456")
        .with_ordinal(3)
        .with_request_id(rid.clone())
        .with_trace_id(tid.clone())
        .with_message("test message");
    assert_eq!(err.kind(), ExErrorKind::NotFound);
    assert_eq!(err.op(), Some("test_op"));
    assert_eq!(err.entity_id(), Some("e-123"));
    assert_eq!(err.ep_id(), Some("ep-456"));
    assert_eq!(err.ordinal(), Some(3));
    assert_eq!(err.message(), "test message");
    assert_eq!(err.code(), "ERR_NOT_FOUND");
}

#[test]
fn test_assert_err_kind_passes_on_correct_kind() {
    let err = ExError::new(ExErrorKind::NotFound).with_message("not found");
    assert_err_kind!(err, ExErrorKind::NotFound);
}

#[test]
#[should_panic]
fn test_assert_err_kind_fails_on_wrong_kind() {
    let err = ExError::new(ExErrorKind::NotFound).with_message("not found");
    assert_err_kind!(err, ExErrorKind::Internal);
}

#[test]
fn test_already_tombstoned_code() {
    let err = ExError::new(ExErrorKind::AlreadyTombstoned);
    assert_eq!(err.code(), "ERR_ALREADY_TOMBSTONED");
}

#[test]
fn test_self_referential_link_code() {
    let err = ExError::new(ExErrorKind::SelfReferentialLink);
    assert_eq!(err.code(), "ERR_SELF_REFERENTIAL_LINK");
}

#[test]
fn test_has_active_dependants_code() {
    let err = ExError::new(ExErrorKind::HasActiveDependants);
    assert_eq!(err.code(), "ERR_HAS_ACTIVE_DEPENDANTS");
}

#[test]
fn test_missing_link_type_code() {
    let err = ExError::new(ExErrorKind::MissingLinkType);
    assert_eq!(err.code(), "ERR_MISSING_LINK_TYPE");
}

#[test]
fn test_new_variants_distinct() {
    let variants = [
        ExErrorKind::AlreadyTombstoned,
        ExErrorKind::SelfReferentialLink,
        ExErrorKind::HasActiveDependants,
        ExErrorKind::MissingLinkType,
        ExErrorKind::Deleted,
    ];
    // All codes must be distinct
    let codes: std::collections::HashSet<&str> = variants.iter().map(|v| v.code()).collect();
    assert_eq!(codes.len(), variants.len());
}
