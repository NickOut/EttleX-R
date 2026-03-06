//! MCP-layer error types and mapping from engine errors.

use ettlex_core::errors::ExError;

// ---------------------------------------------------------------------------
// MCP error codes
// ---------------------------------------------------------------------------

pub const MCP_AUTH_REQUIRED: &str = "AuthRequired";
pub const MCP_TOOL_NOT_FOUND: &str = "ToolNotFound";
pub const MCP_INVALID_CURSOR: &str = "InvalidCursor";
pub const MCP_INVALID_COMMAND: &str = "InvalidCommand";
pub const MCP_INVALID_INPUT: &str = "InvalidInput";
pub const MCP_REQUEST_TOO_LARGE: &str = "RequestTooLarge";
pub const MCP_RESPONSE_TOO_LARGE: &str = "ResponseTooLarge";

// ---------------------------------------------------------------------------
// McpError
// ---------------------------------------------------------------------------

/// A structured MCP-layer error returned in `McpResult::Err`.
#[derive(Debug, Clone)]
pub struct McpError {
    /// Short stable code, e.g. `"AuthRequired"`, `"NotFound"`.
    pub error_code: String,
    /// Human-readable message.
    pub message: String,
}

impl McpError {
    /// Construct a new `McpError` with a static code.
    pub fn new(error_code: &str, message: impl Into<String>) -> Self {
        Self {
            error_code: error_code.to_string(),
            message: message.into(),
        }
    }

    /// Map an `ExError` from the engine/store to an `McpError`.
    ///
    /// The MCP error code is derived from `ExError::kind().code()` by:
    ///   1. Stripping the `ERR_` prefix.
    ///   2. Converting SCREAMING_SNAKE_CASE words to CamelCase.
    pub fn from_ex_error(e: ExError) -> Self {
        let code = ex_code_to_mcp(e.kind().code());
        let message = e.to_string();
        Self {
            error_code: code,
            message,
        }
    }
}

/// Convert `"ERR_SOME_CODE"` → `"SomeCode"`.
fn ex_code_to_mcp(code: &str) -> String {
    let stripped = code.strip_prefix("ERR_").unwrap_or(code);
    stripped
        .split('_')
        .map(|word| {
            let mut chars = word.chars();
            match chars.next() {
                None => String::new(),
                Some(c) => c.to_uppercase().to_string() + &chars.as_str().to_lowercase(),
            }
        })
        .collect()
}

// ---------------------------------------------------------------------------
// McpResult / McpResponse (here for shared use by server + tests)
// ---------------------------------------------------------------------------

/// The result payload of an MCP dispatch call.
#[derive(Debug, Clone)]
pub enum McpResult {
    Ok(serde_json::Value),
    Err(McpError),
}

/// Complete response returned from `McpServer::dispatch`.
#[derive(Debug, Clone)]
pub struct McpResponse {
    /// Echoed correlation_id from the request context (if provided).
    pub correlation_id: Option<String>,
    /// The result — success payload or error.
    pub result: McpResult,
}
