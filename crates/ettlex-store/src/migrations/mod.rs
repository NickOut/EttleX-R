//! Migration framework
//!
//! Provides:
//! - Migration runner with checksums and gap detection
//! - Idempotent application
//! - Embedded SQL migrations

mod checksums;
mod embedded;
mod runner;

pub use runner::apply_migrations;
