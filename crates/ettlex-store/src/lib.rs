//! EttleX Store - Persistence layer with SQLite and CAS
//!
//! Provides:
//! - SQLite schema with migrations framework
//! - Content-addressable storage (CAS) for blob storage
//! - Repository layer for domain models (Ettle, Relation, Group, Decision, Snapshot, Profile)

pub mod cas;
pub mod db;
pub mod errors;
pub mod file_policy_provider;
pub mod migrations;
pub mod model;
pub mod profile;
pub mod repo;
pub mod snapshot;

// Re-export key types
pub use errors::Result;
