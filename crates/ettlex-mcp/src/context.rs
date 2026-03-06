//! Per-request context threaded through the MCP dispatch pipeline.

/// Contextual metadata attached to an MCP tool call.
///
/// Correlation IDs are threaded through from the caller and echoed in
/// the response. MCP never injects or defaults them.
#[derive(Debug, Clone, Default)]
pub struct RequestContext {
    /// Optional caller-supplied correlation ID for request tracing.
    pub correlation_id: Option<String>,
}
