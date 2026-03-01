//! Constraint engine slice 0 tests
//!
//! Tests scenarios S1–S5, S8, S12, S13 for ep:constraint_engine_slice0:0.
//! Also covers Phase 1 error kind additions and Phase 3 engine evaluate().

use ettlex_core::constraint_engine::{ConstraintEvalCtx, ConstraintFamilyStatus};
use ettlex_core::errors::{EttleXError, ExErrorKind};
use ettlex_core::model::{Constraint, Ep, EpConstraintRef, Ettle};
use ettlex_core::ops::constraint_ops;
use ettlex_core::ops::Store;
use serde_json::json;

// ============================================================
// Phase 1: Error kind codes
// ============================================================

#[test]
fn test_constraint_error_kind_codes() {
    let cases = [
        (
            ExErrorKind::InvalidConstraintFamily,
            "ERR_INVALID_CONSTRAINT_FAMILY",
        ),
        (ExErrorKind::AlreadyExists, "ERR_ALREADY_EXISTS"),
        (
            ExErrorKind::ConstraintTombstoned,
            "ERR_CONSTRAINT_TOMBSTONED",
        ),
        (ExErrorKind::DuplicateAttachment, "ERR_DUPLICATE_ATTACHMENT"),
    ];
    for (kind, expected_code) in cases {
        assert_eq!(kind.code(), expected_code, "Wrong code for {:?}", kind);
    }
}

#[test]
fn test_bridge_constraint_already_exists_maps_to_already_exists() {
    let err: ettlex_core::errors::ExError = EttleXError::ConstraintAlreadyExists {
        constraint_id: "c1".to_string(),
    }
    .into();
    assert_eq!(err.kind(), ExErrorKind::AlreadyExists);
    assert_eq!(err.code(), "ERR_ALREADY_EXISTS");
}

#[test]
fn test_bridge_constraint_tombstoned_maps_to_constraint_tombstoned() {
    let err: ettlex_core::errors::ExError = EttleXError::ConstraintTombstoned {
        constraint_id: "c1".to_string(),
    }
    .into();
    assert_eq!(err.kind(), ExErrorKind::ConstraintTombstoned);
    assert_eq!(err.code(), "ERR_CONSTRAINT_TOMBSTONED");
}

#[test]
fn test_bridge_constraint_already_attached_maps_to_duplicate_attachment() {
    let err: ettlex_core::errors::ExError = EttleXError::ConstraintAlreadyAttached {
        constraint_id: "c1".to_string(),
        ep_id: "ep1".to_string(),
    }
    .into();
    assert_eq!(err.kind(), ExErrorKind::DuplicateAttachment);
    assert_eq!(err.code(), "ERR_DUPLICATE_ATTACHMENT");
}

#[test]
fn test_bridge_invalid_constraint_family_maps_to_invalid_constraint_family() {
    let err: ettlex_core::errors::ExError = EttleXError::InvalidConstraintFamily {
        constraint_id: "c1".to_string(),
    }
    .into();
    assert_eq!(err.kind(), ExErrorKind::InvalidConstraintFamily);
    assert_eq!(err.code(), "ERR_INVALID_CONSTRAINT_FAMILY");
}

// ============================================================
// Helpers
// ============================================================

fn make_store_with_ep() -> (Store, String, String) {
    let mut store = Store::new();

    let ettle = Ettle::new("ettle-1".to_string(), "Test Ettle".to_string());
    store.insert_ettle(ettle);

    let ep = Ep::new(
        "ep-1".to_string(),
        "ettle-1".to_string(),
        0,
        false,
        "why".to_string(),
        "what".to_string(),
        "how".to_string(),
    );
    store.insert_ep(ep);

    (store, "ettle-1".to_string(), "ep-1".to_string())
}

// ============================================================
// S1: Create unknown family succeeds
// ============================================================

#[test]
fn s1_create_constraint_unknown_family_succeeds() {
    let mut store = Store::new();

    let result = constraint_ops::create_constraint(
        &mut store,
        "c1".to_string(),
        "UnknownFamily".to_string(),
        "SomeKind".to_string(),
        "EP".to_string(),
        json!({"key": "value"}),
    );

    assert!(
        result.is_ok(),
        "Unknown family should be accepted: {:?}",
        result
    );

    let constraint = store.get_constraint("c1").unwrap();
    assert_eq!(constraint.family, "UnknownFamily");
}

