//! Test capture mode for deterministic logging assertions
//!
//! This module provides a test-only subscriber that captures log events
//! in memory for assertion in tests.

use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use tracing::field::Visit;
use tracing::{Level, Subscriber};
use tracing_subscriber::layer::{Context, SubscriberExt};
use tracing_subscriber::registry::LookupSpan;
use tracing_subscriber::util::SubscriberInitExt;
use tracing_subscriber::Layer;

/// A captured log event with all its fields
#[derive(Clone, Debug)]
pub struct CapturedEvent {
    pub level: Level,
    pub component: Option<String>,
    pub op: Option<String>,
    pub event: Option<String>,
    pub fields: HashMap<String, String>,
}

struct FieldVisitor {
    fields: HashMap<String, String>,
}

impl FieldVisitor {
    fn new() -> Self {
        Self {
            fields: HashMap::new(),
        }
    }
}

impl Visit for FieldVisitor {
    fn record_debug(&mut self, field: &tracing::field::Field, value: &dyn std::fmt::Debug) {
        self.fields
            .insert(field.name().to_string(), format!("{:?}", value));
    }

    fn record_str(&mut self, field: &tracing::field::Field, value: &str) {
        self.fields
            .insert(field.name().to_string(), value.to_string());
    }

    fn record_i64(&mut self, field: &tracing::field::Field, value: i64) {
        self.fields
            .insert(field.name().to_string(), value.to_string());
    }

    fn record_u64(&mut self, field: &tracing::field::Field, value: u64) {
        self.fields
            .insert(field.name().to_string(), value.to_string());
    }
}

/// Test capture layer for collecting log events
pub struct TestCaptureLayer {
    events: Arc<Mutex<Vec<CapturedEvent>>>,
}

impl TestCaptureLayer {
    pub fn new() -> (Self, TestCapture) {
        let events = Arc::new(Mutex::new(Vec::new()));
        let layer = Self {
            events: events.clone(),
        };
        let capture = TestCapture { events };
        (layer, capture)
    }
}

impl<S> Layer<S> for TestCaptureLayer
where
    S: Subscriber + for<'a> LookupSpan<'a>,
{
    fn on_event(&self, event: &tracing::Event<'_>, _ctx: Context<'_, S>) {
        let metadata = event.metadata();
        let mut visitor = FieldVisitor::new();
        event.record(&mut visitor);

        let captured = CapturedEvent {
            level: *metadata.level(),
            component: visitor.fields.get("component").cloned(),
            op: visitor.fields.get("op").cloned(),
            event: visitor.fields.get("event").cloned(),
            fields: visitor.fields,
        };

        self.events
            .lock()
            .map(|mut events| events.push(captured))
            .ok();
    }
}

/// Handle for accessing captured events in tests
pub struct TestCapture {
    events: Arc<Mutex<Vec<CapturedEvent>>>,
}

impl TestCapture {
    /// Get all captured events
    pub fn events(&self) -> Vec<CapturedEvent> {
        self.events.lock().map(|e| e.clone()).unwrap_or_default()
    }

    /// Assert that an event exists with the given operation and event type
    ///
    /// # Panics
    ///
    /// Panics if the event is not found
    pub fn assert_event_exists(&self, op: &str, event: &str) {
        let events = self.events();
        let found = events
            .iter()
            .any(|e| e.op.as_deref() == Some(op) && e.event.as_deref() == Some(event));
        assert!(
            found,
            "Expected event op={} event={} not found in {} captured events",
            op,
            event,
            events.len()
        );
    }

    /// Clear all captured events
    pub fn clear(&self) {
        self.events.lock().map(|mut e| e.clear()).ok();
    }

    /// Count events matching a predicate
    pub fn count_events<F>(&self, predicate: F) -> usize
    where
        F: Fn(&CapturedEvent) -> bool,
    {
        self.events().iter().filter(|e| predicate(e)).count()
    }
}

use std::sync::OnceLock;

static GLOBAL_CAPTURE: OnceLock<TestCapture> = OnceLock::new();

/// Initialize test capture mode
///
/// This should be called at the start of each test that needs to capture logs.
/// Returns a shared global capture instance.
///
/// # Example
///
/// ```
/// use ettlex_core::logging_facility::test_capture::init_test_capture;
/// use ettlex_core::log_op_start;
///
/// let capture = init_test_capture();
/// log_op_start!("my_operation");
/// capture.assert_event_exists("my_operation", "start");
/// ```
pub fn init_test_capture() -> TestCapture {
    GLOBAL_CAPTURE
        .get_or_init(|| {
            let (layer, capture) = TestCaptureLayer::new();
            tracing_subscriber::registry().with(layer).init();
            capture
        })
        .clone()
}

impl Clone for TestCapture {
    fn clone(&self) -> Self {
        Self {
            events: self.events.clone(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_captured_event_clone() {
        let event = CapturedEvent {
            level: Level::INFO,
            component: Some("test".to_string()),
            op: Some("test_op".to_string()),
            event: Some("start".to_string()),
            fields: HashMap::new(),
        };

        let cloned = event.clone();
        assert_eq!(cloned.level, event.level);
        assert_eq!(cloned.op, event.op);
    }
}
