//! Engine-level read-only query surface.
//!
//! `apply_engine_query` is the single entry point for all read-only queries
//! that span the store and core layers. Unlike `apply_engine_command`, it
//! accepts a shared (non-mutable) connection and never writes to the DB or CAS.

#![allow(clippy::result_large_err)]

use ettlex_core::candidate_resolver::{
    compute_dry_run_resolution, AmbiguityPolicy, CandidateEntry, DryRunConstraintStatus,
};
use ettlex_core::diff;
use ettlex_core::diff::human_summary::render_human_summary;
use ettlex_core::diff::model::SnapshotDiff;
use ettlex_core::errors::{ExError, ExErrorKind};
use ettlex_core::{log_op_end, log_op_error, log_op_start};
use ettlex_store::cas::FsStore;
use ettlex_store::errors::Result;
use ettlex_store::profile::{
    fetch_approval_row, list_approval_rows_paginated, list_profiles_paginated,
    load_default_profile, load_profile_full, ApprovalRow,
};
use ettlex_store::repo::SqliteRepo;
use ettlex_store::snapshot::query::{
    fetch_manifest_bytes_by_digest, fetch_snapshot_manifest_digest, fetch_snapshot_row,
    list_snapshot_rows,
};
use rusqlite::Connection;

use crate::commands::read_tools::{
    ApprovalGetResult, ApprovalListItem, ApprovalPage, DecisionPage, EttleGetResult, EttlePage,
    ListOptions, ManifestGetResult, Page, PolicyExportResult, PolicyProjectForHandoffResult,
    PolicyReadResult, PredicatePreviewResult, PreviewStatus, ProfileGetResult, ProfilePage,
    ProfileResolveResult, SnapshotGetResult, StateVersionResult,
};

// ---------------------------------------------------------------------------
// Snapshot diff types (re-exported for backward compat)
// ---------------------------------------------------------------------------

/// A reference to a snapshot that can be resolved to manifest bytes.
#[derive(Debug, Clone)]
pub enum SnapshotRef {
    /// Resolved via the `snapshots` table (`snapshot_id → manifest_digest → CAS`).
    SnapshotId(String),
    /// Resolved directly from CAS by manifest digest.
    ManifestDigest(String),
}

/// The structured + rendered result of a `SnapshotDiff` query.
#[derive(Debug, Clone)]
pub struct SnapshotDiffResult {
    /// Machine-readable structured diff
    pub structured_diff: SnapshotDiff,
    /// Human-readable Markdown summary
    pub human_summary: String,
}

// ---------------------------------------------------------------------------
// EngineQuery
// ---------------------------------------------------------------------------

/// Read-only queries supported by the engine.
#[derive(Debug, Clone)]
pub enum EngineQuery {
    // ── Existing ─────────────────────────────────────────────────────────────
    /// Compute a structured diff between two snapshot manifests.
    SnapshotDiff {
        /// Reference to snapshot A
        a_ref: SnapshotRef,
        /// Reference to snapshot B
        b_ref: SnapshotRef,
    },

    // ── State ─────────────────────────────────────────────────────────────────
    /// Get the current schema version and semantic head digest.
    StateGetVersion,

    // ── Ettle ─────────────────────────────────────────────────────────────────
    /// Get an ettle by ID.
    EttleGet { ettle_id: String },
    /// List ettles with pagination.
    EttleList(ListOptions),

    // ── Constraint ────────────────────────────────────────────────────────────
    /// Get a constraint by ID (including tombstoned).
    ConstraintGet { constraint_id: String },
    /// List constraints for a family.
    ConstraintListByFamily {
        family: String,
        include_tombstoned: bool,
    },

    // ── Decision ─────────────────────────────────────────────────────────────
    /// Get a decision by ID (including tombstoned).
    DecisionGet { decision_id: String },
    /// List all decisions with pagination.
    DecisionList(ListOptions),
    /// List decisions linked to a target entity.
    DecisionListByTarget {
        target_kind: String,
        target_id: String,
        include_tombstoned: bool,
    },
    /// List decisions for an ettle, optionally including ancestors.
    EttleListDecisions {
        ettle_id: String,
        include_eps: bool,
        include_ancestors: bool,
    },

    // ── Snapshot / Manifest ───────────────────────────────────────────────────
    /// Get a snapshot ledger row by snapshot ID.
    SnapshotGet { snapshot_id: String },
    /// List snapshot rows, optionally filtered by root ettle ID.
    SnapshotList { ettle_id: Option<String> },
    /// Get manifest bytes for a snapshot by snapshot ID.
    ManifestGetBySnapshot { snapshot_id: String },
    /// Get manifest bytes for a snapshot by manifest digest.
    ManifestGetByDigest { manifest_digest: String },

    // ── Profile ──────────────────────────────────────────────────────────────
    /// Get a profile by reference.
    ProfileGet { profile_ref: String },
    /// Resolve a profile (None → use default).
    ProfileResolve { profile_ref: Option<String> },
    /// Get the default profile.
    ProfileGetDefault,
    /// List profiles with pagination.
    ProfileList(ListOptions),

    // ── Approval ─────────────────────────────────────────────────────────────
    /// Get an approval request by token.
    ApprovalGet { approval_token: String },
    /// List approval requests with pagination.
    ApprovalList(ListOptions),
    /// List approval requests filtered by kind (NotImplemented in Phase 1).
    ApprovalListByKind { kind: String, options: ListOptions },

    // ── Predicate preview ────────────────────────────────────────────────────
    /// Preview constraint predicate resolution without side effects.
    ConstraintPredicatesPreview {
        profile_ref: Option<String>,
        context: serde_json::Value,
        candidates: Vec<String>,
    },

