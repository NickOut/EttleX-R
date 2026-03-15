//! Error facility re-export
//!
//! All error types are now defined in `ettlex-errors` and re-exported here
//! for backward compatibility within the crate.

pub use ettlex_errors::{ExError, ExErrorKind, Result};

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_commit_policy_error_kind_codes() {
        let cases = [
            (ExErrorKind::HeadMismatch, "ERR_HEAD_MISMATCH"),
            (ExErrorKind::NotALeaf, "ERR_NOT_A_LEAF"),
            (ExErrorKind::PolicyDenied, "ERR_POLICY_DENIED"),
            (ExErrorKind::RootEttleAmbiguous, "ERR_ROOT_ETTLE_AMBIGUOUS"),
            (ExErrorKind::EptAmbiguous, "ERR_EPT_AMBIGUOUS"),
            (ExErrorKind::ProfileNotFound, "ERR_PROFILE_NOT_FOUND"),
            (
                ExErrorKind::ApprovalRoutingUnavailable,
                "ERR_APPROVAL_ROUTING_UNAVAILABLE",
            ),
        ];
        for (kind, expected_code) in cases {
            assert_eq!(kind.code(), expected_code, "Wrong code for {:?}", kind);
        }
    }

    // S8: RootEttleInvalid has the correct error code
    #[test]
    fn test_root_ettle_invalid_error_code() {
        assert_eq!(
            ExErrorKind::RootEttleInvalid.code(),
            "ERR_ROOT_ETTLE_INVALID"
        );
    }

    // S7: ExError carries a structured candidates field
    #[test]
    fn test_ex_error_candidates_field() {
        let err = ExError::new(ExErrorKind::RootEttleAmbiguous)
            .with_candidates(vec!["ep:a".into(), "ep:b".into()]);
        let candidates = err.candidates().expect("candidates should be Some");
        assert_eq!(candidates, &["ep:a".to_string(), "ep:b".to_string()]);
    }

    #[test]
    fn test_ex_error_candidates_none_by_default() {
        let err = ExError::new(ExErrorKind::NotFound);
        assert!(err.candidates().is_none());
    }

    // TDD Cycle 1 — Policy ExErrorKind codes (S8, S9, S13, S14, S15)
    #[test]
    fn test_policy_error_kind_codes() {
        assert_eq!(ExErrorKind::PolicyNotFound.code(), "ERR_POLICY_NOT_FOUND");
        assert_eq!(
            ExErrorKind::PolicyExportFailed.code(),
            "ERR_POLICY_EXPORT_FAILED"
        );
        assert_eq!(
            ExErrorKind::PolicyRefMissing.code(),
            "ERR_POLICY_REF_MISSING"
        );
        assert_eq!(
            ExErrorKind::PolicyExportTooLarge.code(),
            "ERR_POLICY_EXPORT_TOO_LARGE"
        );
        assert_eq!(
            ExErrorKind::PolicyParseError.code(),
            "ERR_POLICY_PARSE_ERROR"
        );
    }
}
