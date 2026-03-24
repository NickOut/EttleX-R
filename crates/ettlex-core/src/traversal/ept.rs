//! EP Traversal (EPT) — retired in Slice 03.
//!
//! EPT was based on the EP construct which has been retired. This module
//! is a stub that returns `NotImplemented` for all operations.

use crate::errors::{ExError, ExErrorKind, Result};
use crate::ops::Store;

/// Compute EP Traversal — RETIRED. Returns `NotImplemented`.
///
/// # Errors
/// Always returns `NotImplemented` — EPT is retired in Slice 03.
#[allow(unused_variables)]
pub fn compute_ept(
    _store: &Store,
    _leaf_id: &str,
    _leaf_ep_ordinal: Option<u32>,
) -> Result<Vec<String>> {
    Err(ExError::new(ExErrorKind::NotImplemented)
        .with_message("EPT traversal retired in Slice 03 — EP construct removed"))
}
