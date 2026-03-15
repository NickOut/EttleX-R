//! Ettle CRUD scenarios — Slice 01.
//!
//! SC-01  test_create_minimal_ettle_succeeds
//! SC-02  test_create_returns_ettle_id
//! SC-03  test_create_with_all_fields_succeeds
//! SC-04  test_create_with_reasoning_link_succeeds
//! SC-05  test_create_empty_title_fails
//! SC-06  test_create_rejects_caller_supplied_id
//! SC-07  test_create_link_without_type_fails
//! SC-08  test_create_type_without_link_fails
//! SC-09  test_create_link_to_nonexistent_ettle_fails
//! SC-10  test_create_link_to_tombstoned_ettle_fails
//! SC-11  test_create_whitespace_only_title_fails
//! SC-12  test_get_returns_all_fields
//! SC-13  test_get_nonexistent_returns_not_found
//! SC-14  test_list_empty_returns_empty_page
//! SC-15  test_list_single_ettle
//! SC-16  test_list_pagination_cursor
//! SC-17  test_list_limit_zero_fails
//! SC-18  test_list_limit_over_500_fails
//! SC-19  test_list_invalid_cursor_fails
//! SC-20  test_list_excludes_tombstoned_by_default
//! SC-21  test_list_include_tombstoned_flag
//! SC-22  test_update_title_succeeds
//! SC-23  test_update_why_succeeds
//! SC-24  test_update_what_succeeds
//! SC-25  test_update_how_succeeds
//! SC-26  test_update_sets_reasoning_link
//! SC-27  test_update_changes_reasoning_link
//! SC-28  test_update_clears_reasoning_link
//! SC-29  test_update_preserves_unspecified_fields
//! SC-30  test_update_rejects_self_referential_link
//! SC-31  test_update_nonexistent_ettle_fails
//! SC-32  test_update_tombstoned_ettle_fails
//! SC-33  test_update_empty_update_fails
//! SC-34  test_update_link_to_nonexistent_fails
//! SC-35  test_update_link_without_type_fails
//! SC-36  test_tombstone_active_ettle_succeeds
//! SC-37  test_tombstone_nonexistent_ettle_fails
//! SC-38  test_tombstone_already_tombstoned_fails
//! SC-39  test_tombstone_with_active_dependants_fails
//! SC-40  test_tombstone_allows_tombstoned_dependant
//! SC-41  test_hard_delete_not_exposed
//! SC-42  test_occ_correct_version_succeeds
//! SC-43  test_occ_wrong_version_fails
//! SC-44  test_each_mutation_appends_one_provenance_event
//! SC-45  test_failed_command_no_provenance_event
//! SC-46  test_ettle_get_byte_identical
//! SC-47  test_ettle_list_byte_identical
//! SC-48  test_create_large_fields_succeeds
//! SC-49  test_list_max_limit_succeeds

#![allow(clippy::unwrap_used)]

use ettlex_core::approval_router::NoopApprovalRouter;
use ettlex_core::errors::ExErrorKind;
use ettlex_core::policy_provider::NoopPolicyProvider;
use ettlex_engine::commands::ettle::{handle_ettle_get, handle_ettle_list};
use ettlex_engine::commands::mcp_command::{apply_mcp_command, McpCommand, McpCommandResult};
use ettlex_store::cas::FsStore;
use ettlex_store::model::{EttleListOpts, EttleListPage};
use rusqlite::Connection;
use tempfile::TempDir;

fn setup() -> (TempDir, Connection, FsStore) {
    let dir = TempDir::new().unwrap();
    let db_path = dir.path().join("test.db");
    let cas_path = dir.path().join("cas");
    let mut conn = Connection::open(&db_path).unwrap();
    ettlex_store::migrations::apply_migrations(&mut conn).unwrap();
    (dir, conn, FsStore::new(cas_path))
}

fn create_ettle(conn: &mut Connection, cas: &FsStore, title: &str) -> String {
    let cmd = McpCommand::EttleCreate {
        title: title.to_string(),
        ettle_id: None,
        why: None,
        what: None,
        how: None,
        reasoning_link_id: None,
        reasoning_link_type: None,
    };
    let (result, _) = apply_mcp_command(
        cmd,
        None,
        conn,
        cas,
        &NoopPolicyProvider,
        &NoopApprovalRouter,
    )
    .unwrap();
    match result {
        McpCommandResult::EttleCreate { ettle_id } => ettle_id,
        _ => panic!("expected EttleCreate result"),
    }
}

// ---------------------------------------------------------------------------
// SC-01: create_minimal_ettle_succeeds
// ---------------------------------------------------------------------------

#[test]
fn test_create_minimal_ettle_succeeds() {
    let (_dir, mut conn, cas) = setup();
    let cmd = McpCommand::EttleCreate {
        title: "My Ettle".to_string(),
        ettle_id: None,
        why: None,
        what: None,
        how: None,
        reasoning_link_id: None,
        reasoning_link_type: None,
    };
    let result = apply_mcp_command(
        cmd,
        None,
        &mut conn,
        &cas,
        &NoopPolicyProvider,
        &NoopApprovalRouter,
    );
    assert!(
        result.is_ok(),
        "minimal ettle create must succeed: {:?}",
        result.err()
    );
}

// ---------------------------------------------------------------------------
// SC-02: create_returns_ettle_id
// ---------------------------------------------------------------------------

#[test]
fn test_create_returns_ettle_id() {
    let (_dir, mut conn, cas) = setup();
    let id = create_ettle(&mut conn, &cas, "Test Ettle");
    assert!(
        id.starts_with("ettle:"),
        "ettle_id must start with 'ettle:': {}",
        id
    );
}

// ---------------------------------------------------------------------------
// SC-03: create_with_all_fields_succeeds
// ---------------------------------------------------------------------------

#[test]
fn test_create_with_all_fields_succeeds() {
    let (_dir, mut conn, cas) = setup();
    let cmd = McpCommand::EttleCreate {
        title: "Full Ettle".to_string(),
        ettle_id: None,
        why: Some("Because we must".to_string()),
        what: Some("The thing we do".to_string()),
        how: Some("By doing it".to_string()),
        reasoning_link_id: None,
        reasoning_link_type: None,
    };
    let result = apply_mcp_command(
        cmd,
        None,
        &mut conn,
        &cas,
        &NoopPolicyProvider,
        &NoopApprovalRouter,
    );
    assert!(
        result.is_ok(),
        "all-fields create must succeed: {:?}",
        result.err()
    );

    // Verify fields persisted
    if let Ok((McpCommandResult::EttleCreate { ettle_id }, _)) = result {
        let record = handle_ettle_get(&conn, &ettle_id).unwrap();
        assert_eq!(record.why, "Because we must");
        assert_eq!(record.what, "The thing we do");
        assert_eq!(record.how, "By doing it");
    }
}

