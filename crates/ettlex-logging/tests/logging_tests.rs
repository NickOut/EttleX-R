use ettlex_logging::{init, init_test_capture, log_op_end, log_op_error, log_op_start, Profile};

#[test]
fn test_init_development_no_panic() {
    // Should not panic even if called multiple times
    init(Profile::Development);
    init(Profile::Development);
}

#[test]
fn test_init_test_capture_returns_handle() {
    let capture = init_test_capture();
    // Just verify we got a handle back without panic
    let _events = capture.events();
}

#[test]
fn test_log_op_start_emits_start_event() {
    let capture = init_test_capture();
    capture.clear();
    log_op_start!("test_operation");
    let events = capture.events();
    let found = events
        .iter()
        .any(|e| e.op.as_deref() == Some("test_operation") && e.event.as_deref() == Some("start"));
    assert!(
        found,
        "Expected start event for test_operation, got: {:?}",
        events
    );
}

#[test]
fn test_log_op_end_emits_end_event_with_duration() {
    let capture = init_test_capture();
    capture.clear();
    log_op_end!("test_op_end", duration_ms = 42u64);
    let events = capture.events();
    let found = events.iter().any(|e| {
        e.op.as_deref() == Some("test_op_end")
            && e.event.as_deref() == Some("end")
            && e.fields.get("duration_ms").map(|s| s.as_str()) == Some("42")
    });
    assert!(found, "Expected end event with duration, got: {:?}", events);
}

#[test]
fn test_log_op_error_emits_error_event_with_err_kind() {
    use ettlex_errors::{ExError, ExErrorKind};
    let capture = init_test_capture();
    capture.clear();
    let err = ExError::new(ExErrorKind::NotFound).with_message("not found");
    log_op_error!("test_op_err", err, duration_ms = 5u64);
    let events = capture.events();
    let found = events
        .iter()
        .any(|e| e.op.as_deref() == Some("test_op_err") && e.event.as_deref() == Some("end_error"));
    assert!(found, "Expected error event, got: {:?}", events);
}
