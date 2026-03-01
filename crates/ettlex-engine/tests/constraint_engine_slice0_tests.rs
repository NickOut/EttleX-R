//! Constraint engine slice 0 integration tests (engine crate)
//!
//! Tests scenarios S6, S7, S9, S10, S11, S14, S15 for ep:constraint_engine_slice0:0.
//! Uses in-memory Store + generate_manifest for fast, deterministic validation.

use ettlex_core::constraint_engine::ConstraintFamilyStatus;
use ettlex_core::model::{Ep, Ettle};
use ettlex_core::ops::constraint_ops;
use ettlex_core::ops::Store;
use ettlex_core::snapshot::manifest::generate_manifest;
use serde_json::json;

// ============================================================
// Helpers
// ============================================================

fn make_flat_store() -> (Store, String, String) {
    let mut store = Store::new();

    let ettle = Ettle::new("ettle:root".to_string(), "Root Ettle".to_string());
    store.insert_ettle(ettle);

    let ep = Ep::new(
        "ep:root:0".to_string(),
        "ettle:root".to_string(),
        0,
        true,
        "why".to_string(),
        "what".to_string(),
        "how".to_string(),
    );
    store.insert_ep(ep);

    (store, "ettle:root".to_string(), "ep:root:0".to_string())
}

fn make_two_ep_store() -> (Store, String, String, String) {
    let mut store = Store::new();

    let ettle = Ettle::new("ettle:root".to_string(), "Root".to_string());
    store.insert_ettle(ettle);

    let ep0 = Ep::new(
        "ep:root:0".to_string(),
        "ettle:root".to_string(),
        0,
        true,
        "why0".to_string(),
        "what0".to_string(),
        "how0".to_string(),
    );
    store.insert_ep(ep0);

    let ep1 = Ep::new(
        "ep:root:1".to_string(),
        "ettle:root".to_string(),
        1,
        true,
        "why1".to_string(),
        "what1".to_string(),
        "how1".to_string(),
    );
    store.insert_ep(ep1);

    (
        store,
        "ettle:root".to_string(),
        "ep:root:0".to_string(),
        "ep:root:1".to_string(),
    )
}

fn commit_manifest(
    store: &Store,
    ept: Vec<String>,
) -> ettlex_core::snapshot::manifest::SnapshotManifest {
    generate_manifest(
        ept,
        "policy/default@0".to_string(),
        "profile/default@0".to_string(),
        "ettle:root".to_string(),
        "0001".to_string(),
        None,
        store,
    )
    .unwrap()
}

// ============================================================
// S6: Attach → appears in manifest declared_refs
// ============================================================

#[test]
fn s6_attach_appears_in_manifest_declared_refs() {
    let (mut store, _ettle_id, ep_id) = make_flat_store();

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

    let manifest = commit_manifest(&store, vec![ep_id]);

    // c1 should appear in declared_refs as plain ID
    assert!(
        manifest
            .constraints
            .declared_refs
            .contains(&"c1".to_string()),
        "c1 should appear in declared_refs"
    );
    assert_eq!(manifest.constraints.declared_refs.len(), 1);
    assert!(manifest.constraints.families.contains_key("ABB"));
}

// ============================================================
// S7: Attach to EP not in EPT → no manifest entry
// ============================================================

#[test]
fn s7_attach_ep_outside_ept_not_in_manifest() {
    let (mut store, _ettle_id, ep0_id, ep1_id) = make_two_ep_store();

    constraint_ops::create_constraint(
        &mut store,
        "c1".to_string(),
        "ABB".to_string(),
        "Rule".to_string(),
        "EP".to_string(),
        json!({"rule": "test"}),
    )
    .unwrap();

    // Attach c1 to ep1 (not in EPT)
    constraint_ops::attach_constraint_to_ep(&mut store, ep1_id, "c1".to_string(), 0).unwrap();

    // EPT contains only ep0
    let manifest = commit_manifest(&store, vec![ep0_id]);

    // c1 should NOT appear — it's on ep1, not in EPT
    assert!(
        !manifest
            .constraints
            .declared_refs
            .contains(&"c1".to_string()),
        "c1 should NOT appear in declared_refs when EP is outside EPT"
    );
    assert!(manifest.constraints.declared_refs.is_empty());
}