// ---------------------------------------------------------------------------
// SC-04: create_with_reasoning_link_succeeds
// ---------------------------------------------------------------------------

#[test]
fn test_create_with_reasoning_link_succeeds() {
    let (_dir, mut conn, cas) = setup();
    let link_id = create_ettle(&mut conn, &cas, "Link Target");

    let cmd = McpCommand::EttleCreate {
        title: "Linked Ettle".to_string(),
        ettle_id: None,
        why: None,
        what: None,
        how: None,
        reasoning_link_id: Some(link_id.clone()),
        reasoning_link_type: Some("refines".to_string()),
    };
    let result = apply_mcp_command(
        cmd,
        None,
        &mut conn,
        &cas,
        &NoopPolicyProvider,
        &NoopApprovalRouter,
    );
    assert!(
        result.is_ok(),
        "create with reasoning link must succeed: {:?}",
        result.err()
    );

    if let Ok((McpCommandResult::EttleCreate { ettle_id }, _)) = result {
        let record = handle_ettle_get(&conn, &ettle_id).unwrap();
        assert_eq!(record.reasoning_link_id, Some(link_id));
        assert_eq!(record.reasoning_link_type, Some("refines".to_string()));
    }
}

// ---------------------------------------------------------------------------
// SC-05: create_empty_title_fails
// ---------------------------------------------------------------------------

#[test]
fn test_create_empty_title_fails() {
    let (_dir, mut conn, cas) = setup();
    let cmd = McpCommand::EttleCreate {
        title: String::new(),
        ettle_id: None,
        why: None,
        what: None,
        how: None,
        reasoning_link_id: None,
        reasoning_link_type: None,
    };
    let result = apply_mcp_command(
        cmd,
        None,
        &mut conn,
        &cas,
        &NoopPolicyProvider,
        &NoopApprovalRouter,
    );
    assert!(result.is_err(), "empty title must fail");
    assert_eq!(result.unwrap_err().kind(), ExErrorKind::InvalidTitle);
}

// ---------------------------------------------------------------------------
// SC-06: create_rejects_caller_supplied_id
// ---------------------------------------------------------------------------

#[test]
fn test_create_rejects_caller_supplied_id() {
    let (_dir, mut conn, cas) = setup();
    let cmd = McpCommand::EttleCreate {
        title: "Ettle".to_string(),
        ettle_id: Some("ettle:caller-supplied".to_string()),
        why: None,
        what: None,
        how: None,
        reasoning_link_id: None,
        reasoning_link_type: None,
    };
    let result = apply_mcp_command(
        cmd,
        None,
        &mut conn,
        &cas,
        &NoopPolicyProvider,
        &NoopApprovalRouter,
    );
    assert!(result.is_err(), "caller-supplied id must be rejected");
    assert_eq!(result.unwrap_err().kind(), ExErrorKind::InvalidInput);
}

// ---------------------------------------------------------------------------
// SC-07: create_link_without_type_fails
// ---------------------------------------------------------------------------

#[test]
fn test_create_link_without_type_fails() {
    let (_dir, mut conn, cas) = setup();
    let link_id = create_ettle(&mut conn, &cas, "Link Target");

    let cmd = McpCommand::EttleCreate {
        title: "Broken Link".to_string(),
        ettle_id: None,
        why: None,
        what: None,
        how: None,
        reasoning_link_id: Some(link_id),
        reasoning_link_type: None, // Missing type
    };
    let result = apply_mcp_command(
        cmd,
        None,
        &mut conn,
        &cas,
        &NoopPolicyProvider,
        &NoopApprovalRouter,
    );
    assert!(result.is_err(), "link without type must fail");
    assert_eq!(result.unwrap_err().kind(), ExErrorKind::MissingLinkType);
}

// ---------------------------------------------------------------------------
// SC-08: create_type_without_link_fails
// ---------------------------------------------------------------------------

#[test]
fn test_create_type_without_link_fails() {
    let (_dir, mut conn, cas) = setup();
    let cmd = McpCommand::EttleCreate {
        title: "Type Only".to_string(),
        ettle_id: None,
        why: None,
        what: None,
        how: None,
        reasoning_link_id: None, // No link
        reasoning_link_type: Some("refines".to_string()),
    };
    let result = apply_mcp_command(
        cmd,
        None,
        &mut conn,
        &cas,
        &NoopPolicyProvider,
        &NoopApprovalRouter,
    );
    assert!(result.is_err(), "type without link must fail");
    assert_eq!(result.unwrap_err().kind(), ExErrorKind::MissingLinkType);
}

// ---------------------------------------------------------------------------
// SC-09: create_link_to_nonexistent_ettle_fails
// ---------------------------------------------------------------------------

#[test]
fn test_create_link_to_nonexistent_ettle_fails() {
    let (_dir, mut conn, cas) = setup();
    let cmd = McpCommand::EttleCreate {
        title: "Bad Link".to_string(),
        ettle_id: None,
        why: None,
        what: None,
        how: None,
        reasoning_link_id: Some("ettle:does-not-exist".to_string()),
        reasoning_link_type: Some("refines".to_string()),
    };
    let result = apply_mcp_command(
        cmd,
        None,
        &mut conn,
        &cas,
        &NoopPolicyProvider,
        &NoopApprovalRouter,
    );
    assert!(result.is_err(), "link to nonexistent ettle must fail");
    assert_eq!(result.unwrap_err().kind(), ExErrorKind::NotFound);
}

// ---------------------------------------------------------------------------
// SC-10: create_link_to_tombstoned_ettle_fails
// ---------------------------------------------------------------------------

