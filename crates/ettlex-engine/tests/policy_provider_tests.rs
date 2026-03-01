//! Scenario tests for ep:policy_codegen_handoff:0
//!
//! All 15 scenarios are tested here, written from spec descriptions only.
//! Tests are claims about requirements — not descriptions of the implementation.
//!
//! Scenario → test mapping:
//!   S1  test_s1_commit_denied_by_policy
//!   S2  test_s2_commit_allowed_by_policy_proceeds
//!   S3  test_s3_dry_run_policy_denied_before_any_writes
//!   S4  test_s4_engine_depends_on_policy_provider_trait
//!   S5  test_s5_anchor_adapter_matches_never_anchored
//!   S6  test_s6_export_returns_all_obligations
//!   S7  test_s7_export_is_deterministic
//!   S8  test_s8_export_fails_on_malformed_markers
//!   S9  test_s9_export_fails_policy_not_found
//!   S10 test_s10_policy_list_stable_ids_and_versions
//!   S11 test_s11_policy_read_returns_full_text
//!   S12 test_s12_manifest_policy_ref_from_committed_snapshot
//!   S13 test_s13_empty_policy_ref_returns_policy_ref_missing
//!   S14 test_s14_export_too_large_error
//!   S15 test_s15_invalid_utf8_returns_parse_error

use ettlex_core::approval_router::NoopApprovalRouter;
use ettlex_core::errors::ExErrorKind;
use ettlex_core::policy_provider::{DenyAllPolicyProvider, NoopPolicyProvider, PolicyProvider};
use ettlex_engine::commands::engine_command::{apply_engine_command, EngineCommand};
use ettlex_engine::commands::engine_query::{apply_engine_query, EngineQuery, EngineQueryResult};
use ettlex_engine::commands::snapshot::{snapshot_commit, SnapshotOptions};
use ettlex_store::cas::FsStore;
use ettlex_store::file_policy_provider::FilePolicyProvider;
use rusqlite::Connection;
use std::fs;
use tempfile::TempDir;

// ---------------------------------------------------------------------------
// Shared setup helpers
// ---------------------------------------------------------------------------

fn setup() -> (TempDir, Connection, FsStore) {
    let dir = TempDir::new().unwrap();
    let db = dir.path().join("test.db");
    let cas = dir.path().join("cas");
    let mut conn = Connection::open(&db).unwrap();
    ettlex_store::migrations::apply_migrations(&mut conn).unwrap();
    (dir, conn, FsStore::new(cas))
}

/// Insert a minimal ettle with one leaf EP.
fn seed_single_leaf(conn: &Connection) {
    conn.execute_batch(
        "INSERT INTO ettles (id, title, parent_id, deleted, created_at, updated_at, metadata)
         VALUES ('ettle:root', 'Root', NULL, 0, 0, 0, '{}');
         INSERT INTO eps (id, ettle_id, ordinal, normative, child_ettle_id, content_inline, deleted, created_at, updated_at)
         VALUES ('ep:root:0', 'ettle:root', 0, 1, NULL, 'leaf content', 0, 0, 0);",
    )
    .unwrap();
}

fn snapshot_count(conn: &Connection) -> i64 {
    conn.query_row("SELECT COUNT(*) FROM snapshots", [], |r| r.get(0))
        .unwrap()
}

fn commit_cmd(leaf_ep_id: &str, policy_ref: &str, dry_run: bool) -> EngineCommand {
    EngineCommand::SnapshotCommit {
        leaf_ep_id: leaf_ep_id.to_string(),
        policy_ref: policy_ref.to_string(),
        profile_ref: None,
        options: SnapshotOptions {
            expected_head: None,
            dry_run,
            allow_dedup: false,
        },
    }
}

// ---------------------------------------------------------------------------
// S1 — Commit denied by policy before CAS/ledger writes
//
// Spec: When DenyAllPolicyProvider is used, apply_engine_command must return
// PolicyDenied and must NOT write any snapshot row or approval row.
// ---------------------------------------------------------------------------

#[test]
fn test_s1_commit_denied_by_policy() {
    let (_dir, mut conn, cas) = setup();
    seed_single_leaf(&conn);

    let result = apply_engine_command(
        commit_cmd("ep:root:0", "policy/default@0", false),
        &mut conn,
        &cas,
        &DenyAllPolicyProvider,
        &NoopApprovalRouter,
    );

    assert!(result.is_err(), "DenyAll must fail the commit");
    assert_eq!(
        result.unwrap_err().kind(),
        ExErrorKind::PolicyDenied,
        "error kind must be PolicyDenied"
    );
    assert_eq!(
        snapshot_count(&conn),
        0,
        "no snapshot row must be written when policy denies"
    );
}

