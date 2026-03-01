//! Backend-agnostic policy provider abstraction.
//!
//! The `PolicyProvider` trait supersedes `CommitPolicyHook` as the engine's
//! indirection layer for policy operations. Unlike the narrow allow/deny hook,
//! `PolicyProvider` also exposes policy discovery (`policy_list`), full text
//! retrieval (`policy_read`), and structured export (`policy_export`).
//!
//! ## Implementations
//!
//! - [`NoopPolicyProvider`]: always allows; policy read/export return `PolicyNotFound`.
//!   Used in the CLI default path and tests that don't exercise policy denial.
//! - [`DenyAllPolicyProvider`]: always denies with `PolicyDenied`.
//!   Used in tests that verify policy gating stops all writes.

use crate::errors::{ExError, ExErrorKind};
use std::result::Result;

/// A single policy discovered by the provider.
#[derive(Debug, Clone)]
pub struct PolicyListEntry {
    /// Stable policy reference identifier (e.g. `codegen_handoff_policy_v1`).
    pub policy_ref: String,
    /// Version string for this policy (e.g. `"0"`).
    pub version: String,
}

/// Backend-agnostic interface for policy operations.
///
/// The engine depends on this trait, not on any concrete backend (file system,
/// database, remote service). Implementations supply policy check, discovery,
/// read, and export capabilities.
///
/// All methods return [`ExError`] on failure. The common error kinds are:
/// - `PolicyDenied` — policy check rejected the operation
/// - `PolicyNotFound` — the `policy_ref` does not exist in this provider
/// - `PolicyExportFailed` — malformed markers or unknown `export_kind`
/// - `PolicyExportTooLarge` — exported bytes exceed the configured limit
/// - `PolicyParseError` — policy file contains invalid UTF-8
pub trait PolicyProvider: Send + Sync {
    /// Check whether a snapshot commit is permitted.
    ///
    /// Called before any writes. Returns `Ok(())` to allow the commit,
    /// or `Err(ExError { kind: PolicyDenied, .. })` to abort it.
    ///
    /// # Arguments
    /// - `policy_ref` — policy document identifier
    /// - `profile_ref` — optional profile identifier (may be `None`)
    /// - `operation` — operation name (e.g. `"snapshot_commit"`)
    /// - `entity_id` — optional entity identifier (e.g. leaf EP ID)
    ///
    /// # Errors
    ///
    /// Returns `PolicyDenied` when the policy rejects the operation.
    /// Returns `PolicyNotFound` when `policy_ref` is unknown to this provider.
    #[allow(clippy::result_large_err)]
    fn policy_check(
        &self,
        policy_ref: &str,
        profile_ref: Option<&str>,
        operation: &str,
        entity_id: Option<&str>,
    ) -> Result<(), ExError>;

    /// Return the full canonical text of a policy document.
    ///
    /// # Errors
    ///
    /// Returns `PolicyNotFound` if the `policy_ref` is unknown to this provider.
    /// Returns `PolicyParseError` if the file contains invalid UTF-8.
    #[allow(clippy::result_large_err)]
    fn policy_read(&self, policy_ref: &str) -> Result<String, ExError>;

    /// Export structured content from a policy document.
    ///
    /// - `export_kind`: e.g. `"codegen_handoff"` — selects which HANDOFF blocks to extract.
    ///
    /// # Errors
    ///
    /// Returns `PolicyNotFound` if the ref is unknown.
    /// Returns `PolicyExportFailed` if the document has malformed/unterminated markers
    /// or an unknown `export_kind`.
    /// Returns `PolicyExportTooLarge` if the result exceeds the configured byte limit.
    #[allow(clippy::result_large_err)]
    fn policy_export(&self, policy_ref: &str, export_kind: &str) -> Result<String, ExError>;

    /// List all policy documents available in this provider.
    ///
    /// Returns a stable list (sorted by `policy_ref`) of known policies and their versions.
    ///
    /// # Errors
    ///
    /// Returns `Io` if the policies directory cannot be read.
    #[allow(clippy::result_large_err)]
    fn policy_list(&self) -> Result<Vec<PolicyListEntry>, ExError>;

    /// Produce a deterministic byte projection of a policy document for handoff.
    ///
    /// Extracts the HANDOFF block content from the policy identified by `policy_ref`
    /// and returns it as raw bytes. Two calls with identical inputs MUST return
    /// byte-identical output.
    ///
    /// - `policy_ref` — policy document identifier
    /// - `profile_ref` — optional profile identifier; if `Some`, profile existence
    ///   is validated at the engine layer before calling this method
    ///
    /// # Errors
    ///
    /// Returns `PolicyNotFound` if the `policy_ref` is unknown to this provider.
    /// Returns `PolicyExportFailed` if the document has malformed/unterminated HANDOFF markers.
    /// Returns `PolicyExportTooLarge` if the result exceeds the configured byte limit.
    #[allow(clippy::result_large_err)]
    fn policy_project_for_handoff(
        &self,
        policy_ref: &str,
        profile_ref: Option<&str>,
    ) -> Result<Vec<u8>, ExError>;
}

// ---------------------------------------------------------------------------
// NoopPolicyProvider
// ---------------------------------------------------------------------------

/// A policy provider that always allows commits and returns empty results.
///
/// Use as the default in CLI and in tests that don't exercise policy gating.
///
/// # Example
/// ```
/// use ettlex_core::policy_provider::{NoopPolicyProvider, PolicyProvider};
///
/// let provider = NoopPolicyProvider;
/// assert!(provider.policy_check("any", None, "snapshot_commit", None).is_ok());
/// assert!(provider.policy_list().unwrap().is_empty());
/// ```
pub struct NoopPolicyProvider;

