//! Bundle render (EP-era bundle rendering retired — Slice 03).
//!
//! EPT-based bundle rendering has been retired along with the EP construct.
//! Rendering will be re-specified against the Ettle/Relation model in a future slice.

use crate::errors::{ExError, ExErrorKind, Result};
use crate::ops::Store;

/// Render a leaf bundle to Markdown — RETIRED in Slice 03.
///
/// Returns `NotImplemented` as the EPT-based bundle pipeline has been
/// deferred pending re-specification against the Ettle/Relation model.
///
/// # Errors
/// Always returns `NotImplemented` — bundle render is retired in Slice 03.
#[allow(unused_variables)]
pub fn render_leaf_bundle(
    store: &Store,
    leaf_id: &str,
    leaf_ep_ordinal: Option<u32>,
) -> Result<String> {
    Err(ExError::new(ExErrorKind::NotImplemented).with_message(
        "Bundle render retired in Slice 03 — EP construct removed. \
             Re-specify against Ettle/Relation model.",
    ))
}
