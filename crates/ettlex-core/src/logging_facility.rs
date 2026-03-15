//! Structured logging facility for EttleX
//!
//! Re-exported from `ettlex-logging`. Use `ettlex_logging::init`, `ettlex_logging::log_op_start!`
//! etc. directly, or via this module for backward compatibility.

pub mod macros;
pub mod test_capture;

// Re-export init function and Profile from ettlex-logging
pub use ettlex_logging::{init, Profile};
pub use test_capture::{init_test_capture, CapturedEvent, TestCapture};