// ---------------------------------------------------------------------------
// S2 — Commit allowed by policy proceeds
//
// Spec: When NoopPolicyProvider is used, apply_engine_command must succeed
// and a snapshot row must be persisted.
// ---------------------------------------------------------------------------

#[test]
fn test_s2_commit_allowed_by_policy_proceeds() {
    let (_dir, mut conn, cas) = setup();
    seed_single_leaf(&conn);

    let result = apply_engine_command(
        commit_cmd("ep:root:0", "policy/default@0", false),
        &mut conn,
        &cas,
        &NoopPolicyProvider,
        &NoopApprovalRouter,
    );

    assert!(result.is_ok(), "NoopPolicyProvider must allow the commit");
    assert_eq!(
        snapshot_count(&conn),
        1,
        "exactly one snapshot row must be written"
    );
}

// ---------------------------------------------------------------------------
// S3 — Preview/dry_run mode: DenyAllPolicyProvider → PolicyDenied before any writes
//
// Spec: The policy check fires BEFORE the dry_run short-circuit. Even in
// dry_run=true mode, a DenyAllPolicyProvider must produce PolicyDenied and
// must not perform any computation beyond the check itself. No snapshot rows
// must be written (dry_run never writes, but the test confirms the error
// happens at the policy gate, not later).
// ---------------------------------------------------------------------------

#[test]
fn test_s3_dry_run_policy_denied_before_any_writes() {
    let (_dir, mut conn, cas) = setup();
    seed_single_leaf(&conn);

    let result = apply_engine_command(
        commit_cmd("ep:root:0", "policy/default@0", true),
        &mut conn,
        &cas,
        &DenyAllPolicyProvider,
        &NoopApprovalRouter,
    );

    assert!(result.is_err(), "DenyAll must fail even in dry_run mode");
    assert_eq!(
        result.unwrap_err().kind(),
        ExErrorKind::PolicyDenied,
        "error must be PolicyDenied, not a later-stage error"
    );
    assert_eq!(snapshot_count(&conn), 0);
}

// ---------------------------------------------------------------------------
// S4 — Engine depends on PolicyProvider trait, not backend-specific calls
//
// Spec: apply_engine_command accepts &dyn PolicyProvider. Any type implementing
// PolicyProvider can be passed without modification to the engine callsite.
// The trait object must be dispatchable from both Noop and DenyAll impls.
// ---------------------------------------------------------------------------

#[test]
fn test_s4_engine_depends_on_policy_provider_trait() {
    // Test that the engine accepts the trait object — not a concrete type.
    // If the engine required a concrete type, this would fail to compile.
    fn accepts_trait_object(p: &dyn PolicyProvider) -> ExErrorKind {
        p.policy_check("ref", None, "op", None)
            .err()
            .map(|e| e.kind())
            .unwrap_or(ExErrorKind::NotFound)
    }

    let noop: &dyn PolicyProvider = &NoopPolicyProvider;
    let deny: &dyn PolicyProvider = &DenyAllPolicyProvider;

    // Noop must allow (no error → we get the sentinel NotFound we set as default)
    assert_eq!(accepts_trait_object(noop), ExErrorKind::NotFound);
    // DenyAll must deny
    assert_eq!(accepts_trait_object(deny), ExErrorKind::PolicyDenied);
}

// ---------------------------------------------------------------------------
// S5 — PolicyProviderAnchorAdapter ≡ NeverAnchoredPolicy for all inputs
//
// Spec: PolicyProviderAnchorAdapter wraps any PolicyProvider and implements
// AnchorPolicy with NeverAnchored semantics — is_anchored_ep and
// is_anchored_ettle both return false for any input, matching NeverAnchoredPolicy.
// SelectedAnchoredPolicy must still work correctly (existing behaviour unchanged).
// ---------------------------------------------------------------------------