#[test]
fn test_create_link_to_tombstoned_ettle_fails() {
    let (_dir, mut conn, cas) = setup();
    let link_id = create_ettle(&mut conn, &cas, "Link Target");

    // Tombstone the link target
    let tombstone_cmd = McpCommand::EttleTombstone {
        ettle_id: link_id.clone(),
    };
    apply_mcp_command(
        tombstone_cmd,
        None,
        &mut conn,
        &cas,
        &NoopPolicyProvider,
        &NoopApprovalRouter,
    )
    .unwrap();

    // Now try to link to it
    let cmd = McpCommand::EttleCreate {
        title: "Link to Tombstoned".to_string(),
        ettle_id: None,
        why: None,
        what: None,
        how: None,
        reasoning_link_id: Some(link_id),
        reasoning_link_type: Some("refines".to_string()),
    };
    let result = apply_mcp_command(
        cmd,
        None,
        &mut conn,
        &cas,
        &NoopPolicyProvider,
        &NoopApprovalRouter,
    );
    assert!(result.is_err(), "link to tombstoned ettle must fail");
    assert_eq!(result.unwrap_err().kind(), ExErrorKind::AlreadyTombstoned);
}

// ---------------------------------------------------------------------------
// SC-11: create_whitespace_only_title_fails
// ---------------------------------------------------------------------------

#[test]
fn test_create_whitespace_only_title_fails() {
    let (_dir, mut conn, cas) = setup();
    let cmd = McpCommand::EttleCreate {
        title: "   ".to_string(),
        ettle_id: None,
        why: None,
        what: None,
        how: None,
        reasoning_link_id: None,
        reasoning_link_type: None,
    };
    let result = apply_mcp_command(
        cmd,
        None,
        &mut conn,
        &cas,
        &NoopPolicyProvider,
        &NoopApprovalRouter,
    );
    assert!(result.is_err(), "whitespace-only title must fail");
    assert_eq!(result.unwrap_err().kind(), ExErrorKind::InvalidTitle);
}

// ---------------------------------------------------------------------------
// SC-12: get_returns_all_fields
// ---------------------------------------------------------------------------

#[test]
fn test_get_returns_all_fields() {
    let (_dir, mut conn, cas) = setup();
    let cmd = McpCommand::EttleCreate {
        title: "Full".to_string(),
        ettle_id: None,
        why: Some("Because".to_string()),
        what: Some("This".to_string()),
        how: Some("Thus".to_string()),
        reasoning_link_id: None,
        reasoning_link_type: None,
    };
    let (result, _) = apply_mcp_command(
        cmd,
        None,
        &mut conn,
        &cas,
        &NoopPolicyProvider,
        &NoopApprovalRouter,
    )
    .unwrap();
    let McpCommandResult::EttleCreate { ettle_id } = result else {
        panic!()
    };

    let record = handle_ettle_get(&conn, &ettle_id).unwrap();
    assert_eq!(record.id, ettle_id);
    assert_eq!(record.title, "Full");
    assert_eq!(record.why, "Because");
    assert_eq!(record.what, "This");
    assert_eq!(record.how, "Thus");
    assert!(record.reasoning_link_id.is_none());
    assert!(record.reasoning_link_type.is_none());
    assert!(record.tombstoned_at.is_none());
    assert!(!record.created_at.is_empty());
    assert!(!record.updated_at.is_empty());
}

// ---------------------------------------------------------------------------
// SC-13: get_nonexistent_returns_not_found
// ---------------------------------------------------------------------------

#[test]
fn test_get_nonexistent_returns_not_found() {
    let (_dir, conn, _cas) = setup();
    let result = handle_ettle_get(&conn, "ettle:does-not-exist");
    assert!(result.is_err(), "get nonexistent must fail");
    assert_eq!(result.unwrap_err().kind(), ExErrorKind::NotFound);
}

// ---------------------------------------------------------------------------
// SC-14: list_empty_returns_empty_page
// ---------------------------------------------------------------------------

#[test]
fn test_list_empty_returns_empty_page() {
    let (_dir, conn, _cas) = setup();
    let opts = EttleListOpts {
        limit: 50,
        cursor: None,
        include_tombstoned: false,
    };
    let page: EttleListPage = handle_ettle_list(&conn, opts).unwrap();
    assert!(page.items.is_empty(), "empty store must return empty items");
    assert!(
        page.next_cursor.is_none(),
        "empty store must have no cursor"
    );
}

// ---------------------------------------------------------------------------
// SC-15: list_single_ettle
// ---------------------------------------------------------------------------

#[test]
fn test_list_single_ettle() {
    let (_dir, mut conn, cas) = setup();
    let id = create_ettle(&mut conn, &cas, "One Ettle");

    let opts = EttleListOpts {
        limit: 50,
        cursor: None,
        include_tombstoned: false,
    };
    let page = handle_ettle_list(&conn, opts).unwrap();
    assert_eq!(page.items.len(), 1);
    assert_eq!(page.items[0].id, id);
    assert_eq!(page.items[0].title, "One Ettle");
}

// ---------------------------------------------------------------------------
// SC-16: list_pagination_cursor
// ---------------------------------------------------------------------------

#[test]
fn test_list_pagination_cursor() {
    let (_dir, mut conn, cas) = setup();
    for i in 0..5 {
        create_ettle(&mut conn, &cas, &format!("Ettle {}", i));
    }

    let opts = EttleListOpts {
        limit: 3,
        cursor: None,
        include_tombstoned: false,
    };
    let page1 = handle_ettle_list(&conn, opts).unwrap();
    assert_eq!(page1.items.len(), 3, "first page should have 3 items");
    assert!(
        page1.next_cursor.is_some(),
        "should have cursor for next page"
    );

    // Fetch second page — decode the base64 cursor
    let cursor2 = page1
        .next_cursor
        .as_ref()
        .map(|s| ettlex_store::repo::SqliteRepo::decode_ettle_cursor(s).unwrap());
    let opts2 = EttleListOpts {
        limit: 3,
        cursor: cursor2,
        include_tombstoned: false,
    };
    let page2 = handle_ettle_list(&conn, opts2).unwrap();
    assert_eq!(
        page2.items.len(),
        2,
        "second page should have remaining 2 items"
    );

    // IDs should not overlap
    let ids1: Vec<_> = page1.items.iter().map(|e| &e.id).collect();
    let ids2: Vec<_> = page2.items.iter().map(|e| &e.id).collect();
    for id in &ids2 {
        assert!(!ids1.contains(id), "pages must not overlap");
    }
}

// ---------------------------------------------------------------------------
// SC-17: list_limit_zero_fails
// ---------------------------------------------------------------------------