// ============================================================
// S2: Create rejects empty family
// ============================================================

#[test]
fn s2_create_constraint_rejects_empty_family() {
    let mut store = Store::new();

    let result = constraint_ops::create_constraint(
        &mut store,
        "c1".to_string(),
        "".to_string(), // empty family
        "SomeKind".to_string(),
        "EP".to_string(),
        json!({"key": "value"}),
    );

    assert!(result.is_err(), "Empty family should be rejected");

    let ex_err: ettlex_core::errors::ExError = result.unwrap_err().into();
    assert_eq!(
        ex_err.kind(),
        ExErrorKind::InvalidConstraintFamily,
        "Should map to InvalidConstraintFamily"
    );
}

// ============================================================
// S3: Create rejects duplicate id
// ============================================================

#[test]
fn s3_create_constraint_rejects_duplicate_id() {
    let mut store = Store::new();

    constraint_ops::create_constraint(
        &mut store,
        "c1".to_string(),
        "ABB".to_string(),
        "Rule".to_string(),
        "EP".to_string(),
        json!({"rule": "first"}),
    )
    .unwrap();

    let result = constraint_ops::create_constraint(
        &mut store,
        "c1".to_string(), // duplicate id
        "SBB".to_string(),
        "OtherRule".to_string(),
        "EP".to_string(),
        json!({"rule": "second"}),
    );

    assert!(
        result.is_err(),
        "Duplicate constraint ID should be rejected"
    );

    let ex_err: ettlex_core::errors::ExError = result.unwrap_err().into();
    assert_eq!(
        ex_err.kind(),
        ExErrorKind::AlreadyExists,
        "Should map to AlreadyExists"
    );
}

// ============================================================
// S4: Update changes payload_digest deterministically
// ============================================================

#[test]
fn s4_update_constraint_changes_digest_deterministically() {
    let mut store = Store::new();

    constraint_ops::create_constraint(
        &mut store,
        "c1".to_string(),
        "ABB".to_string(),
        "Rule".to_string(),
        "EP".to_string(),
        json!({"rule": "old"}),
    )
    .unwrap();

    let old_digest = store.get_constraint("c1").unwrap().payload_digest.clone();

    let new_payload = json!({"rule": "new"});
    constraint_ops::update_constraint(&mut store, "c1", new_payload.clone()).unwrap();

    let new_digest = store.get_constraint("c1").unwrap().payload_digest.clone();
    assert_ne!(
        old_digest, new_digest,
        "Digest should change on payload update"
    );

    // Deterministic: same payload → same digest
    let c2 = Constraint::new(
        "c2".to_string(),
        "ABB".to_string(),
        "Rule".to_string(),
        "EP".to_string(),
        new_payload,
    );
    assert_eq!(
        new_digest, c2.payload_digest,
        "Digest must be deterministic"
    );
}

// ============================================================
// S5: Tombstone prevents attach, reads preserved
// ============================================================

#[test]
fn s5_tombstone_prevents_attachment_preserves_history() {
    let (mut store, _ettle_id, ep_id) = make_store_with_ep();

    constraint_ops::create_constraint(
        &mut store,
        "c1".to_string(),
        "ABB".to_string(),
        "Rule".to_string(),
        "EP".to_string(),
        json!({"rule": "test"}),
    )
    .unwrap();

    constraint_ops::tombstone_constraint(&mut store, "c1").unwrap();

    // Attach should fail with ConstraintTombstoned
    let result =
        constraint_ops::attach_constraint_to_ep(&mut store, ep_id.clone(), "c1".to_string(), 0);
    assert!(result.is_err(), "Attach after tombstone should fail");

    let ex_err: ettlex_core::errors::ExError = result.unwrap_err().into();
    assert_eq!(
        ex_err.kind(),
        ExErrorKind::ConstraintTombstoned,
        "Should be ConstraintTombstoned, got {:?}",
        ex_err.kind()
    );

    // History preserved: constraint still accessible via including-deleted method
    let raw = store.get_constraint_including_deleted("c1");
    assert!(
        raw.is_ok(),
        "Tombstoned constraint should be retrievable via including_deleted"
    );
    assert!(
        raw.unwrap().is_deleted(),
        "Constraint should be marked deleted"
    );
}