#[test]
fn test_s5_anchor_adapter_matches_never_anchored() {
    use ettlex_core::policy::{
        AnchorPolicy, NeverAnchoredPolicy, PolicyProviderAnchorAdapter, SelectedAnchoredPolicy,
    };
    use std::collections::HashSet;

    let adapter = PolicyProviderAnchorAdapter::new(&NoopPolicyProvider);
    let never = NeverAnchoredPolicy;

    // Adapter must match NeverAnchoredPolicy for arbitrary inputs
    for id in &["ep-1", "ep-abc", "ep:root:0", ""] {
        assert_eq!(
            adapter.is_anchored_ep(id),
            never.is_anchored_ep(id),
            "is_anchored_ep({id}) must match NeverAnchoredPolicy"
        );
        assert_eq!(
            adapter.is_anchored_ettle(id),
            never.is_anchored_ettle(id),
            "is_anchored_ettle({id}) must match NeverAnchoredPolicy"
        );
    }

    // Both must return false
    assert!(!adapter.is_anchored_ep("any-ep"));
    assert!(!adapter.is_anchored_ettle("any-ettle"));

    // SelectedAnchoredPolicy still works correctly (existing behaviour)
    let mut eps = HashSet::new();
    eps.insert("ep-anchored".to_string());
    let selected = SelectedAnchoredPolicy::new(eps, HashSet::new());
    assert!(selected.is_anchored_ep("ep-anchored"));
    assert!(!selected.is_anchored_ep("ep-other"));
}

// ---------------------------------------------------------------------------
// S6 — policy_export(ref, "codegen_handoff") returns all B1.x obligations
//
// Spec: Exporting the real codegen_handoff_policy_v1.md with export_kind
// "codegen_handoff" must return text that contains all the B1.x obligation
// sections (B1.1 through B1.6).
// ---------------------------------------------------------------------------

#[test]
fn test_s6_export_returns_all_obligations() {
    let policies_dir = workspace_policies_dir();
    let provider = FilePolicyProvider::new(&policies_dir);

    let result = provider.policy_export("codegen_handoff_policy_v1", "codegen_handoff");
    assert!(
        result.is_ok(),
        "export of codegen_handoff_policy_v1 must succeed: {:?}",
        result.err()
    );

    let text = result.unwrap();
    // Every B1.x obligation must appear in the exported text
    for label in &["B1.1", "B1.2", "B1.3", "B1.4", "B1.5", "B1.6"] {
        assert!(
            text.contains(label),
            "exported text must contain obligation {label}"
        );
    }
}

// ---------------------------------------------------------------------------
// S7 — Export is deterministic: same input → identical bytes on repeat calls
//
// Spec: Calling policy_export twice with the same arguments must produce
// byte-identical output.
// ---------------------------------------------------------------------------

#[test]
fn test_s7_export_is_deterministic() {
    let tmp = TempDir::new().unwrap();
    fs::write(
        tmp.path().join("det_policy.md"),
        "<!-- HANDOFF: START -->\nObligation A\nMore text\n<!-- HANDOFF: END -->\n\
         <!-- HANDOFF: START -->\nObligation B\n<!-- HANDOFF: END -->",
    )
    .unwrap();

    let provider = FilePolicyProvider::new(tmp.path());

    let first = provider
        .policy_export("det_policy", "codegen_handoff")
        .unwrap();
    let second = provider
        .policy_export("det_policy", "codegen_handoff")
        .unwrap();

    assert_eq!(
        first, second,
        "export must be byte-identical across repeated calls"
    );
}

// ---------------------------------------------------------------------------
// S8 — Export fails PolicyExportFailed on malformed/unterminated HANDOFF markers
//
// Spec: If the policy file has a <!-- HANDOFF: START --> without a matching
// <!-- HANDOFF: END -->, policy_export must return PolicyExportFailed.
// ---------------------------------------------------------------------------

#[test]
fn test_s8_export_fails_on_malformed_markers() {
    let tmp = TempDir::new().unwrap();
    // Unterminated START — no END
    fs::write(
        tmp.path().join("bad.md"),
        "<!-- HANDOFF: START -->\nThis block is never closed",
    )
    .unwrap();

    let provider = FilePolicyProvider::new(tmp.path());
    let err = provider
        .policy_export("bad", "codegen_handoff")
        .unwrap_err();

    assert_eq!(
        err.kind(),
        ExErrorKind::PolicyExportFailed,
        "unterminated HANDOFF block must return PolicyExportFailed"
    );
}

// ---------------------------------------------------------------------------
// S9 — Export fails PolicyNotFound on unknown policy_ref
//
// Spec: If policy_ref does not correspond to any file in the provider,
// policy_export must return PolicyNotFound.
// ---------------------------------------------------------------------------

