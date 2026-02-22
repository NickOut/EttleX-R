//! Database connection management
//!
//! Provides utilities for opening and managing SQLite connections

#![allow(clippy::result_large_err)]

use crate::errors::{from_rusqlite, Result};
use rusqlite::Connection;
use std::path::Path;

/// Open a SQLite database at the given path
pub fn open<P: AsRef<Path>>(path: P) -> Result<Connection> {
    Connection::open(path).map_err(from_rusqlite)
}

/// Open an in-memory SQLite database (for testing)
pub fn open_in_memory() -> Result<Connection> {
    Connection::open_in_memory().map_err(from_rusqlite)
}

/// Configure a connection with optimal settings
pub fn configure(conn: &Connection) -> Result<()> {
    // Enable foreign keys
    conn.execute("PRAGMA foreign_keys = ON", [])
        .map_err(from_rusqlite)?;

    // Set WAL mode for better concurrency
    conn.execute("PRAGMA journal_mode = WAL", [])
        .map_err(from_rusqlite)?;

    Ok(())
}