#[test]
fn test_list_limit_zero_fails() {
    let (_dir, conn, _cas) = setup();
    let opts = EttleListOpts {
        limit: 0,
        cursor: None,
        include_tombstoned: false,
    };
    let result = handle_ettle_list(&conn, opts);
    assert!(result.is_err(), "limit=0 must fail");
    assert_eq!(result.unwrap_err().kind(), ExErrorKind::InvalidInput);
}

// ---------------------------------------------------------------------------
// SC-18: list_limit_over_500_fails
// ---------------------------------------------------------------------------

#[test]
fn test_list_limit_over_500_fails() {
    let (_dir, conn, _cas) = setup();
    let opts = EttleListOpts {
        limit: 501,
        cursor: None,
        include_tombstoned: false,
    };
    let result = handle_ettle_list(&conn, opts);
    assert!(result.is_err(), "limit>500 must fail");
    assert_eq!(result.unwrap_err().kind(), ExErrorKind::InvalidInput);
}

// ---------------------------------------------------------------------------
// SC-19: list_invalid_cursor_fails
// ---------------------------------------------------------------------------
// Validates that decoding an invalid base64 cursor string returns InvalidInput.
// The list engine itself accepts struct cursors, so this tests the decode layer.

#[test]
fn test_list_invalid_cursor_fails() {
    // Test that an invalid base64 cursor string is rejected at decode time
    let result = ettlex_store::repo::SqliteRepo::decode_ettle_cursor("this-is-not-valid-base64!!!");
    assert!(result.is_err(), "invalid base64 cursor must fail");
    assert_eq!(result.unwrap_err().kind(), ExErrorKind::InvalidInput);
}

// ---------------------------------------------------------------------------
// SC-20: list_excludes_tombstoned_by_default
// ---------------------------------------------------------------------------

#[test]
fn test_list_excludes_tombstoned_by_default() {
    let (_dir, mut conn, cas) = setup();
    let id1 = create_ettle(&mut conn, &cas, "Active");
    let id2 = create_ettle(&mut conn, &cas, "Tombstoned");

    // Tombstone id2
    apply_mcp_command(
        McpCommand::EttleTombstone {
            ettle_id: id2.clone(),
        },
        None,
        &mut conn,
        &cas,
        &NoopPolicyProvider,
        &NoopApprovalRouter,
    )
    .unwrap();

    let opts = EttleListOpts {
        limit: 50,
        cursor: None,
        include_tombstoned: false,
    };
    let page = handle_ettle_list(&conn, opts).unwrap();
    let ids: Vec<_> = page.items.iter().map(|e| &e.id).collect();
    assert!(ids.contains(&&id1), "active ettle must be in list");
    assert!(
        !ids.contains(&&id2),
        "tombstoned ettle must NOT be in list by default"
    );
}

// ---------------------------------------------------------------------------
// SC-21: list_include_tombstoned_flag
// ---------------------------------------------------------------------------

#[test]
fn test_list_include_tombstoned_flag() {
    let (_dir, mut conn, cas) = setup();
    let id1 = create_ettle(&mut conn, &cas, "Active");
    let id2 = create_ettle(&mut conn, &cas, "Tombstoned");

    apply_mcp_command(
        McpCommand::EttleTombstone {
            ettle_id: id2.clone(),
        },
        None,
        &mut conn,
        &cas,
        &NoopPolicyProvider,
        &NoopApprovalRouter,
    )
    .unwrap();

    let opts = EttleListOpts {
        limit: 50,
        cursor: None,
        include_tombstoned: true,
    };
    let page = handle_ettle_list(&conn, opts).unwrap();
    let ids: Vec<_> = page.items.iter().map(|e| &e.id).collect();
    assert!(ids.contains(&&id1), "active ettle must be in list");
    assert!(
        ids.contains(&&id2),
        "tombstoned ettle must be in list when include_tombstoned=true"
    );
}

// ---------------------------------------------------------------------------
// SC-22: update_title_succeeds
// ---------------------------------------------------------------------------

#[test]
fn test_update_title_succeeds() {
    let (_dir, mut conn, cas) = setup();
    let id = create_ettle(&mut conn, &cas, "Old Title");

    apply_mcp_command(
        McpCommand::EttleUpdate {
            ettle_id: id.clone(),
            title: Some("New Title".to_string()),
            why: None,
            what: None,
            how: None,
            reasoning_link_id: None,
            reasoning_link_type: None,
        },
        None,
        &mut conn,
        &cas,
        &NoopPolicyProvider,
        &NoopApprovalRouter,
    )
    .unwrap();

    let record = handle_ettle_get(&conn, &id).unwrap();
    assert_eq!(record.title, "New Title");
}

// ---------------------------------------------------------------------------
// SC-23: update_why_succeeds
// ---------------------------------------------------------------------------

#[test]
fn test_update_why_succeeds() {
    let (_dir, mut conn, cas) = setup();
    let id = create_ettle(&mut conn, &cas, "Ettle");

    apply_mcp_command(
        McpCommand::EttleUpdate {
            ettle_id: id.clone(),
            title: None,
            why: Some("New Why".to_string()),
            what: None,
            how: None,
            reasoning_link_id: None,
            reasoning_link_type: None,
        },
        None,
        &mut conn,
        &cas,
        &NoopPolicyProvider,
        &NoopApprovalRouter,
    )
    .unwrap();

    let record = handle_ettle_get(&conn, &id).unwrap();
    assert_eq!(record.why, "New Why");
}

// ---------------------------------------------------------------------------
// SC-24: update_what_succeeds
// ---------------------------------------------------------------------------

#[test]
fn test_update_what_succeeds() {
    let (_dir, mut conn, cas) = setup();
    let id = create_ettle(&mut conn, &cas, "Ettle");

    apply_mcp_command(
        McpCommand::EttleUpdate {
            ettle_id: id.clone(),
            title: None,
            why: None,
            what: Some("New What".to_string()),
            how: None,
            reasoning_link_id: None,
            reasoning_link_type: None,
        },
        None,
        &mut conn,
        &cas,
        &NoopPolicyProvider,
        &NoopApprovalRouter,
    )
    .unwrap();

    let record = handle_ettle_get(&conn, &id).unwrap();
    assert_eq!(record.what, "New What");
}

// ---------------------------------------------------------------------------
// SC-25: update_how_succeeds
// ---------------------------------------------------------------------------

