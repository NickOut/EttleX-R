//! File-system backed policy provider.
//!
//! [`FilePolicyProvider`] reads policy documents from a directory on the local
//! filesystem. Each `.md` file in the directory is a policy document. The
//! `policy_ref` is the file stem (e.g. `codegen_handoff_policy_v1` for
//! `codegen_handoff_policy_v1.md`).
//!
//! ## HANDOFF marker format
//!
//! Policy export with `export_kind = "codegen_handoff"` extracts all blocks
//! delimited by:
//!
//! ```markdown
//! <!-- HANDOFF: START -->
//! ... obligation text ...
//! <!-- HANDOFF: END -->
//! ```
//!
//! Blocks are concatenated in document order. Unterminated or nested markers
//! produce [`PolicyExportFailed`].
//!
//! ## Byte limit
//!
//! The default maximum export size is 1 MiB (1,048,576 bytes). Override with
//! [`FilePolicyProvider::with_max_bytes`].
//!
//! [`PolicyExportFailed`]: ettlex_core::errors::ExErrorKind::PolicyExportFailed

use ettlex_core::errors::{ExError, ExErrorKind};
use ettlex_core::policy_provider::{PolicyListEntry, PolicyProvider};
use std::path::PathBuf;
use std::result::Result;

/// Default maximum byte size for a policy export result (1 MiB).
pub const DEFAULT_MAX_EXPORT_BYTES: usize = 1_048_576;

const HANDOFF_START: &str = "<!-- HANDOFF: START -->";
const HANDOFF_END: &str = "<!-- HANDOFF: END -->";

/// A policy provider backed by Markdown files on the local filesystem.
///
/// ## Usage
///
/// ```no_run
/// use ettlex_store::file_policy_provider::FilePolicyProvider;
/// use ettlex_core::policy_provider::PolicyProvider;
///
/// let provider = FilePolicyProvider::new("policies");
/// let list = provider.policy_list().unwrap();
/// ```
pub struct FilePolicyProvider {
    policies_dir: PathBuf,
    max_export_bytes: usize,
}

impl FilePolicyProvider {
    /// Create a new provider reading from `policies_dir`.
    pub fn new(policies_dir: impl Into<PathBuf>) -> Self {
        Self {
            policies_dir: policies_dir.into(),
            max_export_bytes: DEFAULT_MAX_EXPORT_BYTES,
        }
    }

    /// Override the maximum allowed export byte size.
    pub fn with_max_bytes(mut self, max: usize) -> Self {
        self.max_export_bytes = max;
        self
    }

    fn policy_path(&self, policy_ref: &str) -> PathBuf {
        self.policies_dir.join(format!("{}.md", policy_ref))
    }

    #[allow(clippy::result_large_err)]
    fn read_policy_text(&self, policy_ref: &str) -> Result<String, ExError> {
        let path = self.policy_path(policy_ref);
        if !path.exists() {
            return Err(ExError::new(ExErrorKind::PolicyNotFound)
                .with_entity_id(policy_ref)
                .with_message(format!("Policy file not found: {}", path.display())));
        }
        let bytes = std::fs::read(&path).map_err(|e| {
            ExError::new(ExErrorKind::PolicyParseError)
                .with_entity_id(policy_ref)
                .with_message(format!("Failed to read policy file: {}", e))
        })?;
        String::from_utf8(bytes).map_err(|e| {
            ExError::new(ExErrorKind::PolicyParseError)
                .with_entity_id(policy_ref)
                .with_message(format!("Policy file contains invalid UTF-8: {}", e))
        })
    }
}

impl PolicyProvider for FilePolicyProvider {
    #[allow(clippy::result_large_err)]
    fn policy_create(&self, policy_ref: &str, text: &str) -> Result<(), ExError> {
        // Validate policy_ref: non-empty, must contain '@' separator
        if policy_ref.is_empty() {
            return Err(ExError::new(ExErrorKind::InvalidInput)
                .with_entity_id(policy_ref)
                .with_message("policy_ref must not be empty"));
        }
        if !policy_ref.contains('@') {
            return Err(ExError::new(ExErrorKind::InvalidInput)
                .with_entity_id(policy_ref)
                .with_message("policy_ref must contain '@' version separator"));
        }
        // Validate text: non-empty
        if text.is_empty() {
            return Err(ExError::new(ExErrorKind::InvalidInput)
                .with_entity_id(policy_ref)
                .with_message("policy text must not be empty"));
        }
        // Check if file already exists → PolicyConflict
        let dest = self.policy_path(policy_ref);
        if dest.exists() {
            return Err(ExError::new(ExErrorKind::PolicyConflict)
                .with_entity_id(policy_ref)
                .with_message(format!("Policy already exists: {}", policy_ref)));
        }
        // Write atomically: write to a temp file, then rename
        let tmp_path = dest.with_extension("md.tmp");
        std::fs::write(&tmp_path, text).map_err(|e| {
            ExError::new(ExErrorKind::Io)
                .with_entity_id(policy_ref)
                .with_message(format!("Failed to write policy file: {}", e))
        })?;
        std::fs::rename(&tmp_path, &dest).map_err(|e| {
            // Clean up temp file on rename failure
            let _ = std::fs::remove_file(&tmp_path);
            ExError::new(ExErrorKind::Io)
                .with_entity_id(policy_ref)
                .with_message(format!("Failed to rename policy file: {}", e))
        })?;
        Ok(())
    }

