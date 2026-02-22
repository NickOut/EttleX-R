#![allow(clippy::unwrap_used, clippy::expect_used)]

use ettlex_core::errors::{EttleXError, ExErrorKind};
use ettlex_core::logging_facility::test_capture::init_test_capture;
use ettlex_core::{log_op_end, log_op_error, log_op_start};
use ettlex_core_types::schema::{EVENT_END, EVENT_END_ERROR, EVENT_START};

#[test]
fn test_log_op_start_macro() {
    let capture = init_test_capture();
    let op_name = "test_log_op_start_unique_1";

    log_op_start!(op_name);

    let events = capture.events();
    let start_events: Vec<_> = events
        .iter()
        .filter(|e| e.op.as_deref() == Some(op_name) && e.event.as_deref() == Some(EVENT_START))
        .collect();

    assert!(
        !start_events.is_empty(),
        "Should have captured at least one start event"
    );
}

#[test]
fn test_log_op_end_macro() {
    let capture = init_test_capture();
    let op_name = "test_log_op_end_unique_2";

    log_op_end!(op_name, duration_ms = 42);

    let events = capture.events();
    let end_events: Vec<_> = events
        .iter()
        .filter(|e| e.op.as_deref() == Some(op_name) && e.event.as_deref() == Some(EVENT_END))
        .collect();

    assert_eq!(end_events.len(), 1, "Should have exactly one end event");

    let end_event = end_events[0];
    assert_eq!(end_event.fields.get("duration_ms"), Some(&"42".to_string()));
}

#[test]
fn test_log_op_error_includes_kind() {
    let capture = init_test_capture();
    let op_name = "test_log_op_error_unique_3";

    let err = EttleXError::EttleNotFound {
        ettle_id: "e1".to_string(),
    };
    log_op_error!(op_name, err, duration_ms = 10);

    let events = capture.events();
    let error_events: Vec<_> = events
        .iter()
        .filter(|e| e.op.as_deref() == Some(op_name) && e.event.as_deref() == Some(EVENT_END_ERROR))
        .collect();

    assert_eq!(error_events.len(), 1, "Should have exactly one error event");

    let error_event = error_events[0];
    assert_eq!(
        error_event.fields.get("err.code"),
        Some(&"ERR_NOT_FOUND".to_string())
    );
}

#[test]
fn test_boundary_ownership_single_start_end() {
    let capture = init_test_capture();
    let op_name = "test_boundary_ownership_unique_4";

    log_op_start!(op_name, ettle_id = "e1");
    log_op_end!(op_name, duration_ms = 42);

    let events = capture.events();

    let starts = events
        .iter()
        .filter(|e| e.op.as_deref() == Some(op_name) && e.event.as_deref() == Some(EVENT_START))
        .count();

    let ends = events
        .iter()
        .filter(|e| e.op.as_deref() == Some(op_name) && e.event.as_deref() == Some(EVENT_END))
        .count();

    assert_eq!(starts, 1, "Should have exactly one start event");
    assert_eq!(ends, 1, "Should have exactly one end event");
}

#[test]
fn test_error_event_includes_error_code() {
    let capture = init_test_capture();
    let op_name = "test_error_event_unique_5";

    let err = EttleXError::CycleDetected {
        ettle_id: "e1".to_string(),
    };
    log_op_error!(op_name, err, duration_ms = 5);

    capture.assert_event_exists(op_name, EVENT_END_ERROR);

    let events = capture.events();
    let error_event = events
        .iter()
        .find(|e| e.op.as_deref() == Some(op_name) && e.event.as_deref() == Some(EVENT_END_ERROR))
        .expect("Should have error event");

    assert_eq!(
        error_event.fields.get("err.code"),
        Some(&"ERR_CYCLE_DETECTED".to_string())
    );
}

#[test]
fn test_log_macros_with_multiple_fields() {
    let capture = init_test_capture();
    let op_name = "test_log_macros_fields_unique_6";

    log_op_start!(op_name, ettle_id = "e123", title = "Test");

    let events = capture.events();
    let start_event = events
        .iter()
        .find(|e| e.op.as_deref() == Some(op_name))
        .expect("Should have start event");

    assert_eq!(
        start_event.fields.get("ettle_id"),
        Some(&"e123".to_string())
    );
    assert_eq!(start_event.fields.get("title"), Some(&"Test".to_string()));
}

#[test]
fn test_test_capture_assert_event_exists() {
    let capture = init_test_capture();
    let op_name = "test_capture_assert_unique_7";

    log_op_start!(op_name);

    // This should not panic
    capture.assert_event_exists(op_name, EVENT_START);
}

#[test]
#[should_panic(expected = "Expected event")]
fn test_test_capture_assert_event_exists_fails() {
    let capture = init_test_capture();

    // This should panic because no such event exists
    capture.assert_event_exists("nonexistent_op_truly_unique_999", EVENT_START);
}

#[test]
fn test_test_capture_count_events() {
    let capture = init_test_capture();
    let op1_name = "test_count_events_op1_unique_8";
    let op2_name = "test_count_events_op2_unique_8";

    log_op_start!(op1_name);
    log_op_start!(op2_name);
    log_op_end!(op1_name, duration_ms = 10);

    let start_count = capture.count_events(|e| {
        e.event.as_deref() == Some(EVENT_START)
            && (e.op.as_deref() == Some(op1_name) || e.op.as_deref() == Some(op2_name))
    });
    let end_count = capture.count_events(|e| {
        e.event.as_deref() == Some(EVENT_END)
            && (e.op.as_deref() == Some(op1_name) || e.op.as_deref() == Some(op2_name))
    });

    assert_eq!(start_count, 2);
    assert_eq!(end_count, 1);
}

#[test]
fn test_error_conversion_preserves_context() {
    let capture = init_test_capture();
    let op_name = "test_error_conversion_unique_9";

    let err = EttleXError::DeleteWithChildren {
        ettle_id: "e1".to_string(),
        child_count: 3,
    };

    log_op_error!(op_name, err.clone(), duration_ms = 5);

    let events = capture.events();
    let error_event = events
        .iter()
        .find(|e| e.op.as_deref() == Some(op_name) && e.event.as_deref() == Some(EVENT_END_ERROR))
        .expect("Should have error event for this test");

    // Verify the error was converted to ExError with correct kind
    use ettlex_core::errors::ExError;
    let ex_err: ExError = err.into();
    assert_eq!(ex_err.kind(), ExErrorKind::CannotDelete);

    // Verify the error code is in the logged event
    assert_eq!(
        error_event.fields.get("err.code"),
        Some(&"ERR_CANNOT_DELETE".to_string())
    );
}

#[test]
fn test_multiple_operations_logged_independently() {
    let capture = init_test_capture();
    let op1_name = "test_multi_ops_create_ettle_unique_10";
    let op2_name = "test_multi_ops_create_ep_unique_10";

    // Op 1
    log_op_start!(op1_name);
    log_op_end!(op1_name, duration_ms = 10);

    // Op 2
    log_op_start!(op2_name);
    log_op_end!(op2_name, duration_ms = 5);

    let events = capture.events();

    let op1_events = events
        .iter()
        .filter(|e| e.op.as_deref() == Some(op1_name))
        .count();
    let op2_events = events
        .iter()
        .filter(|e| e.op.as_deref() == Some(op2_name))
        .count();

    assert_eq!(op1_events, 2); // start + end
    assert_eq!(op2_events, 2); // start + end
}