#[test]
fn test_update_how_succeeds() {
    let (_dir, mut conn, cas) = setup();
    let id = create_ettle(&mut conn, &cas, "Ettle");

    apply_mcp_command(
        McpCommand::EttleUpdate {
            ettle_id: id.clone(),
            title: None,
            why: None,
            what: None,
            how: Some("New How".to_string()),
            reasoning_link_id: None,
            reasoning_link_type: None,
        },
        None,
        &mut conn,
        &cas,
        &NoopPolicyProvider,
        &NoopApprovalRouter,
    )
    .unwrap();

    let record = handle_ettle_get(&conn, &id).unwrap();
    assert_eq!(record.how, "New How");
}

// ---------------------------------------------------------------------------
// SC-26: update_sets_reasoning_link
// ---------------------------------------------------------------------------

#[test]
fn test_update_sets_reasoning_link() {
    let (_dir, mut conn, cas) = setup();
    let id = create_ettle(&mut conn, &cas, "Ettle");
    let link_id = create_ettle(&mut conn, &cas, "Link Target");

    apply_mcp_command(
        McpCommand::EttleUpdate {
            ettle_id: id.clone(),
            title: None,
            why: None,
            what: None,
            how: None,
            reasoning_link_id: Some(Some(link_id.clone())),
            reasoning_link_type: Some(Some("informs".to_string())),
        },
        None,
        &mut conn,
        &cas,
        &NoopPolicyProvider,
        &NoopApprovalRouter,
    )
    .unwrap();

    let record = handle_ettle_get(&conn, &id).unwrap();
    assert_eq!(record.reasoning_link_id, Some(link_id));
    assert_eq!(record.reasoning_link_type, Some("informs".to_string()));
}

// ---------------------------------------------------------------------------
// SC-27: update_changes_reasoning_link
// ---------------------------------------------------------------------------

#[test]
fn test_update_changes_reasoning_link() {
    let (_dir, mut conn, cas) = setup();
    let link_id1 = create_ettle(&mut conn, &cas, "Link 1");
    let link_id2 = create_ettle(&mut conn, &cas, "Link 2");

    let cmd = McpCommand::EttleCreate {
        title: "Linked".to_string(),
        ettle_id: None,
        why: None,
        what: None,
        how: None,
        reasoning_link_id: Some(link_id1.clone()),
        reasoning_link_type: Some("refines".to_string()),
    };
    let (result, _) = apply_mcp_command(
        cmd,
        None,
        &mut conn,
        &cas,
        &NoopPolicyProvider,
        &NoopApprovalRouter,
    )
    .unwrap();
    let McpCommandResult::EttleCreate { ettle_id } = result else {
        panic!()
    };

    // Change link to link_id2
    apply_mcp_command(
        McpCommand::EttleUpdate {
            ettle_id: ettle_id.clone(),
            title: None,
            why: None,
            what: None,
            how: None,
            reasoning_link_id: Some(Some(link_id2.clone())),
            reasoning_link_type: Some(Some("supersedes".to_string())),
        },
        None,
        &mut conn,
        &cas,
        &NoopPolicyProvider,
        &NoopApprovalRouter,
    )
    .unwrap();

    let record = handle_ettle_get(&conn, &ettle_id).unwrap();
    assert_eq!(record.reasoning_link_id, Some(link_id2));
    assert_eq!(record.reasoning_link_type, Some("supersedes".to_string()));
}

// ---------------------------------------------------------------------------
// SC-28: update_clears_reasoning_link
// ---------------------------------------------------------------------------

#[test]
fn test_update_clears_reasoning_link() {
    let (_dir, mut conn, cas) = setup();
    let link_id = create_ettle(&mut conn, &cas, "Link Target");

    let cmd = McpCommand::EttleCreate {
        title: "Linked".to_string(),
        ettle_id: None,
        why: None,
        what: None,
        how: None,
        reasoning_link_id: Some(link_id),
        reasoning_link_type: Some("refines".to_string()),
    };
    let (result, _) = apply_mcp_command(
        cmd,
        None,
        &mut conn,
        &cas,
        &NoopPolicyProvider,
        &NoopApprovalRouter,
    )
    .unwrap();
    let McpCommandResult::EttleCreate { ettle_id } = result else {
        panic!()
    };

    // Clear the link
    apply_mcp_command(
        McpCommand::EttleUpdate {
            ettle_id: ettle_id.clone(),
            title: None,
            why: None,
            what: None,
            how: None,
            reasoning_link_id: Some(None), // Clear
            reasoning_link_type: None,
        },
        None,
        &mut conn,
        &cas,
        &NoopPolicyProvider,
        &NoopApprovalRouter,
    )
    .unwrap();

    let record = handle_ettle_get(&conn, &ettle_id).unwrap();
    assert!(record.reasoning_link_id.is_none(), "link should be cleared");
    assert!(
        record.reasoning_link_type.is_none(),
        "link type should also be cleared"
    );
}

// ---------------------------------------------------------------------------
// SC-29: update_preserves_unspecified_fields
// ---------------------------------------------------------------------------

#[test]
fn test_update_preserves_unspecified_fields() {
    let (_dir, mut conn, cas) = setup();
    let cmd = McpCommand::EttleCreate {
        title: "Preserve".to_string(),
        ettle_id: None,
        why: Some("Original Why".to_string()),
        what: Some("Original What".to_string()),
        how: Some("Original How".to_string()),
        reasoning_link_id: None,
        reasoning_link_type: None,
    };
    let (result, _) = apply_mcp_command(
        cmd,
        None,
        &mut conn,
        &cas,
        &NoopPolicyProvider,
        &NoopApprovalRouter,
    )
    .unwrap();
    let McpCommandResult::EttleCreate { ettle_id } = result else {
        panic!()
    };

    // Only update title
    apply_mcp_command(
        McpCommand::EttleUpdate {
            ettle_id: ettle_id.clone(),
            title: Some("New Title".to_string()),
            why: None,
            what: None,
            how: None,
            reasoning_link_id: None,
            reasoning_link_type: None,
        },
        None,
        &mut conn,
        &cas,
        &NoopPolicyProvider,
        &NoopApprovalRouter,
    )
    .unwrap();

    let record = handle_ettle_get(&conn, &ettle_id).unwrap();
    assert_eq!(record.title, "New Title");
    assert_eq!(record.why, "Original Why", "why should be preserved");
    assert_eq!(record.what, "Original What", "what should be preserved");
    assert_eq!(record.how, "Original How", "how should be preserved");
}

// ---------------------------------------------------------------------------
// SC-30: update_rejects_self_referential_link
// ---------------------------------------------------------------------------

