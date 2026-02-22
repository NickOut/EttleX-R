//! EttleX Store - Persistence layer with SQLite, CAS, and seed import
//!
//! Provides:
//! - SQLite schema with migrations framework
//! - Content-addressable storage (CAS) for EP content
//! - Seed Format v0 parser and importer
//! - Repository layer bridging Phase 0.5 domain models to persistence

pub mod cas;
pub mod db;
pub mod errors;
pub mod migrations;
pub mod repo;
pub mod seed;
pub mod snapshot;

// Re-export key types
pub use errors::Result;