#[test]
fn test_s9_export_fails_policy_not_found() {
    let tmp = TempDir::new().unwrap(); // empty dir — no policy files
    let provider = FilePolicyProvider::new(tmp.path());

    let err = provider
        .policy_export("does_not_exist", "codegen_handoff")
        .unwrap_err();

    assert_eq!(
        err.kind(),
        ExErrorKind::PolicyNotFound,
        "unknown policy_ref must return PolicyNotFound"
    );
}

// ---------------------------------------------------------------------------
// S10 — policy_list() returns stable ids + versions via engine query
//
// Spec: apply_engine_query(PolicyList, ..., Some(provider)) must return a
// sorted, stable list of PolicyListEntry values, one per policy file.
// Each entry has a policy_ref (file stem) and a version string.
// ---------------------------------------------------------------------------

#[test]
fn test_s10_policy_list_stable_ids_and_versions() {
    let (_dir, conn, cas) = setup();
    let policies_tmp = TempDir::new().unwrap();

    fs::write(policies_tmp.path().join("policy_z.md"), "# Z").unwrap();
    fs::write(policies_tmp.path().join("policy_a.md"), "# A").unwrap();
    fs::write(policies_tmp.path().join("policy_m.md"), "# M").unwrap();

    let provider = FilePolicyProvider::new(policies_tmp.path());
    let result = apply_engine_query(EngineQuery::PolicyList, &conn, &cas, Some(&provider)).unwrap();

    match result {
        EngineQueryResult::PolicyList(entries) => {
            // Must be sorted by policy_ref
            assert_eq!(entries.len(), 3);
            assert_eq!(entries[0].policy_ref, "policy_a");
            assert_eq!(entries[1].policy_ref, "policy_m");
            assert_eq!(entries[2].policy_ref, "policy_z");
            // Each must have a version field
            for e in &entries {
                assert!(
                    !e.version.is_empty(),
                    "version must not be empty for {}",
                    e.policy_ref
                );
            }
        }
        other => panic!("Expected PolicyList, got {:?}", other),
    }
}

// ---------------------------------------------------------------------------
// S11 — policy_read(ref) returns full canonical bytes via engine query
//
// Spec: apply_engine_query(PolicyRead { policy_ref }, ..., Some(provider))
// must return the complete, unmodified text of the policy file.
// ---------------------------------------------------------------------------

#[test]
fn test_s11_policy_read_returns_full_text() {
    let (_dir, conn, cas) = setup();
    let policies_tmp = TempDir::new().unwrap();

    let content = "# My Policy\n\nLine 1.\nLine 2.\n";
    fs::write(policies_tmp.path().join("my_policy.md"), content).unwrap();

    let provider = FilePolicyProvider::new(policies_tmp.path());
    let result = apply_engine_query(
        EngineQuery::PolicyRead {
            policy_ref: "my_policy".to_string(),
        },
        &conn,
        &cas,
        Some(&provider),
    )
    .unwrap();

    match result {
        EngineQueryResult::PolicyRead(r) => {
            assert_eq!(r.policy_ref, "my_policy");
            assert_eq!(
                r.text, content,
                "returned text must be byte-identical to the policy file"
            );
        }
        other => panic!("Expected PolicyRead, got {:?}", other),
    }
}

// ---------------------------------------------------------------------------
// S12 — SnapshotManifestPolicyRef returns policy_ref from committed manifest
//
// Spec: After a snapshot is committed with a specific policy_ref string,
// apply_engine_query(SnapshotManifestPolicyRef { manifest_digest }, ...)
// must return that exact policy_ref string.
// ---------------------------------------------------------------------------

#[test]
fn test_s12_manifest_policy_ref_from_committed_snapshot() {
    let (_dir, mut conn, cas) = setup();
    seed_single_leaf(&conn);

    let committed = snapshot_commit(
        "ettle:root",
        "policy/codegen-handoff@v1",
        "profile/default@0",
        SnapshotOptions {
            expected_head: None,
            dry_run: false,
            allow_dedup: false,
        },
        &mut conn,
        &cas,
    )
    .unwrap();

    let manifest_digest = committed.manifest_digest.clone();

    let result = apply_engine_query(
        EngineQuery::SnapshotManifestPolicyRef {
            manifest_digest: manifest_digest.clone(),
        },
        &conn,
        &cas,
        None,
    )
    .unwrap();

    match result {
        EngineQueryResult::SnapshotManifestPolicyRef(policy_ref) => {
            assert_eq!(
                policy_ref, "policy/codegen-handoff@v1",
                "policy_ref in manifest must match what was passed at commit time"
            );
        }
        other => panic!("Expected SnapshotManifestPolicyRef, got {:?}", other),
    }
}