#[test]
fn test_update_rejects_self_referential_link() {
    let (_dir, mut conn, cas) = setup();
    let id = create_ettle(&mut conn, &cas, "Self Ref");

    let result = apply_mcp_command(
        McpCommand::EttleUpdate {
            ettle_id: id.clone(),
            title: None,
            why: None,
            what: None,
            how: None,
            reasoning_link_id: Some(Some(id.clone())), // Self-reference
            reasoning_link_type: Some(Some("refines".to_string())),
        },
        None,
        &mut conn,
        &cas,
        &NoopPolicyProvider,
        &NoopApprovalRouter,
    );
    assert!(result.is_err(), "self-referential link must fail");
    assert_eq!(result.unwrap_err().kind(), ExErrorKind::SelfReferentialLink);
}

// ---------------------------------------------------------------------------
// SC-31: update_nonexistent_ettle_fails
// ---------------------------------------------------------------------------

#[test]
fn test_update_nonexistent_ettle_fails() {
    let (_dir, mut conn, cas) = setup();
    let result = apply_mcp_command(
        McpCommand::EttleUpdate {
            ettle_id: "ettle:does-not-exist".to_string(),
            title: Some("New Title".to_string()),
            why: None,
            what: None,
            how: None,
            reasoning_link_id: None,
            reasoning_link_type: None,
        },
        None,
        &mut conn,
        &cas,
        &NoopPolicyProvider,
        &NoopApprovalRouter,
    );
    assert!(result.is_err(), "update nonexistent must fail");
    assert_eq!(result.unwrap_err().kind(), ExErrorKind::NotFound);
}

// ---------------------------------------------------------------------------
// SC-32: update_tombstoned_ettle_fails
// ---------------------------------------------------------------------------

#[test]
fn test_update_tombstoned_ettle_fails() {
    let (_dir, mut conn, cas) = setup();
    let id = create_ettle(&mut conn, &cas, "To Tombstone");

    apply_mcp_command(
        McpCommand::EttleTombstone {
            ettle_id: id.clone(),
        },
        None,
        &mut conn,
        &cas,
        &NoopPolicyProvider,
        &NoopApprovalRouter,
    )
    .unwrap();

    let result = apply_mcp_command(
        McpCommand::EttleUpdate {
            ettle_id: id.clone(),
            title: Some("New Title".to_string()),
            why: None,
            what: None,
            how: None,
            reasoning_link_id: None,
            reasoning_link_type: None,
        },
        None,
        &mut conn,
        &cas,
        &NoopPolicyProvider,
        &NoopApprovalRouter,
    );
    assert!(result.is_err(), "update tombstoned must fail");
    assert_eq!(result.unwrap_err().kind(), ExErrorKind::AlreadyTombstoned);
}

// ---------------------------------------------------------------------------
// SC-33: update_empty_update_fails
// ---------------------------------------------------------------------------

#[test]
fn test_update_empty_update_fails() {
    let (_dir, mut conn, cas) = setup();
    let id = create_ettle(&mut conn, &cas, "Ettle");

    let result = apply_mcp_command(
        McpCommand::EttleUpdate {
            ettle_id: id,
            title: None,
            why: None,
            what: None,
            how: None,
            reasoning_link_id: None,
            reasoning_link_type: None,
        },
        None,
        &mut conn,
        &cas,
        &NoopPolicyProvider,
        &NoopApprovalRouter,
    );
    assert!(result.is_err(), "empty update must fail");
    assert_eq!(result.unwrap_err().kind(), ExErrorKind::EmptyUpdate);
}

// ---------------------------------------------------------------------------
// SC-34: update_link_to_nonexistent_fails
// ---------------------------------------------------------------------------

#[test]
fn test_update_link_to_nonexistent_fails() {
    let (_dir, mut conn, cas) = setup();
    let id = create_ettle(&mut conn, &cas, "Ettle");

    let result = apply_mcp_command(
        McpCommand::EttleUpdate {
            ettle_id: id,
            title: None,
            why: None,
            what: None,
            how: None,
            reasoning_link_id: Some(Some("ettle:nonexistent".to_string())),
            reasoning_link_type: Some(Some("refines".to_string())),
        },
        None,
        &mut conn,
        &cas,
        &NoopPolicyProvider,
        &NoopApprovalRouter,
    );
    assert!(result.is_err(), "link to nonexistent ettle must fail");
    assert_eq!(result.unwrap_err().kind(), ExErrorKind::NotFound);
}

// ---------------------------------------------------------------------------
// SC-35: update_link_without_type_fails
// ---------------------------------------------------------------------------

#[test]
fn test_update_link_without_type_fails() {
    let (_dir, mut conn, cas) = setup();
    let id = create_ettle(&mut conn, &cas, "Ettle");
    let link_id = create_ettle(&mut conn, &cas, "Link Target");

    // Set a link with no type (existing record also has no type)
    let result = apply_mcp_command(
        McpCommand::EttleUpdate {
            ettle_id: id,
            title: None,
            why: None,
            what: None,
            how: None,
            reasoning_link_id: Some(Some(link_id)),
            reasoning_link_type: None, // No type supplied and no existing type
        },
        None,
        &mut conn,
        &cas,
        &NoopPolicyProvider,
        &NoopApprovalRouter,
    );
    assert!(result.is_err(), "link without type must fail");
    assert_eq!(result.unwrap_err().kind(), ExErrorKind::MissingLinkType);
}

// ---------------------------------------------------------------------------
// SC-36: tombstone_active_ettle_succeeds
// ---------------------------------------------------------------------------

#[test]
fn test_tombstone_active_ettle_succeeds() {
    let (_dir, mut conn, cas) = setup();
    let id = create_ettle(&mut conn, &cas, "To Tombstone");

    let result = apply_mcp_command(
        McpCommand::EttleTombstone {
            ettle_id: id.clone(),
        },
        None,
        &mut conn,
        &cas,
        &NoopPolicyProvider,
        &NoopApprovalRouter,
    );
    assert!(
        result.is_ok(),
        "tombstone active ettle must succeed: {:?}",
        result.err()
    );

    let record = handle_ettle_get(&conn, &id).unwrap();
    assert!(record.tombstoned_at.is_some(), "tombstoned_at must be set");
}

// ---------------------------------------------------------------------------
// SC-37: tombstone_nonexistent_ettle_fails
// ---------------------------------------------------------------------------