// ============================================================
// S9: Detach removes from declared_refs
// ============================================================

#[test]
fn s9_detach_removes_from_declared_refs() {
    let (mut store, _ettle_id, ep_id) = make_flat_store();

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

    // Verify c1 is in manifest before detach
    let manifest_before = commit_manifest(&store, vec![ep_id.clone()]);
    assert!(manifest_before
        .constraints
        .declared_refs
        .contains(&"c1".to_string()));

    // Detach and verify removed
    constraint_ops::detach_constraint_from_ep(&mut store, &ep_id, "c1").unwrap();

    let manifest_after = commit_manifest(&store, vec![ep_id]);
    assert!(
        !manifest_after
            .constraints
            .declared_refs
            .contains(&"c1".to_string()),
        "c1 should be gone from declared_refs after detach"
    );
    assert!(manifest_after.constraints.declared_refs.is_empty());
}

// ============================================================
// S10: declared_refs ordering is deterministic (ordinal-based)
// ============================================================

#[test]
fn s10_declared_refs_ordering_is_deterministic() {
    let (mut store, _ettle_id, ep_id) = make_flat_store();

    // Create constraints in non-alphabetical order, assign specific ordinals
    let specs = [
        ("c-zulu", 0i32),
        ("c-alpha", 1),
        ("c-mike", 2),
        ("c-bravo", 3),
    ];

    for (id, ord) in &specs {
        constraint_ops::create_constraint(
            &mut store,
            id.to_string(),
            "Family".to_string(),
            "Kind".to_string(),
            "EP".to_string(),
            json!({"id": id}),
        )
        .unwrap();
        constraint_ops::attach_constraint_to_ep(&mut store, ep_id.clone(), id.to_string(), *ord)
            .unwrap();
    }

    let manifest = commit_manifest(&store, vec![ep_id]);

    // Should be in ordinal order, not alphabetical
    let refs = &manifest.constraints.declared_refs;
    assert_eq!(refs.len(), 4);
    assert_eq!(refs[0], "c-zulu"); // ordinal 0
    assert_eq!(refs[1], "c-alpha"); // ordinal 1
    assert_eq!(refs[2], "c-mike"); // ordinal 2
    assert_eq!(refs[3], "c-bravo"); // ordinal 3

    // Two calls produce identical ordering
    let manifest2 = commit_manifest(&store, vec!["ep:root:0".to_string()]);
    assert_eq!(
        manifest.constraints.declared_refs,
        manifest2.constraints.declared_refs
    );
}

// ============================================================
// S11: constraints_digest changes iff set changes
// ============================================================

#[test]
fn s11_constraints_digest_changes_iff_set_changes() {
    let (mut store, _ettle_id, ep_id) = make_flat_store();

    constraint_ops::create_constraint(
        &mut store,
        "c1".to_string(),
        "ABB".to_string(),
        "Rule".to_string(),
        "EP".to_string(),
        json!({"rule": "test"}),
    )
    .unwrap();

    // No attachment — digest with empty constraint set
    let d_empty = commit_manifest(&store, vec![ep_id.clone()])
        .constraints
        .constraints_digest;

    // Attach c1 — digest should change
    constraint_ops::attach_constraint_to_ep(&mut store, ep_id.clone(), "c1".to_string(), 0)
        .unwrap();

    let d_with_c1 = commit_manifest(&store, vec![ep_id.clone()])
        .constraints
        .constraints_digest;

    assert_ne!(
        d_empty, d_with_c1,
        "Digest should change when constraint set changes"
    );

    // Add and attach c2 — digest changes again
    constraint_ops::create_constraint(
        &mut store,
        "c2".to_string(),
        "ABB".to_string(),
        "Rule".to_string(),
        "EP".to_string(),
        json!({"rule": "other"}),
    )
    .unwrap();
    constraint_ops::attach_constraint_to_ep(&mut store, ep_id.clone(), "c2".to_string(), 1)
        .unwrap();

    let d_with_c1_c2 = commit_manifest(&store, vec![ep_id.clone()])
        .constraints
        .constraints_digest;

    assert_ne!(
        d_with_c1, d_with_c1_c2,
        "Digest changes when constraint added"
    );

    // Detach c2 — digest should return to the c1-only value
    constraint_ops::detach_constraint_from_ep(&mut store, &ep_id, "c2").unwrap();
    let d_back_to_c1 = commit_manifest(&store, vec![ep_id])
        .constraints
        .constraints_digest;

    assert_eq!(
        d_with_c1, d_back_to_c1,
        "Digest returns to original when set is restored"
    );
}

