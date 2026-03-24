//! Boundary mapping — single designated module for ExError → external type conversions.
//!
//! The agent API returns `Result<T, ExError>` directly. This module exists as the
//! single designated boundary for any future error type conversion or display mapping.
//! No error mapping logic should appear in `src/operations/*.rs`.

use ettlex_memory::ExError;

/// Format an ExError as a human-readable string for display purposes.
pub fn display_error(err: &ExError) -> String {
    format!("[{:?}] {}", err.kind(), err.message())
}
