//! Repository layer for persisting domain models to SQLite
//!
//! Bridges Phase 0.5 in-memory Store to SQLite persistence

pub mod hydration;
pub mod sqlite_repo;

pub use sqlite_repo::SqliteRepo;