// ============================================================
// S14: Evaluate returns UNCOMPUTED for ABB/SBB families
// ============================================================

#[test]
fn s14_evaluate_returns_uncomputed_for_known_families() {
    let (mut store, _ettle_id, ep_id) = make_flat_store();

    // Attach both ABB and SBB constraints
    for (id, family) in [("c-abb", "ABB"), ("c-sbb", "SBB")] {
        constraint_ops::create_constraint(
            &mut store,
            id.to_string(),
            family.to_string(),
            "Rule".to_string(),
            "EP".to_string(),
            json!({"family": family}),
        )
        .unwrap();
        constraint_ops::attach_constraint_to_ep(&mut store, ep_id.clone(), id.to_string(), 0)
            .unwrap();
    }

    let manifest = commit_manifest(&store, vec![ep_id]);

    // Both families should exist with UNCOMPUTED status
    for family in ["ABB", "SBB"] {
        let fam = manifest
            .constraints
            .families
            .get(family)
            .unwrap_or_else(|| panic!("Family {} should be present", family));
        assert_eq!(
            fam.status,
            ConstraintFamilyStatus::Uncomputed,
            "Family {} should have UNCOMPUTED status in Phase 1",
            family
        );
    }
}

// ============================================================
// S15: 500 constraints complete + deterministic
// ============================================================

#[test]
fn s15_large_constraint_set_deterministic() {
    let (mut store, _ettle_id, ep_id) = make_flat_store();

    // Create and attach 500 constraints across multiple families
    let families = ["ABB", "SBB", "Custom", "FrameworkX", "FrameworkY"];
    for i in 0..500usize {
        let family = families[i % families.len()];
        let id = format!("c-{:04}", i);
        constraint_ops::create_constraint(
            &mut store,
            id.clone(),
            family.to_string(),
            "Rule".to_string(),
            "EP".to_string(),
            json!({"index": i}),
        )
        .unwrap();
        constraint_ops::attach_constraint_to_ep(&mut store, ep_id.clone(), id, i as i32).unwrap();
    }

    // Generate manifest twice — should be identical
    let m1 = commit_manifest(&store, vec![ep_id.clone()]);
    let m2 = commit_manifest(&store, vec![ep_id]);

    assert_eq!(m1.constraints.declared_refs.len(), 500);
    assert_eq!(m1.constraints.declared_refs, m2.constraints.declared_refs);
    assert_eq!(
        m1.constraints.constraints_digest,
        m2.constraints.constraints_digest
    );

    // All families should be present with UNCOMPUTED status
    for family in families {
        let fam = m1
            .constraints
            .families
            .get(family)
            .unwrap_or_else(|| panic!("Family {} missing from manifest", family));
        assert_eq!(fam.status, ConstraintFamilyStatus::Uncomputed);
        // Each family has 100 constraints (500 / 5)
        assert_eq!(
            fam.active_refs.len(),
            100,
            "Family {} should have 100 constraints",
            family
        );
    }

    // Verify ordering: first ref should be ordinal 0 → c-0000
    assert_eq!(m1.constraints.declared_refs[0], "c-0000");
    // ordinal 499 → c-0499
    assert_eq!(m1.constraints.declared_refs[499], "c-0499");
}
