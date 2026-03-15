//! EttleX canonical logging facility
//!
//! This crate provides the foundational logging infrastructure for the EttleX workspace:
//! - `init(profile)` — single initialisation point
//! - `log_op_start!`, `log_op_end!`, `log_op_error!` — structured logging macros
//! - `TestCapture` / `init_test_capture()` — deterministic test capture mode

pub mod init;
pub mod macros;
pub mod test_capture;

pub use init::{init, Profile};
pub use test_capture::{init_test_capture, CapturedEvent, TestCapture, TestCaptureLayer};
