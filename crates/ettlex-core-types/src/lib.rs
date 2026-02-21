//! Core types shared across EttleX facilities
//!
//! This crate provides foundational types used by both error handling
//! and logging facilities:
//!
//! - **Correlation types**: RequestId, TraceId, SpanId, RequestContext
//! - **Sensitive data**: Sensitive<T> marker for automatic redaction
//! - **Schema constants**: Canonical field keys and event names

pub mod correlation;
pub mod schema;
pub mod sensitive;

pub use correlation::{RequestContext, RequestId, SpanId, TraceId};
pub use sensitive::Sensitive;
