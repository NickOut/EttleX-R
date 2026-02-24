//! Query module for read-only operations
//!
//! This module provides deterministic, read-only query operations for decisions
//! and other entities. Queries return specialized result types and support
//! pagination, filtering, and aggregation.
//!
//! Key principles:
//! - All queries are read-only (no mutations)
//! - Results are deterministically ordered
//! - Support for cursor-based pagination
//! - Filtering by status, relation, tombstone state

pub mod decision_queries;

pub use decision_queries::{
    decision_get, decision_list, ep_list_decisions, ep_list_decisions_with_ancestors,
    ept_compute_decision_context, DecisionContext, DecisionDetail, DecisionFilters,
    PaginatedDecisions, PaginationParams,
};