    // ── Policy ───────────────────────────────────────────────────────────────
    /// List all policies available in the provider.
    PolicyList,
    /// Return the full canonical text of a policy document.
    PolicyRead { policy_ref: String },
    /// Export structured content from a policy document (e.g. HANDOFF obligations).
    PolicyExport {
        policy_ref: String,
        export_kind: String,
    },
    /// Return the `policy_ref` recorded in a committed snapshot manifest.
    SnapshotManifestPolicyRef { manifest_digest: String },
    /// Produce a deterministic byte projection of a policy document for handoff.
    PolicyProjectForHandoff {
        policy_ref: String,
        profile_ref: Option<String>,
    },

    // ── Snapshot head ─────────────────────────────────────────────────────────
    /// Get the manifest digest of the most recent committed snapshot for an ettle.
    SnapshotGetHead { realised_ettle_id: String },
}

// ---------------------------------------------------------------------------
// EngineQueryResult
// ---------------------------------------------------------------------------

/// All possible results from `apply_engine_query`.
#[derive(Debug, Clone)]
pub enum EngineQueryResult {
    // ── Existing ──────────────────────────────────────────────────────────────
    /// Result of a `SnapshotDiff` query.
    SnapshotDiff(Box<SnapshotDiffResult>),

    // ── State ─────────────────────────────────────────────────────────────────
    StateVersion(StateVersionResult),

    // ── Ettle ─────────────────────────────────────────────────────────────────
    EttleGet(EttleGetResult),
    EttleList(EttlePage),

    // ── Constraint ────────────────────────────────────────────────────────────
    ConstraintGet(ettlex_core::model::Constraint),
    ConstraintListByFamily(Vec<ettlex_core::model::Constraint>),

    // ── Decision ─────────────────────────────────────────────────────────────
    DecisionGet(ettlex_core::model::Decision),
    DecisionList(DecisionPage),
    DecisionListByTarget(Vec<ettlex_core::model::Decision>),
    EttleListDecisions(Vec<ettlex_core::model::Decision>),

    // ── Snapshot / Manifest ───────────────────────────────────────────────────
    SnapshotGet(SnapshotGetResult),
    SnapshotList(Vec<SnapshotGetResult>),
    ManifestGet(ManifestGetResult),

    // ── Profile ──────────────────────────────────────────────────────────────
    ProfileGet(ProfileGetResult),
    ProfileResolve(ProfileResolveResult),
    ProfileList(ProfilePage),

    // ── Approval ─────────────────────────────────────────────────────────────
    ApprovalGet(ApprovalGetResult),
    ApprovalList(ApprovalPage),

    // ── Predicate preview ────────────────────────────────────────────────────
    PredicatePreview(PredicatePreviewResult),

    // ── Policy ───────────────────────────────────────────────────────────────
    /// Result of a `PolicyList` query.
    PolicyList(Vec<ettlex_core::policy_provider::PolicyListEntry>),
    /// Result of a `PolicyRead` query.
    PolicyRead(PolicyReadResult),
    /// Result of a `PolicyExport` query.
    PolicyExport(PolicyExportResult),
    /// Result of a `SnapshotManifestPolicyRef` query.
    SnapshotManifestPolicyRef(String),
    /// Result of a `PolicyProjectForHandoff` query.
    PolicyProjectForHandoff(PolicyProjectForHandoffResult),