// ---------------------------------------------------------------------------
// S13 — PolicyRefMissing when empty policy_ref passed to snapshot commit
//
// Spec: If policy_ref is an empty string, apply_engine_command must return
// PolicyRefMissing immediately — before any policy check, before any writes.
// This applies in both dry_run=false and dry_run=true modes.
// ---------------------------------------------------------------------------

#[test]
fn test_s13_empty_policy_ref_returns_policy_ref_missing() {
    let (_dir, mut conn, cas) = setup();
    seed_single_leaf(&conn);

    // Normal commit mode
    let err = apply_engine_command(
        commit_cmd("ep:root:0", "", false),
        &mut conn,
        &cas,
        &NoopPolicyProvider,
        &NoopApprovalRouter,
    )
    .unwrap_err();

    assert_eq!(
        err.kind(),
        ExErrorKind::PolicyRefMissing,
        "empty policy_ref must produce PolicyRefMissing"
    );
    assert_eq!(
        snapshot_count(&conn),
        0,
        "no snapshot must be written when policy_ref is missing"
    );

    // Also enforced in dry_run mode (policy gate fires before dry_run short-circuit)
    let err_dry = apply_engine_command(
        commit_cmd("ep:root:0", "", true),
        &mut conn,
        &cas,
        &NoopPolicyProvider,
        &NoopApprovalRouter,
    )
    .unwrap_err();

    assert_eq!(
        err_dry.kind(),
        ExErrorKind::PolicyRefMissing,
        "empty policy_ref must produce PolicyRefMissing even in dry_run mode"
    );
}

// ---------------------------------------------------------------------------
// S14 — PolicyExportTooLarge when policy text exceeds configured max bytes
//
// Spec: When the extracted HANDOFF content exceeds the configured max_export_bytes
// limit, policy_export must return PolicyExportTooLarge.
// ---------------------------------------------------------------------------

#[test]
fn test_s14_export_too_large_error() {
    let tmp = TempDir::new().unwrap();
    // Write a policy whose HANDOFF content is larger than the configured limit
    let large_content = "x".repeat(500);
    fs::write(
        tmp.path().join("large.md"),
        format!(
            "<!-- HANDOFF: START -->\n{}\n<!-- HANDOFF: END -->",
            large_content
        ),
    )
    .unwrap();

    // Set max to 100 bytes — far below the content size
    let provider = FilePolicyProvider::new(tmp.path()).with_max_bytes(100);

    let err = provider
        .policy_export("large", "codegen_handoff")
        .unwrap_err();

    assert_eq!(
        err.kind(),
        ExErrorKind::PolicyExportTooLarge,
        "export exceeding max_bytes must return PolicyExportTooLarge"
    );
}

// ---------------------------------------------------------------------------
// S15 — PolicyParseError from invalid UTF-8 in policy file
//
// Spec: If a policy file contains bytes that are not valid UTF-8,
// policy_read must return PolicyParseError with the policy_ref in context.
// ---------------------------------------------------------------------------

#[test]
fn test_s15_invalid_utf8_returns_parse_error() {
    let tmp = TempDir::new().unwrap();
    // Write raw invalid UTF-8 bytes
    fs::write(
        tmp.path().join("corrupt.md"),
        b"\xff\xfe\x80\x81 not valid utf-8",
    )
    .unwrap();

    let provider = FilePolicyProvider::new(tmp.path());

    let err = provider.policy_read("corrupt").unwrap_err();

    assert_eq!(
        err.kind(),
        ExErrorKind::PolicyParseError,
        "invalid UTF-8 in policy file must return PolicyParseError"
    );
}

// ---------------------------------------------------------------------------
// S-PH-1 — policy.project_for_handoff returns deterministic projection bytes
//
// Spec: Two calls to PolicyProjectForHandoff with the same policy_ref must
// return byte-identical results. The call is read-only: no snapshot row is
// written, no approval row is added.
// ---------------------------------------------------------------------------

