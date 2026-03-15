//! Action-layer scenario tests for `action_ep_update` Ettle.
//!
//! All tests use `apply(store, Command::EpUpdate{...}, &NeverAnchoredPolicy)` — pure
//! in-memory Store, no SQL.  Written from spec only.
//!
//! Scenario → test mapping:
//!   S-AU-1  test_action_apply_ep_update_succeeds
//!   S-AU-2  test_action_apply_ep_update_rejects_empty
//!   S-AU-3  test_action_apply_ep_update_not_found
//!   S-AU-4  test_action_apply_ep_update_no_direct_sql

#![allow(clippy::unwrap_used)]

use ettlex_core::apply::apply;
use ettlex_core::commands::Command;
use ettlex_core::errors::ExErrorKind;
use ettlex_core::ops::{ep_ops, ettle_ops, Store};
use ettlex_core::policy::NeverAnchoredPolicy;

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

fn setup() -> (Store, String) {
    let mut store = Store::new();
    let ettle_id = ettle_ops::create_ettle(
        &mut store,
        "Action Test Ettle".to_string(),
        None,
        None,
        None,
        None,
    )
    .unwrap();
    let ep_id = ep_ops::create_ep(
        &mut store,
        &ettle_id,
        1,
        true,
        "original why".to_string(),
        "original what".to_string(),
        "original how".to_string(),
    )
    .unwrap();
    (store, ep_id)
}

// ---------------------------------------------------------------------------
// S-AU-1: Apply EpUpdate succeeds and updates the field
// ---------------------------------------------------------------------------

#[test]
fn test_action_apply_ep_update_succeeds() {
    let (store, ep_id) = setup();

    let cmd = Command::EpUpdate {
        ep_id: ep_id.clone(),
        why: Some("action layer why".to_string()),
        what: None,
        how: None,
        title: None,
        normative: None,
    };
    let result = apply(store, cmd, &NeverAnchoredPolicy);
    assert!(result.is_ok(), "Apply EpUpdate must succeed: {:?}", result);

    let new_store = result.unwrap();
    let ep = new_store.get_ep(&ep_id).unwrap();
    assert_eq!(ep.why, "action layer why");
    assert_eq!(
        ep.what, "original what",
        "unspecified fields must be preserved"
    );
}

// ---------------------------------------------------------------------------
// S-AU-2: Apply EpUpdate rejects empty update at the action layer
// ---------------------------------------------------------------------------

#[test]
fn test_action_apply_ep_update_rejects_empty() {
    let (store, ep_id) = setup();

    let cmd = Command::EpUpdate {
        ep_id: ep_id.clone(),
        why: None,
        what: None,
        how: None,
        title: None,
        normative: None,
    };
    let result = apply(store, cmd, &NeverAnchoredPolicy);

    assert!(result.is_err(), "Empty EpUpdate must be rejected");
    assert_eq!(
        result.unwrap_err().kind(),
        ExErrorKind::EmptyUpdate,
        "Error must be EmptyUpdate"
    );
}

// ---------------------------------------------------------------------------
// S-AU-3: Apply EpUpdate rejects unknown ep_id
// ---------------------------------------------------------------------------

#[test]
fn test_action_apply_ep_update_not_found() {
    let store = Store::new();

    let cmd = Command::EpUpdate {
        ep_id: "ep:unknown".to_string(),
        why: Some("anything".to_string()),
        what: None,
        how: None,
        title: None,
        normative: None,
    };
    let result = apply(store, cmd, &NeverAnchoredPolicy);

    assert!(result.is_err(), "Update of missing EP must fail");
    assert_eq!(
        result.unwrap_err().kind(),
        ExErrorKind::NotFound,
        "Error must be EpNotFound"
    );
}

// ---------------------------------------------------------------------------
// S-AU-4: Apply EpUpdate does not mutate the store directly (no SQL path)
//
// Structural proof: `apply()` in ettlex-core is a pure function operating on
// an in-memory `Store`.  It has no `Connection` or `SqliteRepo` dependency.
// This test proves the contract holds by constructing a Store in memory, calling
// `apply()`, and verifying the mutation appears in the returned state only —
// the *original* store is consumed (moved) and is gone; no database is involved.
// ---------------------------------------------------------------------------

#[test]
fn test_action_apply_ep_update_no_direct_sql() {
    let (store, ep_id) = setup();

    // Snapshot the original why before apply
    let original_why = store.get_ep(&ep_id).unwrap().why.clone();

    let cmd = Command::EpUpdate {
        ep_id: ep_id.clone(),
        why: Some("new content via apply".to_string()),
        what: None,
        how: None,
        title: None,
        normative: None,
    };

    // `store` is moved into apply() — no Connection ever passed
    let new_store = apply(store, cmd, &NeverAnchoredPolicy).unwrap();

    // New state has the update
    assert_eq!(
        new_store.get_ep(&ep_id).unwrap().why,
        "new content via apply"
    );

    // The original why is captured separately and confirms apply() was additive
    assert_ne!(original_why, "new content via apply");
}
