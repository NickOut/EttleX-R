//! Approval routing interface for commit policy ambiguity resolution.

use crate::errors::{ExError, ExErrorKind};

/// Route an ambiguous commit for human/workflow approval, returning an approval token.
#[allow(clippy::result_large_err)]
pub trait ApprovalRouter: Send + Sync {
    /// Route an approval request.
    ///
    /// # Errors
    ///
    /// Returns `ExErrorKind::ApprovalRoutingUnavailable` if no router is configured,
    /// or `ExErrorKind::Persistence` if the router fails to persist the request.
    fn route_approval_request(
        &self,
        reason_code: &str,
        candidate_set: Vec<String>,
    ) -> Result<String, ExError>;
}

/// Noop router: always returns `ApprovalRoutingUnavailable`.
/// Used as default when no router is configured.
pub struct NoopApprovalRouter;

impl ApprovalRouter for NoopApprovalRouter {
    #[allow(clippy::result_large_err)]
    fn route_approval_request(&self, _: &str, _: Vec<String>) -> Result<String, ExError> {
        Err(ExError::new(ExErrorKind::ApprovalRoutingUnavailable)
            .with_message("No approval router configured"))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_noop_approval_router_returns_unavailable() {
        use crate::errors::ExErrorKind;
        let router = NoopApprovalRouter;
        let result = router.route_approval_request("AmbiguousSelection", vec!["A".to_string()]);
        assert!(result.is_err());
        assert_eq!(
            result.unwrap_err().kind(),
            ExErrorKind::ApprovalRoutingUnavailable
        );
    }
}
