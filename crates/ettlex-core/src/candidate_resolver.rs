//! Constraint candidate resolution.
//!
//! Resolves a list of candidate constraints to a single selection using
//! the configured ambiguity policy.

use crate::approval_router::ApprovalRouter;
use crate::errors::{ExError, ExErrorKind};

/// Ambiguity policy for constraint candidate selection.
#[derive(Debug, Clone, PartialEq)]
pub enum AmbiguityPolicy {
    /// Fail immediately if more than one candidate matches.
    FailFast,
    /// Choose the lexicographically first candidate deterministically.
    ChooseDeterministic,
    /// Route to an approval workflow.
    RouteForApproval,
}

impl AmbiguityPolicy {
    /// Parse from string; unknown values default to `FailFast`.
    pub fn parse(s: &str) -> Self {
        match s {
            "choose_deterministic" => AmbiguityPolicy::ChooseDeterministic,
            "route_for_approval" => AmbiguityPolicy::RouteForApproval,
            _ => AmbiguityPolicy::FailFast,
        }
    }
}

/// A constraint candidate for resolution.
#[derive(Debug, Clone)]
pub struct CandidateEntry {
    pub candidate_id: String,
    pub priority: i64,
}

/// Result of candidate resolution.
#[derive(Debug, Clone)]
pub enum ResolveResult {
    /// No candidates — commit proceeds without constraint selection.
    Empty,
    /// Exactly one candidate selected.
    Selected(String),
    /// Ambiguity routed for approval; token returned.
    PendingApproval(String),
}

/// Resolve candidates using the given ambiguity policy.
///
/// Phase 1: all predicates are always true (no predicate evaluation).
/// Multiple candidates are always ambiguous.
///
/// # Errors
///
/// - `ExErrorKind::AmbiguousSelection` if `FailFast` policy and multiple candidates.
/// - `ExErrorKind::ApprovalRoutingUnavailable` if router is unavailable.
#[allow(clippy::result_large_err)]
pub fn resolve_candidates(
    candidates: &[CandidateEntry],
    policy: &AmbiguityPolicy,
    router: &dyn ApprovalRouter,
) -> Result<ResolveResult, ExError> {
    match candidates.len() {
        0 => Ok(ResolveResult::Empty),
        1 => Ok(ResolveResult::Selected(candidates[0].candidate_id.clone())),
        _ => match policy {
            AmbiguityPolicy::FailFast => Err(ExError::new(ExErrorKind::AmbiguousSelection)
                .with_message(format!(
                    "Ambiguous constraint selection: {} candidates",
                    candidates.len()
                ))),
            AmbiguityPolicy::ChooseDeterministic => {
                // Lexicographic selection is deterministic.
                let mut ids: Vec<&str> =
                    candidates.iter().map(|c| c.candidate_id.as_str()).collect();
                ids.sort_unstable();
                Ok(ResolveResult::Selected(ids[0].to_string()))
            }
            AmbiguityPolicy::RouteForApproval => {
                let candidate_ids: Vec<String> =
                    candidates.iter().map(|c| c.candidate_id.clone()).collect();
                let token = router.route_approval_request("AmbiguousSelection", candidate_ids)?;
                Ok(ResolveResult::PendingApproval(token))
            }
        },
    }
}

/// Status of a dry-run constraint resolution.
#[derive(Debug, Clone, PartialEq)]
pub enum DryRunConstraintStatus {
    /// Profile has predicate evaluation disabled; resolution was not attempted.
    Uncomputed,
    /// Resolution completed (or no constraints exist).
    Resolved,
    /// Multiple candidates and policy would route for approval.
    RoutedForApproval,
}

/// Result of a dry-run constraint resolution — computed without side effects.
#[derive(Debug, Clone)]
pub struct DryRunConstraintResolution {
    /// Resolution status.
    pub status: DryRunConstraintStatus,
    /// Winning candidate ID when `status == Resolved` and at least one candidate exists.
    pub selected_profile_ref: Option<String>,
    /// All candidate IDs sorted lexicographically; empty for `Uncomputed`.
    pub candidates: Vec<String>,
}

/// Compute what constraint resolution *would* produce without any side effects.
///
/// Does NOT call `ApprovalRouter`, does NOT write anything.
/// Use this in `dry_run` mode to populate `SnapshotCommitResult::constraint_resolution`.
pub fn compute_dry_run_resolution(
    candidates: &[CandidateEntry],
    policy: &AmbiguityPolicy,
) -> DryRunConstraintResolution {
    match candidates.len() {
        0 => DryRunConstraintResolution {
            status: DryRunConstraintStatus::Resolved,
            selected_profile_ref: None,
            candidates: vec![],
        },
        1 => DryRunConstraintResolution {
            status: DryRunConstraintStatus::Resolved,
            selected_profile_ref: Some(candidates[0].candidate_id.clone()),
            candidates: vec![candidates[0].candidate_id.clone()],
        },
        _ => {
            let mut sorted: Vec<String> =
                candidates.iter().map(|c| c.candidate_id.clone()).collect();
            sorted.sort();
            match policy {
                AmbiguityPolicy::ChooseDeterministic => DryRunConstraintResolution {
                    status: DryRunConstraintStatus::Resolved,
                    selected_profile_ref: Some(sorted[0].clone()),
                    candidates: sorted,
                },
                AmbiguityPolicy::RouteForApproval | AmbiguityPolicy::FailFast => {
                    DryRunConstraintResolution {
                        status: DryRunConstraintStatus::RoutedForApproval,
                        selected_profile_ref: None,
                        candidates: sorted,
                    }
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::approval_router::NoopApprovalRouter;

    fn cands(ids: &[&str]) -> Vec<CandidateEntry> {
        ids.iter()
            .enumerate()
            .map(|(i, id)| CandidateEntry {
                candidate_id: id.to_string(),
                priority: i as i64,
            })
            .collect()
    }

    #[test]
    fn test_resolve_empty_returns_empty() {
        let r = resolve_candidates(&[], &AmbiguityPolicy::FailFast, &NoopApprovalRouter).unwrap();
        assert!(matches!(r, ResolveResult::Empty));
    }

    #[test]
    fn test_resolve_single_returns_selected() {
        let r = resolve_candidates(
            &cands(&["c:A"]),
            &AmbiguityPolicy::FailFast,
            &NoopApprovalRouter,
        )
        .unwrap();
        assert!(matches!(r, ResolveResult::Selected(id) if id == "c:A"));
    }

    #[test]
    fn test_resolve_multiple_fail_fast_errors() {
        let r = resolve_candidates(
            &cands(&["c:A", "c:B"]),
            &AmbiguityPolicy::FailFast,
            &NoopApprovalRouter,
        );
        assert!(r.is_err());
        assert_eq!(r.unwrap_err().kind(), ExErrorKind::AmbiguousSelection);
    }

    #[test]
    fn test_resolve_multiple_choose_deterministic_picks_first_lex() {
        let r = resolve_candidates(
            &cands(&["c:B", "c:A"]),
            &AmbiguityPolicy::ChooseDeterministic,
            &NoopApprovalRouter,
        )
        .unwrap();
        assert!(matches!(r, ResolveResult::Selected(id) if id == "c:A"));
    }

    #[test]
    fn test_resolve_route_for_approval_noop_returns_unavailable() {
        let r = resolve_candidates(
            &cands(&["c:A", "c:B"]),
            &AmbiguityPolicy::RouteForApproval,
            &NoopApprovalRouter,
        );
        assert!(r.is_err());
        assert_eq!(
            r.unwrap_err().kind(),
            ExErrorKind::ApprovalRoutingUnavailable
        );
    }
}