impl PolicyProvider for NoopPolicyProvider {
    #[allow(clippy::result_large_err)]
    fn policy_check(
        &self,
        _policy_ref: &str,
        _profile_ref: Option<&str>,
        _operation: &str,
        _entity_id: Option<&str>,
    ) -> Result<(), ExError> {
        Ok(())
    }

    #[allow(clippy::result_large_err)]
    fn policy_read(&self, policy_ref: &str) -> Result<String, ExError> {
        Err(ExError::new(ExErrorKind::PolicyNotFound)
            .with_entity_id(policy_ref)
            .with_message("NoopPolicyProvider: policy not found"))
    }

    #[allow(clippy::result_large_err)]
    fn policy_export(&self, policy_ref: &str, _export_kind: &str) -> Result<String, ExError> {
        Err(ExError::new(ExErrorKind::PolicyNotFound)
            .with_entity_id(policy_ref)
            .with_message("NoopPolicyProvider: policy not found"))
    }

    #[allow(clippy::result_large_err)]
    fn policy_list(&self) -> Result<Vec<PolicyListEntry>, ExError> {
        Ok(vec![])
    }

    #[allow(clippy::result_large_err)]
    fn policy_project_for_handoff(
        &self,
        policy_ref: &str,
        _profile_ref: Option<&str>,
    ) -> Result<Vec<u8>, ExError> {
        Err(ExError::new(ExErrorKind::PolicyNotFound)
            .with_entity_id(policy_ref)
            .with_message("NoopPolicyProvider: policy not found"))
    }
}

// ---------------------------------------------------------------------------
// DenyAllPolicyProvider
// ---------------------------------------------------------------------------

/// A policy provider that always denies commits with `PolicyDenied`.
///
/// Use in tests that verify the policy gate stops all writes before any
/// durable state changes occur.
///
/// # Example
/// ```
/// use ettlex_core::errors::ExErrorKind;
/// use ettlex_core::policy_provider::{DenyAllPolicyProvider, PolicyProvider};
///
/// let provider = DenyAllPolicyProvider;
/// let err = provider.policy_check("any", None, "snapshot_commit", None).unwrap_err();
/// assert_eq!(err.kind(), ExErrorKind::PolicyDenied);
/// ```
pub struct DenyAllPolicyProvider;

impl PolicyProvider for DenyAllPolicyProvider {
    #[allow(clippy::result_large_err)]
    fn policy_check(
        &self,
        _policy_ref: &str,
        _profile_ref: Option<&str>,
        _operation: &str,
        _entity_id: Option<&str>,
    ) -> Result<(), ExError> {
        Err(ExError::new(ExErrorKind::PolicyDenied).with_message("DenyAll policy provider"))
    }

    #[allow(clippy::result_large_err)]
    fn policy_read(&self, policy_ref: &str) -> Result<String, ExError> {
        Err(ExError::new(ExErrorKind::PolicyNotFound)
            .with_entity_id(policy_ref)
            .with_message("DenyAllPolicyProvider: policy not found"))
    }

    #[allow(clippy::result_large_err)]
    fn policy_export(&self, policy_ref: &str, _export_kind: &str) -> Result<String, ExError> {
        Err(ExError::new(ExErrorKind::PolicyNotFound)
            .with_entity_id(policy_ref)
            .with_message("DenyAllPolicyProvider: policy not found"))
    }

    #[allow(clippy::result_large_err)]
    fn policy_list(&self) -> Result<Vec<PolicyListEntry>, ExError> {
        Ok(vec![])
    }

    #[allow(clippy::result_large_err)]
    fn policy_project_for_handoff(
        &self,
        policy_ref: &str,
        _profile_ref: Option<&str>,
    ) -> Result<Vec<u8>, ExError> {
        Err(ExError::new(ExErrorKind::PolicyNotFound)
            .with_entity_id(policy_ref)
            .with_message("DenyAllPolicyProvider: policy not found"))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::errors::ExErrorKind;

    #[test]
    fn test_noop_provider_allows() {
        let p = NoopPolicyProvider;
        assert!(p
            .policy_check(
                "policy/default@0",
                None,
                "snapshot_commit",
                Some("ep:root:0")
            )
            .is_ok());
    }

    #[test]
    fn test_noop_provider_list_empty() {
        let p = NoopPolicyProvider;
        assert!(p.policy_list().unwrap().is_empty());
    }

    #[test]
    fn test_noop_provider_read_not_found() {
        let p = NoopPolicyProvider;
        let err = p.policy_read("any-ref").unwrap_err();
        assert_eq!(err.kind(), ExErrorKind::PolicyNotFound);
    }

    #[test]
    fn test_deny_all_provider_denies() {
        let p = DenyAllPolicyProvider;
        let err = p
            .policy_check(
                "policy/default@0",
                None,
                "snapshot_commit",
                Some("ep:root:0"),
            )
            .unwrap_err();
        assert_eq!(err.kind(), ExErrorKind::PolicyDenied);
    }

    #[test]
    fn test_deny_all_provider_read_not_found() {
        let p = DenyAllPolicyProvider;
        let err = p.policy_read("any-ref").unwrap_err();
        assert_eq!(err.kind(), ExErrorKind::PolicyNotFound);
    }
}