#[test]
fn test_tombstone_nonexistent_ettle_fails() {
    let (_dir, mut conn, cas) = setup();
    let result = apply_mcp_command(
        McpCommand::EttleTombstone {
            ettle_id: "ettle:does-not-exist".to_string(),
        },
        None,
        &mut conn,
        &cas,
        &NoopPolicyProvider,
        &NoopApprovalRouter,
    );
    assert!(result.is_err(), "tombstone nonexistent must fail");
    assert_eq!(result.unwrap_err().kind(), ExErrorKind::NotFound);
}

// ---------------------------------------------------------------------------
// SC-38: tombstone_already_tombstoned_fails
// ---------------------------------------------------------------------------

#[test]
fn test_tombstone_already_tombstoned_fails() {
    let (_dir, mut conn, cas) = setup();
    let id = create_ettle(&mut conn, &cas, "Ettle");

    // First tombstone
    apply_mcp_command(
        McpCommand::EttleTombstone {
            ettle_id: id.clone(),
        },
        None,
        &mut conn,
        &cas,
        &NoopPolicyProvider,
        &NoopApprovalRouter,
    )
    .unwrap();

    // Second tombstone
    let result = apply_mcp_command(
        McpCommand::EttleTombstone { ettle_id: id },
        None,
        &mut conn,
        &cas,
        &NoopPolicyProvider,
        &NoopApprovalRouter,
    );
    assert!(result.is_err(), "double tombstone must fail");
    assert_eq!(result.unwrap_err().kind(), ExErrorKind::AlreadyTombstoned);
}

// ---------------------------------------------------------------------------
// SC-39: tombstone_with_active_dependants_fails
// ---------------------------------------------------------------------------

#[test]
fn test_tombstone_with_active_dependants_fails() {
    let (_dir, mut conn, cas) = setup();
    let target_id = create_ettle(&mut conn, &cas, "Target");

    // Create an ettle that links to target
    apply_mcp_command(
        McpCommand::EttleCreate {
            title: "Dependant".to_string(),
            ettle_id: None,
            why: None,
            what: None,
            how: None,
            reasoning_link_id: Some(target_id.clone()),
            reasoning_link_type: Some("refines".to_string()),
        },
        None,
        &mut conn,
        &cas,
        &NoopPolicyProvider,
        &NoopApprovalRouter,
    )
    .unwrap();

    // Try to tombstone the target while dependant is active
    let result = apply_mcp_command(
        McpCommand::EttleTombstone {
            ettle_id: target_id,
        },
        None,
        &mut conn,
        &cas,
        &NoopPolicyProvider,
        &NoopApprovalRouter,
    );
    assert!(
        result.is_err(),
        "tombstone with active dependants must fail"
    );
    assert_eq!(result.unwrap_err().kind(), ExErrorKind::HasActiveDependants);
}

// ---------------------------------------------------------------------------
// SC-40: tombstone_allows_tombstoned_dependant
// ---------------------------------------------------------------------------

#[test]
fn test_tombstone_allows_tombstoned_dependant() {
    let (_dir, mut conn, cas) = setup();
    let target_id = create_ettle(&mut conn, &cas, "Target");

    // Create an ettle that links to target
    let cmd = McpCommand::EttleCreate {
        title: "Dependant".to_string(),
        ettle_id: None,
        why: None,
        what: None,
        how: None,
        reasoning_link_id: Some(target_id.clone()),
        reasoning_link_type: Some("refines".to_string()),
    };
    let (result, _) = apply_mcp_command(
        cmd,
        None,
        &mut conn,
        &cas,
        &NoopPolicyProvider,
        &NoopApprovalRouter,
    )
    .unwrap();
    let McpCommandResult::EttleCreate {
        ettle_id: dependant_id,
    } = result
    else {
        panic!()
    };

    // Tombstone the dependant first
    apply_mcp_command(
        McpCommand::EttleTombstone {
            ettle_id: dependant_id,
        },
        None,
        &mut conn,
        &cas,
        &NoopPolicyProvider,
        &NoopApprovalRouter,
    )
    .unwrap();

    // Now tombstone the target (all dependants are tombstoned, so this should work)
    let result = apply_mcp_command(
        McpCommand::EttleTombstone {
            ettle_id: target_id,
        },
        None,
        &mut conn,
        &cas,
        &NoopPolicyProvider,
        &NoopApprovalRouter,
    );
    assert!(
        result.is_ok(),
        "tombstone with only tombstoned dependants must succeed: {:?}",
        result.err()
    );
}

// ---------------------------------------------------------------------------
// SC-41: hard_delete_not_exposed
// ---------------------------------------------------------------------------

#[test]
fn test_hard_delete_not_exposed() {
    let src = std::fs::read_to_string("src/commands/mcp_command.rs").unwrap_or_else(|_| {
        // Try with full path
        std::fs::read_to_string(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/src/commands/mcp_command.rs"
        ))
        .unwrap()
    });
    assert!(
        !src.contains("EttleDelete"),
        "EttleDelete must not exist in McpCommand — only soft tombstone is exposed"
    );
}

// ---------------------------------------------------------------------------
// SC-42: occ_correct_version_succeeds
// ---------------------------------------------------------------------------

#[test]
fn test_occ_correct_version_succeeds() {
    let (_dir, mut conn, cas) = setup();

    // Get current state_version (0 initially)
    let sv: u64 = conn
        .query_row("SELECT COUNT(*) FROM mcp_command_log", [], |r| r.get(0))
        .unwrap();

    let cmd = McpCommand::EttleCreate {
        title: "OCC Test".to_string(),
        ettle_id: None,
        why: None,
        what: None,
        how: None,
        reasoning_link_id: None,
        reasoning_link_type: None,
    };
    let result = apply_mcp_command(
        cmd,
        Some(sv), // correct version
        &mut conn,
        &cas,
        &NoopPolicyProvider,
        &NoopApprovalRouter,
    );
    assert!(
        result.is_ok(),
        "correct OCC version must succeed: {:?}",
        result.err()
    );
}

// ---------------------------------------------------------------------------
// SC-43: occ_wrong_version_fails
// ---------------------------------------------------------------------------

#[test]
fn test_occ_wrong_version_fails() {
    let (_dir, mut conn, cas) = setup();
    let cmd = McpCommand::EttleCreate {
        title: "OCC Test".to_string(),
        ettle_id: None,
        why: None,
        what: None,
        how: None,
        reasoning_link_id: None,
        reasoning_link_type: None,
    };
    let result = apply_mcp_command(
        cmd,
        Some(9999), // wrong version
        &mut conn,
        &cas,
        &NoopPolicyProvider,
        &NoopApprovalRouter,
    );
    assert!(result.is_err(), "wrong OCC version must fail");
    assert_eq!(result.unwrap_err().kind(), ExErrorKind::HeadMismatch);
}