    // ── Snapshot head ─────────────────────────────────────────────────────────
    /// Result of a `SnapshotGetHead` query: manifest digest of the head, or None.
    SnapshotGetHead(Option<String>),
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

/// Resolve a `SnapshotRef` to raw manifest bytes.
fn resolve_ref(snapshot_ref: &SnapshotRef, conn: &Connection, cas: &FsStore) -> Result<Vec<u8>> {
    match snapshot_ref {
        SnapshotRef::SnapshotId(id) => {
            let digest = fetch_snapshot_manifest_digest(conn, id)?;
            fetch_manifest_bytes_by_digest(cas, &digest)
        }
        SnapshotRef::ManifestDigest(digest) => fetch_manifest_bytes_by_digest(cas, digest),
    }
}

fn snapshot_row_to_result(row: ettlex_store::snapshot::query::SnapshotRow) -> SnapshotGetResult {
    SnapshotGetResult {
        snapshot_id: row.snapshot_id,
        root_ettle_id: row.root_ettle_id,
        manifest_digest: row.manifest_digest,
        semantic_manifest_digest: row.semantic_manifest_digest,
        created_at: row.created_at,
        parent_snapshot_id: row.parent_snapshot_id,
        policy_ref: row.policy_ref,
        profile_ref: row.profile_ref,
        status: row.status,
    }
}

fn approval_row_to_list_item(row: ApprovalRow) -> ApprovalListItem {
    ApprovalListItem {
        approval_token: row.approval_token,
        reason_code: row.reason_code,
        semantic_request_digest: row.semantic_request_digest,
        status: row.status,
        created_at: row.created_at,
        request_digest: row.request_digest,
    }
}

/// Compute SHA-256 hex digest of bytes.
#[allow(dead_code)]
fn sha256_hex(data: &[u8]) -> String {
    use sha2::Digest;
    let mut h = sha2::Sha256::new();
    h.update(data);
    format!("{:x}", h.finalize())
}

// ---------------------------------------------------------------------------
// apply_engine_query
// ---------------------------------------------------------------------------

/// Apply a read-only engine query.
///
/// All branches use only `&Connection` (shared, non-mutable) and `&FsStore`.
/// Nothing is written to the database, CAS, or ledger.
///
/// The `policy_provider` parameter is required for policy-related queries
/// (`PolicyList`, `PolicyRead`, `PolicyExport`, `SnapshotManifestPolicyRef`).
/// Pass `None` for non-policy queries. Policy queries with `policy_provider = None`
/// return `Err(NotImplemented)`.
///
/// # Errors
///
/// Error kinds depend on the query; see individual variant documentation.
pub fn apply_engine_query(
    query: EngineQuery,
    conn: &Connection,
    cas: &FsStore,
    policy_provider: Option<&dyn ettlex_core::policy_provider::PolicyProvider>,
) -> Result<EngineQueryResult> {
    match query {
        // ── SnapshotDiff ──────────────────────────────────────────────────────
        EngineQuery::SnapshotDiff { a_ref, b_ref } => {
            log_op_start!("snapshot_diff");
            let start = std::time::Instant::now();

            let result = (|| -> Result<EngineQueryResult> {
                let a_bytes = resolve_ref(&a_ref, conn, cas)?;
                let b_bytes = resolve_ref(&b_ref, conn, cas)?;

                let structured_diff = diff::engine::compute_diff(&a_bytes, &b_bytes)?;
                let human_summary = render_human_summary(&structured_diff);

                Ok(EngineQueryResult::SnapshotDiff(Box::new(
                    SnapshotDiffResult {
                        structured_diff,
                        human_summary,
                    },
                )))
            })();

            let elapsed = start.elapsed().as_millis() as u64;
            match &result {
                Ok(_) => log_op_end!("snapshot_diff", duration_ms = elapsed),
                Err(e) => {
                    let e_clone = e.clone();
                    log_op_error!("snapshot_diff", e_clone, duration_ms = elapsed);
                }
            }
            result
        }

        // ── StateGetVersion ───────────────────────────────────────────────────
        EngineQuery::StateGetVersion => {
            log_op_start!("state_get_version");
            let start = std::time::Instant::now();

            let result = (|| -> Result<EngineQueryResult> {
                let version: u64 = conn
                    .query_row("SELECT COUNT(*) FROM command_log", [], |row| row.get(0))
                    .map_err(|e| {
                        ExError::new(ExErrorKind::Persistence)
                            .with_op("state_get_version")
                            .with_message(e.to_string())
                    })?;

                let head_digest: Option<String> = conn
                    .query_row(
                        "SELECT semantic_manifest_digest FROM snapshots
                         ORDER BY created_at DESC, snapshot_id DESC LIMIT 1",
                        [],
                        |row| row.get(0),
                    )
                    .optional()
                    .map_err(|e| {
                        ExError::new(ExErrorKind::Persistence)
                            .with_op("state_get_version")
                            .with_message(e.to_string())
                    })?;

                Ok(EngineQueryResult::StateVersion(StateVersionResult {
                    state_version: version,
                    semantic_head_digest: head_digest,
                }))
            })();

            let elapsed = start.elapsed().as_millis() as u64;
            match &result {
                Ok(_) => log_op_end!("state_get_version", duration_ms = elapsed),
                Err(e) => {
                    let e_clone = e.clone();
                    log_op_error!("state_get_version", e_clone, duration_ms = elapsed);
                }
            }
            result
        }

        // ── EttleGet ──────────────────────────────────────────────────────────
        EngineQuery::EttleGet { ettle_id } => {
            log_op_start!("ettle_get");
            let start = std::time::Instant::now();

            let result = (|| -> Result<EngineQueryResult> {
                let ettle = SqliteRepo::get_ettle(conn, &ettle_id)?.ok_or_else(|| {
                    ExError::new(ExErrorKind::NotFound)
                        .with_op("ettle_get")
                        .with_entity_id(&ettle_id)
                        .with_message("ettle not found")
                })?;
                Ok(EngineQueryResult::EttleGet(EttleGetResult { ettle }))
            })();

            let elapsed = start.elapsed().as_millis() as u64;
            match &result {
                Ok(_) => log_op_end!("ettle_get", duration_ms = elapsed),
                Err(e) => {
                    let e_clone = e.clone();
                    log_op_error!("ettle_get", e_clone, duration_ms = elapsed);
                }
            }
            result
        }

        // ── EttleList ─────────────────────────────────────────────────────────
        EngineQuery::EttleList(opts) => {
            log_op_start!("ettle_list");
            let start = std::time::Instant::now();

            let result = (|| -> Result<EngineQueryResult> {
                let limit = opts.effective_limit();
                let after_id = opts.decode_cursor();
                let raw = SqliteRepo::list_ettles_paginated(
                    conn,
                    opts.prefix_filter.as_deref(),
                    after_id.as_deref(),
                    limit + 1, // over-fetch by 1 to detect has_more
                )?;

                let page =
                    Page::from_overshot(raw, limit, |e: &ettlex_core::model::Ettle| e.id.clone());
                Ok(EngineQueryResult::EttleList(page))
            })();

            let elapsed = start.elapsed().as_millis() as u64;
            match &result {
                Ok(_) => log_op_end!("ettle_list", duration_ms = elapsed),
                Err(e) => {
                    let e_clone = e.clone();
                    log_op_error!("ettle_list", e_clone, duration_ms = elapsed);
                }
            }
            result
        }

        // ── ConstraintGet ─────────────────────────────────────────────────────
        EngineQuery::ConstraintGet { constraint_id } => {
            log_op_start!("constraint_get");
            let start = std::time::Instant::now();
            let result = (|| -> Result<EngineQueryResult> {
                let c = SqliteRepo::get_constraint(conn, &constraint_id)?.ok_or_else(|| {
                    ExError::new(ExErrorKind::NotFound)
                        .with_op("constraint_get")
                        .with_entity_id(&constraint_id)
                        .with_message("constraint not found")
                })?;
                Ok(EngineQueryResult::ConstraintGet(c))
            })();
            let elapsed = start.elapsed().as_millis() as u64;
            match &result {
                Ok(_) => log_op_end!("constraint_get", duration_ms = elapsed),
                Err(e) => {
                    let e_clone = e.clone();
                    log_op_error!("constraint_get", e_clone, duration_ms = elapsed);
                }
            }
            result
        }

        // ── ConstraintListByFamily ────────────────────────────────────────────
        EngineQuery::ConstraintListByFamily {
            family,
            include_tombstoned,
        } => {
            log_op_start!("constraint_list_by_family");
            let start = std::time::Instant::now();
            let result = (|| -> Result<EngineQueryResult> {
                let cs = SqliteRepo::list_constraints_by_family(conn, &family, include_tombstoned)?;
                Ok(EngineQueryResult::ConstraintListByFamily(cs))
            })();
            let elapsed = start.elapsed().as_millis() as u64;
            match &result {
                Ok(_) => log_op_end!("constraint_list_by_family", duration_ms = elapsed),
                Err(e) => {
                    let e_clone = e.clone();
                    log_op_error!("constraint_list_by_family", e_clone, duration_ms = elapsed);
                }
            }
            result
        }

        // ── DecisionGet ───────────────────────────────────────────────────────
        EngineQuery::DecisionGet { decision_id } => {
            log_op_start!("decision_get");
            let start = std::time::Instant::now();
            let result = (|| -> Result<EngineQueryResult> {
                let d = SqliteRepo::get_decision(conn, &decision_id)?.ok_or_else(|| {
                    ExError::new(ExErrorKind::NotFound)
                        .with_op("decision_get")
                        .with_entity_id(&decision_id)
                        .with_message("decision not found")
                })?;
                Ok(EngineQueryResult::DecisionGet(d))
            })();
            let elapsed = start.elapsed().as_millis() as u64;
            match &result {
                Ok(_) => log_op_end!("decision_get", duration_ms = elapsed),
                Err(e) => {
                    let e_clone = e.clone();
                    log_op_error!("decision_get", e_clone, duration_ms = elapsed);
                }
            }
            result
        }

        // ── DecisionList ──────────────────────────────────────────────────────
        EngineQuery::DecisionList(opts) => {
            log_op_start!("decision_list");
            let start = std::time::Instant::now();
            let result = (|| -> Result<EngineQueryResult> {
                let limit = opts.effective_limit();
                let after_key: Option<(i64, String)> = opts.decode_cursor().and_then(|c| {
                    // Cursor format: "ts_ms|decision_id"
                    let parts: Vec<&str> = c.splitn(2, '|').collect();
                    if parts.len() == 2 {
                        parts[0]
                            .parse::<i64>()
                            .ok()
                            .map(|ts| (ts, parts[1].to_string()))
                    } else {
                        None
                    }
                });
                let raw = SqliteRepo::list_decisions_paginated(
                    conn,
                    after_key.as_ref().map(|(ts, id)| (*ts, id.as_str())),
                    limit + 1,
                )?;
                let page = Page::from_overshot(raw, limit, |d: &ettlex_core::model::Decision| {
                    format!("{}|{}", d.created_at.timestamp_millis(), d.decision_id)
                });
                Ok(EngineQueryResult::DecisionList(page))
            })();
            let elapsed = start.elapsed().as_millis() as u64;
            match &result {
                Ok(_) => log_op_end!("decision_list", duration_ms = elapsed),
                Err(e) => {
                    let e_clone = e.clone();
                    log_op_error!("decision_list", e_clone, duration_ms = elapsed);
                }
            }
            result
        }

        // ── DecisionListByTarget ──────────────────────────────────────────────
        EngineQuery::DecisionListByTarget {
            target_kind,
            target_id,
            include_tombstoned,
        } => {
            log_op_start!("decision_list_by_target");
            let start = std::time::Instant::now();
            let result = (|| -> Result<EngineQueryResult> {
                let ds = SqliteRepo::list_decisions_by_target(
                    conn,
                    &target_kind,
                    &target_id,
                    include_tombstoned,
                )?;
                Ok(EngineQueryResult::DecisionListByTarget(ds))
            })();
            let elapsed = start.elapsed().as_millis() as u64;
            match &result {
                Ok(_) => log_op_end!("decision_list_by_target", duration_ms = elapsed),
                Err(e) => {
                    let e_clone = e.clone();
                    log_op_error!("decision_list_by_target", e_clone, duration_ms = elapsed);
                }
            }
            result
        }

        // ── EttleListDecisions ────────────────────────────────────────────────
        EngineQuery::EttleListDecisions {
            ettle_id,
            include_eps: _,
            include_ancestors: _,
        } => {
            log_op_start!("ettle_list_decisions");
            let start = std::time::Instant::now();
            let result = (|| -> Result<EngineQueryResult> {
                let mut all: Vec<ettlex_core::model::Decision> = Vec::new();
                let mut seen = std::collections::BTreeSet::new();

                let mut add = |ds: Vec<ettlex_core::model::Decision>| {
                    for d in ds {
                        if seen.insert(d.decision_id.clone()) {
                            all.push(d);
                        }
                    }
                };

                // Decisions for the ettle itself.
                // include_eps and include_ancestors are no-ops in Slice 03 (EP retired,
                // parent_id removed from Ettle). Only direct ettle decisions are returned.
                add(SqliteRepo::list_decisions_by_target(
                    conn, "ettle", &ettle_id, false,
                )?);

                all.sort_by(|a, b| {
                    a.created_at
                        .cmp(&b.created_at)
                        .then(a.decision_id.cmp(&b.decision_id))
                });
                Ok(EngineQueryResult::EttleListDecisions(all))
            })();
            let elapsed = start.elapsed().as_millis() as u64;
            match &result {
                Ok(_) => log_op_end!("ettle_list_decisions", duration_ms = elapsed),
                Err(e) => {
                    let e_clone = e.clone();
                    log_op_error!("ettle_list_decisions", e_clone, duration_ms = elapsed);
                }
            }
            result
        }

        // ── SnapshotGet ───────────────────────────────────────────────────────
        EngineQuery::SnapshotGet { snapshot_id } => {
            log_op_start!("snapshot_get");
            let start = std::time::Instant::now();
            let result = (|| -> Result<EngineQueryResult> {
                let row = fetch_snapshot_row(conn, &snapshot_id)?;
                Ok(EngineQueryResult::SnapshotGet(snapshot_row_to_result(row)))
            })();
            let elapsed = start.elapsed().as_millis() as u64;
            match &result {
                Ok(_) => log_op_end!("snapshot_get", duration_ms = elapsed),
                Err(e) => {
                    let e_clone = e.clone();
                    log_op_error!("snapshot_get", e_clone, duration_ms = elapsed);
                }
            }
            result
        }

        // ── SnapshotList ──────────────────────────────────────────────────────
        EngineQuery::SnapshotList { ettle_id } => {
            log_op_start!("snapshot_list");
            let start = std::time::Instant::now();
            let result = (|| -> Result<EngineQueryResult> {
                let rows = list_snapshot_rows(conn, ettle_id.as_deref())?;
                let results = rows.into_iter().map(snapshot_row_to_result).collect();
                Ok(EngineQueryResult::SnapshotList(results))
            })();
            let elapsed = start.elapsed().as_millis() as u64;
            match &result {
                Ok(_) => log_op_end!("snapshot_list", duration_ms = elapsed),
                Err(e) => {
                    let e_clone = e.clone();
                    log_op_error!("snapshot_list", e_clone, duration_ms = elapsed);
                }
            }
            result
        }

        // ── ManifestGetBySnapshot ─────────────────────────────────────────────
        EngineQuery::ManifestGetBySnapshot { snapshot_id } => {
            log_op_start!("manifest_get_by_snapshot");
            let start = std::time::Instant::now();
            let result = (|| -> Result<EngineQueryResult> {
                let row = fetch_snapshot_row(conn, &snapshot_id)?;
                let bytes = fetch_manifest_bytes_by_digest(cas, &row.manifest_digest)?;
                Ok(EngineQueryResult::ManifestGet(ManifestGetResult {
                    snapshot_id: row.snapshot_id,
                    manifest_digest: row.manifest_digest,
                    semantic_manifest_digest: row.semantic_manifest_digest,
                    manifest_bytes: bytes,
                }))
            })();
            let elapsed = start.elapsed().as_millis() as u64;
            match &result {
                Ok(_) => log_op_end!("manifest_get_by_snapshot", duration_ms = elapsed),
                Err(e) => {
                    let e_clone = e.clone();
                    log_op_error!("manifest_get_by_snapshot", e_clone, duration_ms = elapsed);
                }
            }
            result
        }

        // ── ManifestGetByDigest ───────────────────────────────────────────────
        EngineQuery::ManifestGetByDigest { manifest_digest } => {
            log_op_start!("manifest_get_by_digest");
            let start = std::time::Instant::now();
            let result = (|| -> Result<EngineQueryResult> {
                // Lookup snapshot row that has this manifest_digest
                let row: Option<(String, String, String)> = conn
                    .query_row(
                        "SELECT snapshot_id, manifest_digest, semantic_manifest_digest
                         FROM snapshots WHERE manifest_digest = ?1 LIMIT 1",
                        [&manifest_digest],
                        |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
                    )
                    .optional()
                    .map_err(|e| {
                        ExError::new(ExErrorKind::Persistence)
                            .with_op("manifest_get_by_digest")
                            .with_message(e.to_string())
                    })?;

                match row {
                    None => {
                        // Digest not in snapshots table — try CAS directly
                        let bytes = fetch_manifest_bytes_by_digest(cas, &manifest_digest)?;
                        Ok(EngineQueryResult::ManifestGet(ManifestGetResult {
                            snapshot_id: String::new(),
                            manifest_digest: manifest_digest.clone(),
                            semantic_manifest_digest: String::new(),
                            manifest_bytes: bytes,
                        }))
                    }
                    Some((sid, md, smd)) => {
                        let bytes = fetch_manifest_bytes_by_digest(cas, &md)?;
                        Ok(EngineQueryResult::ManifestGet(ManifestGetResult {
                            snapshot_id: sid,
                            manifest_digest: md,
                            semantic_manifest_digest: smd,
                            manifest_bytes: bytes,
                        }))
                    }
                }
            })();
            let elapsed = start.elapsed().as_millis() as u64;
            match &result {
                Ok(_) => log_op_end!("manifest_get_by_digest", duration_ms = elapsed),
                Err(e) => {
                    let e_clone = e.clone();
                    log_op_error!("manifest_get_by_digest", e_clone, duration_ms = elapsed);
                }
            }
            result
        }

        // ── ProfileGet ────────────────────────────────────────────────────────
        EngineQuery::ProfileGet { profile_ref } => {
            log_op_start!("profile_get");
            let start = std::time::Instant::now();
            let result = (|| -> Result<EngineQueryResult> {
                match load_profile_full(conn, &profile_ref)? {
                    None => Err(ExError::new(ExErrorKind::ProfileNotFound)
                        .with_op("profile_get")
                        .with_entity_id(&profile_ref)
                        .with_message("profile not found")),
                    Some((pref, digest, payload)) => {
                        Ok(EngineQueryResult::ProfileGet(ProfileGetResult {
                            profile_ref: pref,
                            profile_digest: digest,
                            payload_json: payload,
                        }))
                    }
                }
            })();
            let elapsed = start.elapsed().as_millis() as u64;
            match &result {
                Ok(_) => log_op_end!("profile_get", duration_ms = elapsed),
                Err(e) => {
                    let e_clone = e.clone();
                    log_op_error!("profile_get", e_clone, duration_ms = elapsed);
                }
            }
            result
        }

        // ── ProfileResolve ────────────────────────────────────────────────────
        EngineQuery::ProfileResolve { profile_ref } => {
            log_op_start!("profile_resolve");
            let start = std::time::Instant::now();
            let result = (|| -> Result<EngineQueryResult> {
                let found = if let Some(ref pref) = profile_ref {
                    load_profile_full(conn, pref)?.map(|t| (pref.clone(), t.1, t.2))
                } else {
                    load_default_profile(conn)?
                };

                match found {
                    None => {
                        if profile_ref.is_some() {
                            Err(ExError::new(ExErrorKind::ProfileNotFound)
                                .with_op("profile_resolve")
                                .with_message("profile not found"))
                        } else {
                            Err(ExError::new(ExErrorKind::ProfileDefaultMissing)
                                .with_op("profile_resolve")
                                .with_message("no default profile found"))
                        }
                    }
                    Some((pref, digest, payload)) => {
                        Ok(EngineQueryResult::ProfileResolve(ProfileResolveResult {
                            profile_ref: pref,
                            profile_digest: digest,
                            parsed_profile: payload,
                        }))
                    }
                }
            })();
            let elapsed = start.elapsed().as_millis() as u64;
            match &result {
                Ok(_) => log_op_end!("profile_resolve", duration_ms = elapsed),
                Err(e) => {
                    let e_clone = e.clone();
                    log_op_error!("profile_resolve", e_clone, duration_ms = elapsed);
                }
            }
            result
        }

        // ── ProfileGetDefault ─────────────────────────────────────────────────
        EngineQuery::ProfileGetDefault => {
            log_op_start!("profile_get_default");
            let start = std::time::Instant::now();
            let result = (|| -> Result<EngineQueryResult> {
                match load_default_profile(conn)? {
                    None => Err(ExError::new(ExErrorKind::ProfileDefaultMissing)
                        .with_op("profile_get_default")
                        .with_message("no default profile found")),
                    Some((pref, digest, payload)) => {
                        Ok(EngineQueryResult::ProfileGet(ProfileGetResult {
                            profile_ref: pref,
                            profile_digest: digest,
                            payload_json: payload,
                        }))
                    }
                }
            })();
            let elapsed = start.elapsed().as_millis() as u64;
            match &result {
                Ok(_) => log_op_end!("profile_get_default", duration_ms = elapsed),
                Err(e) => {
                    let e_clone = e.clone();
                    log_op_error!("profile_get_default", e_clone, duration_ms = elapsed);
                }
            }
            result
        }

        // ── ProfileList ───────────────────────────────────────────────────────
        EngineQuery::ProfileList(opts) => {
            log_op_start!("profile_list");
            let start = std::time::Instant::now();
            let result = (|| -> Result<EngineQueryResult> {
                let limit = opts.effective_limit();
                let after_ref = opts.decode_cursor();
                let raw = list_profiles_paginated(conn, after_ref.as_deref(), limit + 1)?;
                let as_results: Vec<ProfileGetResult> = raw
                    .into_iter()
                    .map(|(pref, digest, payload)| ProfileGetResult {
                        profile_ref: pref,
                        profile_digest: digest,
                        payload_json: payload,
                    })
                    .collect();
                let page = Page::from_overshot(as_results, limit, |p: &ProfileGetResult| {
                    p.profile_ref.clone()
                });
                Ok(EngineQueryResult::ProfileList(page))
            })();
            let elapsed = start.elapsed().as_millis() as u64;
            match &result {
                Ok(_) => log_op_end!("profile_list", duration_ms = elapsed),
                Err(e) => {
                    let e_clone = e.clone();
                    log_op_error!("profile_list", e_clone, duration_ms = elapsed);
                }
            }
            result
        }

        // ── ApprovalGet ───────────────────────────────────────────────────────
        EngineQuery::ApprovalGet { approval_token } => {
            log_op_start!("approval_get");
            let start = std::time::Instant::now();
            let result = (|| -> Result<EngineQueryResult> {
                let row = fetch_approval_row(conn, &approval_token)?.ok_or_else(|| {
                    ExError::new(ExErrorKind::ApprovalNotFound)
                        .with_op("approval_get")
                        .with_entity_id(&approval_token)
                        .with_message("approval request not found")
                })?;

                let request_digest = row.request_digest.clone().ok_or_else(|| {
                    ExError::new(ExErrorKind::ApprovalStorageCorrupt)
                        .with_op("approval_get")
                        .with_entity_id(&approval_token)
                        .with_message("approval row has no request_digest (migration 007 not applied or CAS write failed)")
                })?;

                let blob_bytes = cas.read(&request_digest).map_err(|e| {
                    if e.kind() == ExErrorKind::NotFound {
                        ExError::new(ExErrorKind::ApprovalStorageCorrupt)
                            .with_op("approval_get")
                            .with_entity_id(&approval_token)
                            .with_message(format!(
                                "CAS blob missing for request_digest {}",
                                request_digest
                            ))
                    } else {
                        e
                    }
                })?;

                let payload_json: serde_json::Value =
                    serde_json::from_slice(&blob_bytes).map_err(|e| {
                        ExError::new(ExErrorKind::ApprovalStorageCorrupt)
                            .with_op("approval_get")
                            .with_entity_id(&approval_token)
                            .with_message(format!("CAS blob is not valid JSON: {}", e))
                    })?;

                Ok(EngineQueryResult::ApprovalGet(ApprovalGetResult {
                    approval_token,
                    request_digest,
                    semantic_request_digest: row.semantic_request_digest,
                    payload_json,
                }))
            })();
            let elapsed = start.elapsed().as_millis() as u64;
            match &result {
                Ok(_) => log_op_end!("approval_get", duration_ms = elapsed),
                Err(e) => {
                    let e_clone = e.clone();
                    log_op_error!("approval_get", e_clone, duration_ms = elapsed);
                }
            }
            result
        }

        // ── ApprovalList ──────────────────────────────────────────────────────
        EngineQuery::ApprovalList(opts) => {
            log_op_start!("approval_list");
            let start = std::time::Instant::now();
            let result = (|| -> Result<EngineQueryResult> {
                let limit = opts.effective_limit();
                let after_key: Option<(i64, String)> = opts.decode_cursor().and_then(|c| {
                    let parts: Vec<&str> = c.splitn(2, '|').collect();
                    if parts.len() == 2 {
                        parts[0]
                            .parse::<i64>()
                            .ok()
                            .map(|ts| (ts, parts[1].to_string()))
                    } else {
                        None
                    }
                });
                let raw = list_approval_rows_paginated(
                    conn,
                    after_key.as_ref().map(|(ts, tok)| (*ts, tok.as_str())),
                    limit + 1,
                )?;
                let items: Vec<ApprovalListItem> =
                    raw.into_iter().map(approval_row_to_list_item).collect();
                let page = Page::from_overshot(items, limit, |item: &ApprovalListItem| {
                    format!("{}|{}", item.created_at, item.approval_token)
                });
                Ok(EngineQueryResult::ApprovalList(page))
            })();
            let elapsed = start.elapsed().as_millis() as u64;
            match &result {
                Ok(_) => log_op_end!("approval_list", duration_ms = elapsed),
                Err(e) => {
                    let e_clone = e.clone();
                    log_op_error!("approval_list", e_clone, duration_ms = elapsed);
                }
            }
            result
        }

        // ── ApprovalListByKind ────────────────────────────────────────────────
        EngineQuery::ApprovalListByKind {
            kind: _,
            options: _,
        } => Err(ExError::new(ExErrorKind::NotImplemented)
            .with_op("approval_list_by_kind")
            .with_message("ApprovalListByKind is not implemented in Phase 1")),

        // ── ConstraintPredicatesPreview ───────────────────────────────────────
        EngineQuery::ConstraintPredicatesPreview {
            profile_ref,
            context: _,
            candidates,
        } => {
            log_op_start!("constraint_predicates_preview");
            let start = std::time::Instant::now();
            let result = (|| -> Result<EngineQueryResult> {
                // Resolve ambiguity policy from profile (read-only)
                let ambiguity_policy = resolve_ambiguity_policy(conn, profile_ref.as_deref())?;

                // Build candidate entries (priority 0 for all — Phase 1 has no predicate eval)
                let candidate_entries: Vec<CandidateEntry> = candidates
                    .iter()
                    .enumerate()
                    .map(|(i, id)| CandidateEntry {
                        candidate_id: id.clone(),
                        priority: i as i64,
                    })
                    .collect();

                let resolution = compute_dry_run_resolution(&candidate_entries, &ambiguity_policy);

                let (status, selected) = match resolution.status {
                    DryRunConstraintStatus::Uncomputed => (PreviewStatus::NoMatch, None),
                    DryRunConstraintStatus::Resolved => {
                        if resolution.selected_profile_ref.is_none() && candidates.is_empty() {
                            (PreviewStatus::NoMatch, None)
                        } else {
                            (PreviewStatus::Selected, resolution.selected_profile_ref)
                        }
                    }
                    DryRunConstraintStatus::RoutedForApproval => match ambiguity_policy {
                        AmbiguityPolicy::RouteForApproval => {
                            (PreviewStatus::RoutedForApproval, None)
                        }
                        _ => (PreviewStatus::Ambiguous, None),
                    },
                };

                Ok(EngineQueryResult::PredicatePreview(
                    PredicatePreviewResult {
                        status,
                        selected,
                        candidates: resolution.candidates,
                    },
                ))
            })();
            let elapsed = start.elapsed().as_millis() as u64;
            match &result {
                Ok(_) => log_op_end!("constraint_predicates_preview", duration_ms = elapsed),
                Err(e) => {
                    let e_clone = e.clone();
                    log_op_error!(
                        "constraint_predicates_preview",
                        e_clone,
                        duration_ms = elapsed
                    );
                }
            }
            result
        }

        // ── PolicyList ────────────────────────────────────────────────────────
        EngineQuery::PolicyList => {
            log_op_start!("policy_list");
            let start = std::time::Instant::now();
            let result = (|| -> Result<EngineQueryResult> {
                let provider = policy_provider.ok_or_else(|| {
                    ExError::new(ExErrorKind::NotImplemented)
                        .with_op("policy_list")
                        .with_message("policy_provider is required for PolicyList")
                })?;
                let entries = provider.policy_list()?;
                Ok(EngineQueryResult::PolicyList(entries))
            })();
            let elapsed = start.elapsed().as_millis() as u64;
            match &result {
                Ok(_) => log_op_end!("policy_list", duration_ms = elapsed),
                Err(e) => {
                    let e_clone = e.clone();
                    log_op_error!("policy_list", e_clone, duration_ms = elapsed);
                }
            }
            result
        }

        // ── PolicyRead ────────────────────────────────────────────────────────
        EngineQuery::PolicyRead { policy_ref } => {
            log_op_start!("policy_read");
            let start = std::time::Instant::now();
            let result = (|| -> Result<EngineQueryResult> {
                let provider = policy_provider.ok_or_else(|| {
                    ExError::new(ExErrorKind::NotImplemented)
                        .with_op("policy_read")
                        .with_message("policy_provider is required for PolicyRead")
                })?;
                let text = provider.policy_read(&policy_ref)?;
                Ok(EngineQueryResult::PolicyRead(PolicyReadResult {
                    policy_ref,
                    text,
                }))
            })();
            let elapsed = start.elapsed().as_millis() as u64;
            match &result {
                Ok(_) => log_op_end!("policy_read", duration_ms = elapsed),
                Err(e) => {
                    let e_clone = e.clone();
                    log_op_error!("policy_read", e_clone, duration_ms = elapsed);
                }
            }
            result
        }

        // ── PolicyExport ──────────────────────────────────────────────────────
        EngineQuery::PolicyExport {
            policy_ref,
            export_kind,
        } => {
            log_op_start!("policy_export");
            let start = std::time::Instant::now();
            let result = (|| -> Result<EngineQueryResult> {
                let provider = policy_provider.ok_or_else(|| {
                    ExError::new(ExErrorKind::NotImplemented)
                        .with_op("policy_export")
                        .with_message("policy_provider is required for PolicyExport")
                })?;
                let text = provider.policy_export(&policy_ref, &export_kind)?;
                Ok(EngineQueryResult::PolicyExport(PolicyExportResult {
                    policy_ref,
                    export_kind,
                    text,
                }))
            })();
            let elapsed = start.elapsed().as_millis() as u64;
            match &result {
                Ok(_) => log_op_end!("policy_export", duration_ms = elapsed),
                Err(e) => {
                    let e_clone = e.clone();
                    log_op_error!("policy_export", e_clone, duration_ms = elapsed);
                }
            }
            result
        }

        // ── PolicyProjectForHandoff ───────────────────────────────────────────
        EngineQuery::PolicyProjectForHandoff {
            policy_ref,
            profile_ref,
        } => {
            log_op_start!("policy_project_for_handoff");
            let start = std::time::Instant::now();
            let result = (|| -> Result<EngineQueryResult> {
                let provider = policy_provider.ok_or_else(|| {
                    ExError::new(ExErrorKind::NotImplemented)
                        .with_op("policy_project_for_handoff")
                        .with_message("policy_provider is required for PolicyProjectForHandoff")
                })?;

                // Validate profile_ref if provided — must exist in the store
                if let Some(ref pref) = profile_ref {
                    load_profile_full(conn, pref)?.ok_or_else(|| {
                        ExError::new(ExErrorKind::ProfileNotFound)
                            .with_op("policy_project_for_handoff")
                            .with_entity_id(pref)
                            .with_message("profile not found")
                    })?;
                }

                let projection_bytes =
                    provider.policy_project_for_handoff(&policy_ref, profile_ref.as_deref())?;

                Ok(EngineQueryResult::PolicyProjectForHandoff(
                    PolicyProjectForHandoffResult {
                        policy_ref,
                        profile_ref,
                        projection_bytes,
                    },
                ))
            })();
            let elapsed = start.elapsed().as_millis() as u64;
            match &result {
                Ok(_) => log_op_end!("policy_project_for_handoff", duration_ms = elapsed),
                Err(e) => {
                    let e_clone = e.clone();
                    log_op_error!("policy_project_for_handoff", e_clone, duration_ms = elapsed);
                }
            }
            result
        }

        // ── SnapshotManifestPolicyRef ─────────────────────────────────────────
        EngineQuery::SnapshotManifestPolicyRef { manifest_digest } => {
            log_op_start!("snapshot_manifest_policy_ref");
            let start = std::time::Instant::now();
            let result = (|| -> Result<EngineQueryResult> {
                let manifest_bytes = fetch_manifest_bytes_by_digest(cas, &manifest_digest)?;
                let manifest_json: serde_json::Value = serde_json::from_slice(&manifest_bytes)
                    .map_err(|e| {
                        ExError::new(ExErrorKind::InvalidManifest)
                            .with_op("snapshot_manifest_policy_ref")
                            .with_message(format!("Failed to parse manifest JSON: {}", e))
                    })?;
                let policy_ref = manifest_json
                    .get("policy_ref")
                    .and_then(|v| v.as_str())
                    .ok_or_else(|| {
                        ExError::new(ExErrorKind::MissingField)
                            .with_op("snapshot_manifest_policy_ref")
                            .with_message("Manifest missing 'policy_ref' field")
                    })?
                    .to_string();
                Ok(EngineQueryResult::SnapshotManifestPolicyRef(policy_ref))
            })();
            let elapsed = start.elapsed().as_millis() as u64;
            match &result {
                Ok(_) => log_op_end!("snapshot_manifest_policy_ref", duration_ms = elapsed),
                Err(e) => {
                    let e_clone = e.clone();
                    log_op_error!(
                        "snapshot_manifest_policy_ref",
                        e_clone,
                        duration_ms = elapsed
                    );
                }
            }
            result
        }

        // ── SnapshotGetHead ──────────────────────────────────────────────────
        EngineQuery::SnapshotGetHead { realised_ettle_id } => {
            let start = std::time::Instant::now();
            log_op_start!("snapshot_get_head", entity_id = %realised_ettle_id);
            let result: Result<EngineQueryResult> = (|| {
                let digest: Option<String> = conn
                    .query_row(
                        "SELECT manifest_digest FROM snapshots
                         WHERE root_ettle_id = ?1 AND status = 'committed'
                         ORDER BY created_at DESC LIMIT 1",
                        [&realised_ettle_id],
                        |row| row.get(0),
                    )
                    .optional()
                    .map_err(|e| {
                        ExError::new(ExErrorKind::Persistence)
                            .with_op("snapshot_get_head")
                            .with_entity_id(&realised_ettle_id)
                            .with_message(format!("DB error: {}", e))
                    })?;
                Ok(EngineQueryResult::SnapshotGetHead(digest))
            })();
            let elapsed = start.elapsed().as_millis() as u64;
            match &result {
                Ok(_) => log_op_end!("snapshot_get_head", duration_ms = elapsed),
                Err(e) => {
                    let e_clone = e.clone();
                    log_op_error!("snapshot_get_head", e_clone, duration_ms = elapsed);
                }
            }
            result
        }
    }
}

// ---------------------------------------------------------------------------
// Internal query helpers
// ---------------------------------------------------------------------------

fn resolve_ambiguity_policy(
    conn: &Connection,
    profile_ref: Option<&str>,
) -> Result<AmbiguityPolicy> {
    use ettlex_store::profile::load_profile_payload;

    let effective_ref = profile_ref.unwrap_or("profile/default@0");
    match load_profile_payload(conn, effective_ref)? {
        None => Ok(AmbiguityPolicy::FailFast),
        Some(payload) => {
            let policy_str = payload
                .get("ambiguity_policy")
                .and_then(|v| v.as_str())
                .unwrap_or("fail_fast");
            Ok(AmbiguityPolicy::parse(policy_str))
        }
    }
}

// Allow optional extension (needed for inline query_row calls)
use rusqlite::OptionalExtension;