    #[allow(clippy::result_large_err)]
    fn policy_check(
        &self,
        policy_ref: &str,
        _profile_ref: Option<&str>,
        _operation: &str,
        _entity_id: Option<&str>,
    ) -> Result<(), ExError> {
        // Check that the policy document exists; return PolicyNotFound if not.
        let path = self.policy_path(policy_ref);
        if !path.exists() {
            return Err(ExError::new(ExErrorKind::PolicyNotFound)
                .with_entity_id(policy_ref)
                .with_message(format!("Policy file not found: {}", path.display())));
        }
        Ok(())
    }

    #[allow(clippy::result_large_err)]
    fn policy_read(&self, policy_ref: &str) -> Result<String, ExError> {
        self.read_policy_text(policy_ref)
    }

    #[allow(clippy::result_large_err)]
    fn policy_export(&self, policy_ref: &str, export_kind: &str) -> Result<String, ExError> {
        if export_kind != "codegen_handoff" {
            return Err(ExError::new(ExErrorKind::PolicyExportFailed)
                .with_entity_id(policy_ref)
                .with_message(format!("Unknown export_kind: '{}'", export_kind)));
        }

        let text = self.read_policy_text(policy_ref)?;

        // Extract all HANDOFF blocks
        let mut result = String::new();
        let mut remaining = text.as_str();
        let mut found_any = false;

        loop {
            match remaining.find(HANDOFF_START) {
                None => {
                    // No more START markers; check for orphaned END
                    if remaining.contains(HANDOFF_END) {
                        return Err(ExError::new(ExErrorKind::PolicyExportFailed)
                            .with_entity_id(policy_ref)
                            .with_message("Malformed HANDOFF markers: END without START"));
                    }
                    break;
                }
                Some(start_pos) => {
                    let after_start = &remaining[start_pos + HANDOFF_START.len()..];
                    match after_start.find(HANDOFF_END) {
                        None => {
                            return Err(ExError::new(ExErrorKind::PolicyExportFailed)
                                .with_entity_id(policy_ref)
                                .with_message(
                                    "Malformed HANDOFF markers: unterminated START block",
                                ));
                        }
                        Some(end_pos) => {
                            // Check for nested START inside the block
                            let block_content = &after_start[..end_pos];
                            if block_content.contains(HANDOFF_START) {
                                return Err(ExError::new(ExErrorKind::PolicyExportFailed)
                                    .with_entity_id(policy_ref)
                                    .with_message(
                                        "Malformed HANDOFF markers: nested START inside block",
                                    ));
                            }
                            if !result.is_empty() {
                                result.push('\n');
                            }
                            result.push_str(block_content.trim());
                            found_any = true;
                            remaining = &after_start[end_pos + HANDOFF_END.len()..];
                        }
                    }
                }
            }
        }

        if !found_any {
            return Err(ExError::new(ExErrorKind::PolicyExportFailed)
                .with_entity_id(policy_ref)
                .with_message("No HANDOFF blocks found in policy document"));
        }

        if result.len() > self.max_export_bytes {
            return Err(ExError::new(ExErrorKind::PolicyExportTooLarge)
                .with_entity_id(policy_ref)
                .with_message(format!(
                    "Export size {} bytes exceeds limit of {} bytes",
                    result.len(),
                    self.max_export_bytes
                )));
        }

        Ok(result)
    }

    #[allow(clippy::result_large_err)]
    fn policy_project_for_handoff(
        &self,
        policy_ref: &str,
        _profile_ref: Option<&str>,
    ) -> Result<Vec<u8>, ExError> {
        // Reuse the HANDOFF extraction logic from policy_export.
        // profile_ref existence is validated at the engine layer before this call.
        let text = self.policy_export(policy_ref, "codegen_handoff")?;
        Ok(text.into_bytes())
    }