// ---------------------------------------------------------------------------
// SC-44: each_mutation_appends_one_provenance_event
// ---------------------------------------------------------------------------

#[test]
fn test_each_mutation_appends_one_provenance_event() {
    let (_dir, mut conn, cas) = setup();

    let count_prov = |conn: &Connection| -> u64 {
        conn.query_row("SELECT COUNT(*) FROM provenance_events", [], |r| r.get(0))
            .unwrap()
    };

    let before = count_prov(&conn);

    // Create
    let id = create_ettle(&mut conn, &cas, "Prov Test");
    assert_eq!(
        count_prov(&conn),
        before + 1,
        "create must append one provenance event"
    );

    // Update
    apply_mcp_command(
        McpCommand::EttleUpdate {
            ettle_id: id.clone(),
            title: Some("Updated".to_string()),
            why: None,
            what: None,
            how: None,
            reasoning_link_id: None,
            reasoning_link_type: None,
        },
        None,
        &mut conn,
        &cas,
        &NoopPolicyProvider,
        &NoopApprovalRouter,
    )
    .unwrap();
    assert_eq!(
        count_prov(&conn),
        before + 2,
        "update must append one provenance event"
    );

    // Tombstone
    apply_mcp_command(
        McpCommand::EttleTombstone { ettle_id: id },
        None,
        &mut conn,
        &cas,
        &NoopPolicyProvider,
        &NoopApprovalRouter,
    )
    .unwrap();
    assert_eq!(
        count_prov(&conn),
        before + 3,
        "tombstone must append one provenance event"
    );
}

// ---------------------------------------------------------------------------
// SC-45: failed_command_no_provenance_event
// ---------------------------------------------------------------------------

#[test]
fn test_failed_command_no_provenance_event() {
    let (_dir, mut conn, cas) = setup();

    let count_prov = |conn: &Connection| -> u64 {
        conn.query_row("SELECT COUNT(*) FROM provenance_events", [], |r| r.get(0))
            .unwrap()
    };

    let before = count_prov(&conn);

    // Attempt a failing command (empty title)
    let _ = apply_mcp_command(
        McpCommand::EttleCreate {
            title: String::new(),
            ettle_id: None,
            why: None,
            what: None,
            how: None,
            reasoning_link_id: None,
            reasoning_link_type: None,
        },
        None,
        &mut conn,
        &cas,
        &NoopPolicyProvider,
        &NoopApprovalRouter,
    );

    assert_eq!(
        count_prov(&conn),
        before,
        "failed command must not append provenance event"
    );
}

// ---------------------------------------------------------------------------
// SC-46: ettle_get_byte_identical
// ---------------------------------------------------------------------------

#[test]
fn test_ettle_get_byte_identical() {
    let (_dir, mut conn, cas) = setup();
    let id = create_ettle(&mut conn, &cas, "Idempotent Get");

    let r1 = handle_ettle_get(&conn, &id).unwrap();
    let r2 = handle_ettle_get(&conn, &id).unwrap();

    assert_eq!(r1.id, r2.id);
    assert_eq!(r1.title, r2.title);
    assert_eq!(r1.why, r2.why);
    assert_eq!(r1.what, r2.what);
    assert_eq!(r1.how, r2.how);
    assert_eq!(r1.created_at, r2.created_at);
    assert_eq!(r1.updated_at, r2.updated_at);
}

// ---------------------------------------------------------------------------
// SC-47: ettle_list_byte_identical
// ---------------------------------------------------------------------------

#[test]
fn test_ettle_list_byte_identical() {
    let (_dir, mut conn, cas) = setup();
    for i in 0..3 {
        create_ettle(&mut conn, &cas, &format!("Ettle {}", i));
    }

    let opts = EttleListOpts {
        limit: 50,
        cursor: None,
        include_tombstoned: false,
    };
    let p1 = handle_ettle_list(&conn, opts.clone()).unwrap();
    let opts2 = EttleListOpts {
        limit: 50,
        cursor: None,
        include_tombstoned: false,
    };
    let p2 = handle_ettle_list(&conn, opts2).unwrap();

    assert_eq!(p1.items.len(), p2.items.len());
    for (a, b) in p1.items.iter().zip(p2.items.iter()) {
        assert_eq!(a.id, b.id);
        assert_eq!(a.title, b.title);
    }
}

// ---------------------------------------------------------------------------
// SC-48: create_large_fields_succeeds
// ---------------------------------------------------------------------------

#[test]
fn test_create_large_fields_succeeds() {
    let (_dir, mut conn, cas) = setup();
    let big = "X".repeat(10_000);
    let cmd = McpCommand::EttleCreate {
        title: "Big Ettle".to_string(),
        ettle_id: None,
        why: Some(big.clone()),
        what: Some(big.clone()),
        how: Some(big.clone()),
        reasoning_link_id: None,
        reasoning_link_type: None,
    };
    let result = apply_mcp_command(
        cmd,
        None,
        &mut conn,
        &cas,
        &NoopPolicyProvider,
        &NoopApprovalRouter,
    );
    assert!(
        result.is_ok(),
        "large fields must succeed: {:?}",
        result.err()
    );

    if let Ok((McpCommandResult::EttleCreate { ettle_id }, _)) = result {
        let record = handle_ettle_get(&conn, &ettle_id).unwrap();
        assert_eq!(record.why.len(), 10_000);
        assert_eq!(record.what.len(), 10_000);
        assert_eq!(record.how.len(), 10_000);
    }
}

// ---------------------------------------------------------------------------
// SC-49: list_max_limit_succeeds
// ---------------------------------------------------------------------------

#[test]
fn test_list_max_limit_succeeds() {
    let (_dir, mut conn, cas) = setup();
    for i in 0..3 {
        create_ettle(&mut conn, &cas, &format!("Ettle {}", i));
    }

    let opts = EttleListOpts {
        limit: 500,
        cursor: None,
        include_tombstoned: false,
    };
    let result = handle_ettle_list(&conn, opts);
    assert!(result.is_ok(), "limit=500 must succeed: {:?}", result.err());
    assert_eq!(result.unwrap().items.len(), 3);
}
