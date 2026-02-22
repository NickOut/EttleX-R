//! Error handling for ettlex-store
//!
//! Wraps ettlex-core ExError with store-specific helpers

use ettlex_core::errors::{ExError, ExErrorKind};

/// Result type alias using ExError
pub type Result<T> = std::result::Result<T, ExError>;

/// Create a migration error
pub fn migration_error(migration_id: &str, reason: &str) -> ExError {
    ExError::new(ExErrorKind::Persistence)
        .with_op("migration")
        .with_message(format!("Migration {} failed: {}", migration_id, reason))
}

/// Create a checksum mismatch error
pub fn checksum_mismatch(migration_id: &str, expected: &str, actual: &str) -> ExError {
    ExError::new(ExErrorKind::ConstraintViolation)
        .with_op("migration_checksum")
        .with_message(format!(
            "Checksum mismatch for migration {}: expected {}, got {}",
            migration_id, expected, actual
        ))
}

/// Create a CAS collision error
pub fn cas_collision(digest: &str) -> ExError {
    ExError::new(ExErrorKind::ConstraintViolation)
        .with_op("cas_write")
        .with_message(format!("CAS collision for digest {}", digest))
}

/// Create a CAS missing blob error
pub fn cas_missing(digest: &str) -> ExError {
    ExError::new(ExErrorKind::NotFound)
        .with_op("cas_read")
        .with_message(format!("CAS blob not found for digest {}", digest))
}

/// Create a seed validation error
pub fn seed_validation(reason: &str) -> ExError {
    ExError::new(ExErrorKind::InvalidInput)
        .with_op("seed_parse")
        .with_message(reason.to_string())
}

/// Create a database error from rusqlite::Error
pub fn from_rusqlite(err: rusqlite::Error) -> ExError {
    ExError::new(ExErrorKind::Persistence)
        .with_op("sqlite")
        .with_message(err.to_string())
}

/// Create an IO error
pub fn io_error(operation: &str, err: std::io::Error) -> ExError {
    ExError::new(ExErrorKind::Io)
        .with_op(operation.to_string())
        .with_message(err.to_string())
}