    #[allow(clippy::result_large_err)]
    fn policy_list(&self) -> Result<Vec<PolicyListEntry>, ExError> {
        let dir = &self.policies_dir;
        if !dir.exists() {
            return Ok(vec![]);
        }

        let mut entries: Vec<PolicyListEntry> = std::fs::read_dir(dir)
            .map_err(|e| {
                ExError::new(ExErrorKind::Io)
                    .with_message(format!("Failed to read policies directory: {}", e))
            })?
            .filter_map(|entry| {
                let entry = entry.ok()?;
                let path = entry.path();
                if path.extension()?.to_str()? == "md" {
                    let policy_ref = path.file_stem()?.to_str()?.to_string();
                    Some(PolicyListEntry {
                        policy_ref,
                        version: "0".to_string(),
                    })
                } else {
                    None
                }
            })
            .collect();

        // Return in stable sorted order
        entries.sort_by(|a, b| a.policy_ref.cmp(&b.policy_ref));
        Ok(entries)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;

    fn make_provider(tmp: &TempDir) -> FilePolicyProvider {
        FilePolicyProvider::new(tmp.path())
    }

    fn write_policy(tmp: &TempDir, name: &str, content: &str) {
        fs::write(tmp.path().join(format!("{}.md", name)), content).unwrap();
    }

    // S6: Export returns all obligation blocks
    #[test]
    fn test_s6_export_includes_obligations() {
        let tmp = TempDir::new().unwrap();
        write_policy(
            &tmp,
            "test_policy",
            "<!-- HANDOFF: START -->\nObligation 1\n<!-- HANDOFF: END -->\n<!-- HANDOFF: START -->\nObligation 2\n<!-- HANDOFF: END -->",
        );
        let p = make_provider(&tmp);
        let result = p.policy_export("test_policy", "codegen_handoff").unwrap();
        assert!(
            result.contains("Obligation 1"),
            "should contain obligation 1"
        );
        assert!(
            result.contains("Obligation 2"),
            "should contain obligation 2"
        );
    }

    // S7: Export is deterministic (same bytes on repeated calls)
    #[test]
    fn test_s7_export_deterministic() {
        let tmp = TempDir::new().unwrap();
        write_policy(
            &tmp,
            "test_policy",
            "<!-- HANDOFF: START -->\nObligation A\n<!-- HANDOFF: END -->",
        );
        let p = make_provider(&tmp);
        let r1 = p.policy_export("test_policy", "codegen_handoff").unwrap();
        let r2 = p.policy_export("test_policy", "codegen_handoff").unwrap();
        assert_eq!(r1, r2, "export must be deterministic");
    }

    // S8: Export fails on unterminated HANDOFF markers
    #[test]
    fn test_s8_export_malformed_markers() {
        let tmp = TempDir::new().unwrap();
        write_policy(
            &tmp,
            "bad_policy",
            "<!-- HANDOFF: START -->\nUnterminated block",
        );
        let p = make_provider(&tmp);
        let err = p
            .policy_export("bad_policy", "codegen_handoff")
            .unwrap_err();
        assert_eq!(err.kind(), ExErrorKind::PolicyExportFailed);
    }

    // S8b: Export fails on END without START
    #[test]
    fn test_s8b_export_end_without_start() {
        let tmp = TempDir::new().unwrap();
        write_policy(&tmp, "bad_policy", "Some content\n<!-- HANDOFF: END -->");
        let p = make_provider(&tmp);
        let err = p
            .policy_export("bad_policy", "codegen_handoff")
            .unwrap_err();
        assert_eq!(err.kind(), ExErrorKind::PolicyExportFailed);
    }

    // S9: Export fails PolicyNotFound on unknown policy_ref
    #[test]
    fn test_s9_export_unknown_policy_ref() {
        let tmp = TempDir::new().unwrap();
        let p = make_provider(&tmp);
        let err = p
            .policy_export("nonexistent", "codegen_handoff")
            .unwrap_err();
        assert_eq!(err.kind(), ExErrorKind::PolicyNotFound);
    }

    // S10: policy_list returns stable ids + versions
    #[test]
    fn test_s10_list_policies() {
        let tmp = TempDir::new().unwrap();
        write_policy(&tmp, "policy_alpha", "content");
        write_policy(&tmp, "policy_beta", "content");
        let p = make_provider(&tmp);
        let list = p.policy_list().unwrap();
        assert_eq!(list.len(), 2);
        assert_eq!(list[0].policy_ref, "policy_alpha");
        assert_eq!(list[1].policy_ref, "policy_beta");
        assert!(list.iter().all(|e| e.version == "0"));
    }

    // S11: policy_read returns full canonical text
    #[test]
    fn test_s11_read_policy_text() {
        let tmp = TempDir::new().unwrap();
        write_policy(&tmp, "my_policy", "# Policy\nFull text here.");
        let p = make_provider(&tmp);
        let text = p.policy_read("my_policy").unwrap();
        assert_eq!(text, "# Policy\nFull text here.");
    }

    // S14: PolicyExportTooLarge when result exceeds max_bytes
    #[test]
    fn test_s14_export_too_large() {
        let tmp = TempDir::new().unwrap();
        let big_content = "x".repeat(100);
        write_policy(
            &tmp,
            "big_policy",
            &format!(
                "<!-- HANDOFF: START -->\n{}\n<!-- HANDOFF: END -->",
                big_content
            ),
        );
        let p = FilePolicyProvider::new(tmp.path()).with_max_bytes(10);
        let err = p
            .policy_export("big_policy", "codegen_handoff")
            .unwrap_err();
        assert_eq!(err.kind(), ExErrorKind::PolicyExportTooLarge);
    }

    // S15: PolicyParseError from invalid UTF-8 in policy file
    #[test]
    fn test_s15_policy_parse_error() {
        let tmp = TempDir::new().unwrap();
        // Write invalid UTF-8 bytes
        let path = tmp.path().join("invalid_policy.md");
        fs::write(&path, b"\xff\xfe invalid utf8 \x80\x81").unwrap();
        let p = make_provider(&tmp);
        let err = p.policy_read("invalid_policy").unwrap_err();
        assert_eq!(err.kind(), ExErrorKind::PolicyParseError);
    }

    // policy_check: Ok when file exists
    #[test]
    fn test_policy_check_ok() {
        let tmp = TempDir::new().unwrap();
        write_policy(&tmp, "p", "content");
        let p = make_provider(&tmp);
        assert!(p.policy_check("p", None, "snapshot_commit", None).is_ok());
    }

    // policy_check: PolicyNotFound when file missing
    #[test]
    fn test_policy_check_not_found() {
        let tmp = TempDir::new().unwrap();
        let p = make_provider(&tmp);
        let err = p
            .policy_check("missing", None, "snapshot_commit", None)
            .unwrap_err();
        assert_eq!(err.kind(), ExErrorKind::PolicyNotFound);
    }

    // Nested START inside a block → PolicyExportFailed
    #[test]
    fn test_nested_start_inside_block() {
        let tmp = TempDir::new().unwrap();
        write_policy(
            &tmp,
            "nested",
            "<!-- HANDOFF: START -->\n<!-- HANDOFF: START -->\ninner\n<!-- HANDOFF: END -->",
        );
        let p = make_provider(&tmp);
        let err = p.policy_export("nested", "codegen_handoff").unwrap_err();
        assert_eq!(err.kind(), ExErrorKind::PolicyExportFailed);
    }

    // Unknown export_kind → PolicyExportFailed
    #[test]
    fn test_unknown_export_kind() {
        let tmp = TempDir::new().unwrap();
        write_policy(
            &tmp,
            "p",
            "<!-- HANDOFF: START -->\nA\n<!-- HANDOFF: END -->",
        );
        let p = make_provider(&tmp);
        let err = p.policy_export("p", "unknown_kind").unwrap_err();
        assert_eq!(err.kind(), ExErrorKind::PolicyExportFailed);
    }

    // No HANDOFF blocks in an existing file → PolicyExportFailed
    #[test]
    fn test_no_handoff_blocks() {
        let tmp = TempDir::new().unwrap();
        write_policy(
            &tmp,
            "empty_blocks",
            "# Just a policy with no HANDOFF markers",
        );
        let p = make_provider(&tmp);
        let err = p
            .policy_export("empty_blocks", "codegen_handoff")
            .unwrap_err();
        assert_eq!(err.kind(), ExErrorKind::PolicyExportFailed);
    }

    // Real policy file: export finds 6 obligations
    #[test]
    fn test_codegen_handoff_policy_has_obligations() {
        // This test only runs if the policies/ directory exists at the workspace root.
        let policies_dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .parent()
            .unwrap()
            .parent()
            .unwrap()
            .join("policies");
        if !policies_dir.exists() {
            return; // skip if no policies dir
        }
        let p = FilePolicyProvider::new(&policies_dir);
        let result = p.policy_export("codegen_handoff_policy_v1", "codegen_handoff");
        assert!(
            result.is_ok(),
            "real policy export should succeed: {:?}",
            result
        );
        let text = result.unwrap();
        // Must contain all 6 B1.x obligations
        assert!(text.contains("B1.1"), "missing B1.1");
        assert!(text.contains("B1.2"), "missing B1.2");
        assert!(text.contains("B1.3"), "missing B1.3");
        assert!(text.contains("B1.4"), "missing B1.4");
        assert!(text.contains("B1.5"), "missing B1.5");
        assert!(text.contains("B1.6"), "missing B1.6");
    }
}
