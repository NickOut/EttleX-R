//! Auth configuration and token validation for the MCP server.

use crate::error::{McpError, MCP_AUTH_REQUIRED};

/// Authentication configuration for the MCP server.
///
/// Supports a single shared token for development/testing.
#[derive(Debug, Clone)]
pub struct AuthConfig {
    /// If `None`, auth is disabled (any token accepted).
    required_token: Option<String>,
}

impl AuthConfig {
    /// Disable authentication (accept any or no token).
    pub fn disabled() -> Self {
        Self {
            required_token: None,
        }
    }

    /// Require the given token on every request.
    pub fn with_token(token: impl Into<String>) -> Self {
        Self {
            required_token: Some(token.into()),
        }
    }

    /// Validate the provided token against this config.
    ///
    /// Returns `Ok(())` on success or `Err(McpError { AuthRequired })` on failure.
    pub fn validate(&self, token: &Option<String>) -> Result<(), McpError> {
        match &self.required_token {
            None => Ok(()), // auth disabled
            Some(required) => match token {
                Some(t) if t == required => Ok(()),
                _ => Err(McpError {
                    error_code: MCP_AUTH_REQUIRED.to_string(),
                    message: "Authentication required: missing or invalid token".to_string(),
                }),
            },
        }
    }
}
