//! Structured logging facility for EttleX
//!
//! This module provides a canonical logging facility with:
//! - Single initialization point via `init(profile)`
//! - Structured logging macros (`log_op_start!`, `log_op_end!`, `log_op_error!`)
//! - Boundary ownership enforcement (prevents duplicate start/end)
//! - Correlation propagation via spans
//! - Test capture mode for deterministic assertions
//!
//! # Usage
//!
//! ```rust
//! use ettlex_core::logging_facility::{init, Profile};
//!
//! // Initialize once at application startup
//! init(Profile::Development);
//! ```
//!
//! # Logging Macros
//!
//! - `log_op_start!(op, ...)` - Log operation start
//! - `log_op_end!(op, duration_ms = ...)` - Log operation end
//! - `log_op_error!(op, err, duration_ms = ...)` - Log operation error

pub mod init;
pub mod macros;
pub mod test_capture;

pub use init::{init, Profile};
pub use test_capture::{init_test_capture, CapturedEvent, TestCapture};
