//! Canonical schema constants for structured logging and events
//!
//! These constants ensure consistency across all logging and error reporting.

// Canonical field keys for structured logging
pub const FIELD_COMPONENT: &str = "component";
pub const FIELD_OP: &str = "op";
pub const FIELD_EVENT: &str = "event";
pub const FIELD_DURATION_MS: &str = "duration_ms";
pub const FIELD_REQUEST_ID: &str = "request_id";
pub const FIELD_TRACE_ID: &str = "trace_id";
pub const FIELD_SPAN_ID: &str = "span_id";

// Entity identifiers
pub const FIELD_ETTLE_ID: &str = "ettle_id";
pub const FIELD_EP_ID: &str = "ep_id";
pub const FIELD_EP_ORDINAL: &str = "ep_ordinal";

// Collection sizes
pub const FIELD_RT_LEN: &str = "rt_len";
pub const FIELD_EPT_LEN: &str = "ept_len";

// Error fields
pub const FIELD_ERR_KIND: &str = "err.kind";
pub const FIELD_ERR_CODE: &str = "err.code";

// Canonical event names
pub const EVENT_START: &str = "start";
pub const EVENT_END: &str = "end";
pub const EVENT_END_ERROR: &str = "end_error";

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_constants_accessibility() {
        // Verify all constants are non-empty
        assert!(!FIELD_COMPONENT.is_empty());
        assert!(!FIELD_OP.is_empty());
        assert!(!EVENT_START.is_empty());
        assert!(!EVENT_END.is_empty());
        assert!(!EVENT_END_ERROR.is_empty());
    }

    #[test]
    fn test_event_names_are_distinct() {
        assert_ne!(EVENT_START, EVENT_END);
        assert_ne!(EVENT_START, EVENT_END_ERROR);
        assert_ne!(EVENT_END, EVENT_END_ERROR);
    }
}
