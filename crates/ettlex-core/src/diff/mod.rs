//! Snapshot diff engine.
//!
//! Compares two committed snapshot manifests and produces a structured,
//! deterministic diff suitable for downstream evaluators and human review.
//!
//! ## Entry point
//!
//! ```ignore
//! use ettlex_core::diff::engine::compute_diff;
//!
//! let diff = compute_diff(a_bytes, b_bytes)?;
//! let summary = ettlex_core::diff::human_summary::render_human_summary(&diff);
//! ```
//!
//! ## Guarantees
//!
//! - **Determinism**: identical inputs produce byte-identical structured diff output.
//! - **created_at noise suppression**: `created_at` differences are never treated as
//!   semantic changes.
//! - **Additive manifest compatibility**: unknown future manifest fields are reported
//!   only in `unknown_changes`, not as errors.
//! - **Constraint-family agnosticism**: the diff operates on the constraints envelope
//!   without knowledge of specific families.

pub mod engine;
pub mod human_summary;
pub mod model;

pub use engine::compute_diff;
pub use human_summary::render_human_summary;
pub use model::SnapshotDiff;