// ============================================================
// S8: Duplicate attachment rejected
// ============================================================

#[test]
fn s8_duplicate_attachment_rejected() {
    let (mut store, _ettle_id, ep_id) = make_store_with_ep();

    constraint_ops::create_constraint(
        &mut store,
        "c1".to_string(),
        "ABB".to_string(),
        "Rule".to_string(),
        "EP".to_string(),
        json!({"rule": "test"}),
    )
    .unwrap();

    constraint_ops::attach_constraint_to_ep(&mut store, ep_id.clone(), "c1".to_string(), 0)
        .unwrap();

    let result =
        constraint_ops::attach_constraint_to_ep(&mut store, ep_id.clone(), "c1".to_string(), 1);
    assert!(result.is_err(), "Duplicate attach should fail");

    let ex_err: ettlex_core::errors::ExError = result.unwrap_err().into();
    assert_eq!(
        ex_err.kind(),
        ExErrorKind::DuplicateAttachment,
        "Should be DuplicateAttachment"
    );
}

// ============================================================
// S12: Attach rejects unknown constraint id
// ============================================================

#[test]
fn s12_attach_rejects_unknown_constraint_id() {
    let (mut store, _ettle_id, ep_id) = make_store_with_ep();

    let result = constraint_ops::attach_constraint_to_ep(
        &mut store,
        ep_id.clone(),
        "nonexistent-constraint".to_string(),
        0,
    );

    assert!(
        result.is_err(),
        "Attach with unknown constraint should fail"
    );

    let ex_err: ettlex_core::errors::ExError = result.unwrap_err().into();
    assert_eq!(
        ex_err.kind(),
        ExErrorKind::NotFound,
        "Should be NotFound for unknown constraint"
    );
}

// ============================================================
// S13: Attach rejects unknown ep id
// ============================================================

#[test]
fn s13_attach_rejects_unknown_ep_id() {
    let mut store = Store::new();

    constraint_ops::create_constraint(
        &mut store,
        "c1".to_string(),
        "ABB".to_string(),
        "Rule".to_string(),
        "EP".to_string(),
        json!({"rule": "test"}),
    )
    .unwrap();

    let result = constraint_ops::attach_constraint_to_ep(
        &mut store,
        "nonexistent-ep".to_string(),
        "c1".to_string(),
        0,
    );

    assert!(result.is_err(), "Attach to unknown EP should fail");

    let ex_err: ettlex_core::errors::ExError = result.unwrap_err().into();
    assert_eq!(
        ex_err.kind(),
        ExErrorKind::NotFound,
        "Should be NotFound for unknown EP"
    );
}

// ============================================================
// Phase 3: constraint_engine::evaluate() tests
// ============================================================

fn make_store_for_engine() -> (Store, String, String) {
    let mut store = Store::new();

    let ettle = Ettle::new("ettle-1".to_string(), "Root".to_string());
    store.insert_ettle(ettle);

    let ep = Ep::new(
        "ep-1".to_string(),
        "ettle-1".to_string(),
        0,
        false,
        "why".to_string(),
        "what".to_string(),
        "how".to_string(),
    );
    store.insert_ep(ep);

    (store, "ettle-1".to_string(), "ep-1".to_string())
}

