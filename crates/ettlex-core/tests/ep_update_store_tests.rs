//! Store-layer scenario tests for `store_ep_update` Ettle.
//!
//! All tests use `apply(store, Command::EpUpdate{...}, &NeverAnchoredPolicy)` — pure
//! in-memory Store, no SQL.  Written from spec only.
//!
//! Scenario → test mapping:
//!   S-SU-1  test_ep_update_replaces_why_only
//!   S-SU-2  test_ep_update_replaces_all_fields
//!   S-SU-3  test_ep_update_sets_title
//!   S-SU-4  test_ep_update_rejects_empty_update          [RED until EmptyUpdate added to ep_ops]
//!   S-SU-5  test_ep_update_not_found
//!   S-SU-7  test_ep_update_sets_updated_at

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
    let ettle_id =
        ettle_ops::create_ettle(&mut store, "Test Ettle".to_string(), None, None, None, None)
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
// S-SU-1: EpUpdate replaces why field only; other fields preserved
// ---------------------------------------------------------------------------

#[test]
fn test_ep_update_replaces_why_only() {
    let (store, ep_id) = setup();

    let cmd = Command::EpUpdate {
        ep_id: ep_id.clone(),
        why: Some("updated why".to_string()),
        what: None,
        how: None,
        title: None,
        normative: None,
    };
    let new_store = apply(store, cmd, &NeverAnchoredPolicy).unwrap();
    let ep = new_store.get_ep(&ep_id).unwrap();

    assert_eq!(ep.why, "updated why", "why should be updated");
    assert_eq!(ep.what, "original what", "what must be preserved");
    assert_eq!(ep.how, "original how", "how must be preserved");
}

// ---------------------------------------------------------------------------
// S-SU-2: EpUpdate replaces all content fields
// ---------------------------------------------------------------------------

#[test]
fn test_ep_update_replaces_all_fields() {
    let (store, ep_id) = setup();

    let cmd = Command::EpUpdate {
        ep_id: ep_id.clone(),
        why: Some("new why".to_string()),
        what: Some("new what".to_string()),
        how: Some("new how".to_string()),
        title: None,
        normative: None,
    };
    let new_store = apply(store, cmd, &NeverAnchoredPolicy).unwrap();
    let ep = new_store.get_ep(&ep_id).unwrap();

    assert_eq!(ep.why, "new why");
    assert_eq!(ep.what, "new what");
    assert_eq!(ep.how, "new how");
}

// ---------------------------------------------------------------------------
// S-SU-3: EpUpdate sets title on an EP that had none
// ---------------------------------------------------------------------------

#[test]
fn test_ep_update_sets_title() {
    let (store, ep_id) = setup();

    // Verify no title initially
    assert!(
        store.get_ep(&ep_id).unwrap().title.is_none(),
        "EP should start with no title"
    );

    let cmd = Command::EpUpdate {
        ep_id: ep_id.clone(),
        why: None,
        what: None,
        how: None,
        title: Some("Storage Spine Anchor".to_string()),
        normative: None,
    };
    let new_store = apply(store, cmd, &NeverAnchoredPolicy).unwrap();
    let ep = new_store.get_ep(&ep_id).unwrap();

    assert_eq!(
        ep.title,
        Some("Storage Spine Anchor".to_string()),
        "title should be set"
    );
    // Other fields preserved
    assert_eq!(ep.why, "original why");
    assert_eq!(ep.what, "original what");
    assert_eq!(ep.how, "original how");
}

// ---------------------------------------------------------------------------
// S-SU-4: EpUpdate rejects update where no field is supplied [RED gate]
// ---------------------------------------------------------------------------

#[test]
fn test_ep_update_rejects_empty_update() {
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

    assert!(result.is_err(), "Empty update must return an error");
    assert_eq!(
        result.unwrap_err().kind(),
        ExErrorKind::EmptyUpdate,
        "Error must be EmptyUpdate"
    );
}

// ---------------------------------------------------------------------------
// S-SU-5: EpUpdate on non-existent ep_id returns EpNotFound
// ---------------------------------------------------------------------------

#[test]
fn test_ep_update_not_found() {
    let store = Store::new();

    let cmd = Command::EpUpdate {
        ep_id: "ep:does-not-exist".to_string(),
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
// S-SU-7: EpUpdate sets updated_at to a value >= original updated_at
// ---------------------------------------------------------------------------

#[test]
fn test_ep_update_sets_updated_at() {
    let (store, ep_id) = setup();
    let t1 = store.get_ep(&ep_id).unwrap().updated_at;

    let cmd = Command::EpUpdate {
        ep_id: ep_id.clone(),
        why: Some("changed why".to_string()),
        what: None,
        how: None,
        title: None,
        normative: None,
    };
    let new_store = apply(store, cmd, &NeverAnchoredPolicy).unwrap();
    let t2 = new_store.get_ep(&ep_id).unwrap().updated_at;

    assert!(
        t2 >= t1,
        "updated_at ({:?}) must be >= previous ({:?})",
        t2,
        t1
    );
}