#[test]
fn test_project_for_handoff_deterministic() {
    let (_dir, conn, cas) = setup();
    let policies_tmp = TempDir::new().unwrap();

    // Write a policy with a HANDOFF block
    fs::write(
        policies_tmp.path().join("proj_policy.md"),
        "<!-- HANDOFF: START -->\nObligation X\n<!-- HANDOFF: END -->",
    )
    .unwrap();

    let provider = FilePolicyProvider::new(policies_tmp.path());

    let result1 = apply_engine_query(
        EngineQuery::PolicyProjectForHandoff {
            policy_ref: "proj_policy".to_string(),
            profile_ref: None,
        },
        &conn,
        &cas,
        Some(&provider),
    )
    .unwrap();

    let result2 = apply_engine_query(
        EngineQuery::PolicyProjectForHandoff {
            policy_ref: "proj_policy".to_string(),
            profile_ref: None,
        },
        &conn,
        &cas,
        Some(&provider),
    )
    .unwrap();

    let bytes1 = match result1 {
        EngineQueryResult::PolicyProjectForHandoff(r) => r.projection_bytes,
        other => panic!("Expected PolicyProjectForHandoff, got {:?}", other),
    };
    let bytes2 = match result2 {
        EngineQueryResult::PolicyProjectForHandoff(r) => r.projection_bytes,
        other => panic!("Expected PolicyProjectForHandoff, got {:?}", other),
    };

    assert_eq!(
        bytes1, bytes2,
        "projection bytes must be identical across repeated calls"
    );
    assert!(!bytes1.is_empty(), "projection bytes must not be empty");

    // No snapshot or approval rows written
    assert_eq!(snapshot_count(&conn), 0, "no snapshot must be written");
}

// ---------------------------------------------------------------------------
// S-PH-2 — policy.project_for_handoff fails for unknown policy_ref
//
// Spec: If policy_ref does not exist in the provider, the query must return
// Err with kind PolicyNotFound. No state mutation must occur.
// ---------------------------------------------------------------------------

#[test]
fn test_project_for_handoff_unknown_policy_ref() {
    let (_dir, conn, cas) = setup();
    let policies_tmp = TempDir::new().unwrap(); // empty — no policies

    let provider = FilePolicyProvider::new(policies_tmp.path());

    let err = apply_engine_query(
        EngineQuery::PolicyProjectForHandoff {
            policy_ref: "nonexistent_policy".to_string(),
            profile_ref: None,
        },
        &conn,
        &cas,
        Some(&provider),
    )
    .unwrap_err();

    assert_eq!(
        err.kind(),
        ExErrorKind::PolicyNotFound,
        "unknown policy_ref must return PolicyNotFound"
    );
    assert_eq!(snapshot_count(&conn), 0, "no snapshot must be written");
}

// ---------------------------------------------------------------------------
// S-PH-3 — policy.project_for_handoff fails for unknown profile_ref
//
// Spec: If profile_ref is Some but the referenced profile does not exist in
// the store, the query must return Err with kind ProfileNotFound. No state
// mutation must occur. policy_ref must be valid (policy file exists).
// ---------------------------------------------------------------------------

#[test]
fn test_project_for_handoff_unknown_profile_ref() {
    let (_dir, conn, cas) = setup();
    let policies_tmp = TempDir::new().unwrap();

    // Write a valid policy
    fs::write(
        policies_tmp.path().join("valid_policy.md"),
        "<!-- HANDOFF: START -->\nObligation Y\n<!-- HANDOFF: END -->",
    )
    .unwrap();

    let provider = FilePolicyProvider::new(policies_tmp.path());

    let err = apply_engine_query(
        EngineQuery::PolicyProjectForHandoff {
            policy_ref: "valid_policy".to_string(),
            profile_ref: Some("profile/unknown@99".to_string()),
        },
        &conn,
        &cas,
        Some(&provider),
    )
    .unwrap_err();

    assert_eq!(
        err.kind(),
        ExErrorKind::ProfileNotFound,
        "unknown profile_ref must return ProfileNotFound"
    );
    assert_eq!(snapshot_count(&conn), 0, "no snapshot must be written");
}

// ---------------------------------------------------------------------------
// Helper: locate the workspace-level policies/ directory
// ---------------------------------------------------------------------------

fn workspace_policies_dir() -> std::path::PathBuf {
    // Navigate from this crate's manifest dir up to the workspace root
    std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent() // crates/
        .unwrap()
        .parent() // workspace root
        .unwrap()
        .join("policies")
}
