//! Correlation types for request tracking and tracing
//!
//! These types enable correlation of operations across async boundaries
//! and provide context propagation for distributed tracing.

use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// Unique identifier for a single request or operation
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct RequestId(String);

impl RequestId {
    /// Generate a new random RequestId using UUIDv7
    pub fn new() -> Self {
        Self(Uuid::now_v7().to_string())
    }

    /// Get the string representation
    pub fn as_str(&self) -> &str {
        &self.0
    }

    /// Create from an existing string (for deserialization)
    pub fn from_string(s: String) -> Self {
        Self(s)
    }
}

impl Default for RequestId {
    fn default() -> Self {
        Self::new()
    }
}

impl std::fmt::Display for RequestId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

/// Trace identifier for distributed tracing across service boundaries
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct TraceId(String);

impl TraceId {
    /// Generate a new random TraceId using UUIDv7
    pub fn new() -> Self {
        Self(Uuid::now_v7().to_string())
    }

    /// Get the string representation
    pub fn as_str(&self) -> &str {
        &self.0
    }

    /// Create from an existing string (for deserialization)
    pub fn from_string(s: String) -> Self {
        Self(s)
    }
}

impl Default for TraceId {
    fn default() -> Self {
        Self::new()
    }
}

impl std::fmt::Display for TraceId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

/// Span identifier for hierarchical tracing within a trace
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct SpanId(String);

impl SpanId {
    /// Generate a new random SpanId using UUIDv7
    pub fn new() -> Self {
        Self(Uuid::now_v7().to_string())
    }

    /// Get the string representation
    pub fn as_str(&self) -> &str {
        &self.0
    }

    /// Create from an existing string (for deserialization)
    pub fn from_string(s: String) -> Self {
        Self(s)
    }
}

impl Default for SpanId {
    fn default() -> Self {
        Self::new()
    }
}

impl std::fmt::Display for SpanId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

/// Context carried through operation boundaries for correlation
#[derive(Debug, Clone)]
pub struct RequestContext {
    pub request_id: RequestId,
    pub trace_id: Option<TraceId>,
}

impl RequestContext {
    /// Create a new context with a fresh RequestId
    pub fn new() -> Self {
        Self {
            request_id: RequestId::new(),
            trace_id: None,
        }
    }

    /// Create a context with an existing RequestId
    pub fn with_request_id(request_id: RequestId) -> Self {
        Self {
            request_id,
            trace_id: None,
        }
    }

    /// Add a TraceId to the context
    pub fn with_trace_id(mut self, trace_id: TraceId) -> Self {
        self.trace_id = Some(trace_id);
        self
    }
}

impl Default for RequestContext {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_request_id_generation() {
        let id1 = RequestId::new();
        let id2 = RequestId::new();

        // Should generate different IDs
        assert_ne!(id1, id2);

        // Should be non-empty strings
        assert!(!id1.as_str().is_empty());
        assert!(!id2.as_str().is_empty());
    }

    #[test]
    fn test_request_id_display() {
        let id = RequestId::new();
        let display_str = format!("{}", id);
        assert_eq!(display_str, id.as_str());
    }

    #[test]
    fn test_trace_id_generation() {
        let id1 = TraceId::new();
        let id2 = TraceId::new();

        assert_ne!(id1, id2);
        assert!(!id1.as_str().is_empty());
    }

    #[test]
    fn test_span_id_generation() {
        let id1 = SpanId::new();
        let id2 = SpanId::new();

        assert_ne!(id1, id2);
        assert!(!id1.as_str().is_empty());
    }

    #[test]
    fn test_request_context_creation() {
        let ctx = RequestContext::new();
        assert!(!ctx.request_id.as_str().is_empty());
        assert!(ctx.trace_id.is_none());
    }

    #[test]
    fn test_request_context_with_trace_id() {
        let trace_id = TraceId::new();
        let ctx = RequestContext::new().with_trace_id(trace_id.clone());

        assert!(ctx.trace_id.is_some());
        assert_eq!(ctx.trace_id.unwrap(), trace_id);
    }

    #[test]
    fn test_serialization() {
        let id = RequestId::new();
        let json = serde_json::to_string(&id).unwrap();
        let deserialized: RequestId = serde_json::from_str(&json).unwrap();
        assert_eq!(id, deserialized);
    }
}