#[test]
fn test_engine_evaluate_returns_uncomputed_status() {
    let (mut store, _ettle_id, ep_id) = make_store_for_engine();

    constraint_ops::create_constraint(
        &mut store,
        "c1".to_string(),
        "ABB".to_string(),
        "Rule".to_string(),
        "EP".to_string(),
        json!({"rule": "test"}),
    )
    .unwrap();
    store.insert_ep_constraint_ref(EpConstraintRef::new(ep_id.clone(), "c1".to_string(), 0));

    let ctx = ConstraintEvalCtx {
        leaf_ep_id: ep_id.clone(),
        ept_ep_ids: vec![ep_id.clone()],
        policy_ref: "policy/default@0".to_string(),
        profile_ref: "profile/default@0".to_string(),
    };

    let result = ettlex_core::constraint_engine::evaluate(&ctx, &store);
    assert!(result.is_ok(), "evaluate() should succeed: {:?}", result);

    let eval = result.unwrap();
    assert_eq!(eval.declared_refs.len(), 1);
    assert_eq!(eval.declared_refs[0].constraint_id, "c1");
    assert_eq!(eval.declared_refs[0].family, "ABB");

    // Family evaluation status must be UNCOMPUTED
    let abb_eval = eval
        .families
        .get("ABB")
        .expect("ABB family should be present");
    assert_eq!(abb_eval.status, ConstraintFamilyStatus::Uncomputed);

    assert!(!eval.constraints_digest.is_empty());
    assert_eq!(eval.constraints_digest.len(), 64);
}

#[test]
fn test_engine_evaluate_empty_ept_returns_empty() {
    let store = Store::new();

    let ctx = ConstraintEvalCtx {
        leaf_ep_id: "ep-1".to_string(),
        ept_ep_ids: vec![],
        policy_ref: "policy/default@0".to_string(),
        profile_ref: "profile/default@0".to_string(),
    };

    let result = ettlex_core::constraint_engine::evaluate(&ctx, &store);
    assert!(result.is_ok());

    let eval = result.unwrap();
    assert!(eval.declared_refs.is_empty());
    assert!(eval.families.is_empty());
    assert!(!eval.constraints_digest.is_empty());
}

#[test]
fn test_engine_evaluate_deduplicates_across_eps() {
    let (mut store, _ettle_id, ep1_id) = make_store_for_engine();

    let ep2 = Ep::new(
        "ep-2".to_string(),
        "ettle-1".to_string(),
        1,
        false,
        "why2".to_string(),
        "what2".to_string(),
        "how2".to_string(),
    );
    store.insert_ep(ep2);

    // Create one constraint, attach to both EPs
    constraint_ops::create_constraint(
        &mut store,
        "c1".to_string(),
        "ABB".to_string(),
        "Rule".to_string(),
        "EP".to_string(),
        json!({"rule": "shared"}),
    )
    .unwrap();
    store.insert_ep_constraint_ref(EpConstraintRef::new(ep1_id.clone(), "c1".to_string(), 0));
    store.insert_ep_constraint_ref(EpConstraintRef::new(
        "ep-2".to_string(),
        "c1".to_string(),
        0,
    ));

    let ctx = ConstraintEvalCtx {
        leaf_ep_id: ep1_id.clone(),
        ept_ep_ids: vec![ep1_id.clone(), "ep-2".to_string()],
        policy_ref: "policy/default@0".to_string(),
        profile_ref: "profile/default@0".to_string(),
    };

    let eval = ettlex_core::constraint_engine::evaluate(&ctx, &store).unwrap();
    // Should be deduplicated to 1 even though attached to 2 EPs
    assert_eq!(
        eval.declared_refs.len(),
        1,
        "Same constraint in 2 EPs should deduplicate"
    );
}

#[test]
fn test_engine_evaluate_digest_deterministic() {
    let (mut store, _ettle_id, ep_id) = make_store_for_engine();

    constraint_ops::create_constraint(
        &mut store,
        "c1".to_string(),
        "ABB".to_string(),
        "Rule".to_string(),
        "EP".to_string(),
        json!({"rule": "det"}),
    )
    .unwrap();
    store.insert_ep_constraint_ref(EpConstraintRef::new(ep_id.clone(), "c1".to_string(), 0));

    let ctx = ConstraintEvalCtx {
        leaf_ep_id: ep_id.clone(),
        ept_ep_ids: vec![ep_id.clone()],
        policy_ref: "policy/default@0".to_string(),
        profile_ref: "profile/default@0".to_string(),
    };

    let eval1 = ettlex_core::constraint_engine::evaluate(&ctx, &store).unwrap();
    let eval2 = ettlex_core::constraint_engine::evaluate(&ctx, &store).unwrap();

    assert_eq!(eval1.constraints_digest, eval2.constraints_digest);
}
