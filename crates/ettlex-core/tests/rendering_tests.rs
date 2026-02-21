mod common;

use common::{new_store, setup_simple_tree};
use ettlex_core::{
    ops::{ep_ops, ettle_ops},
    render,
};

// ===== RENDER_ETTLE TESTS =====

#[test]
fn test_render_ettle_includes_title_and_eps() {
    let mut store = new_store();
    let ettle_id =
        ettle_ops::create_ettle(&mut store, "My Ettle".to_string(), None, None, None, None)
            .unwrap();

    ep_ops::create_ep(
        &mut store,
        &ettle_id,
        1,
        true,
        "Why 1".to_string(),
        "What 1".to_string(),
        "How 1".to_string(),
    )
    .unwrap();

    ep_ops::create_ep(
        &mut store,
        &ettle_id,
        2,
        false,
        "Why 2".to_string(),
        "What 2".to_string(),
        "How 2".to_string(),
    )
    .unwrap();

    let output = render::render_ettle(&store, &ettle_id).unwrap();

    assert!(output.contains("# My Ettle"));
    assert!(output.contains("## EP 0"));
    assert!(output.contains("## EP 1"));
    assert!(output.contains("## EP 2"));
    assert!(output.contains("Why 1"));
    assert!(output.contains("What 1"));
    assert!(output.contains("How 1"));
}

#[test]
fn test_render_ettle_eps_in_ordinal_order() {
    let mut store = new_store();
    let ettle_id =
        ettle_ops::create_ettle(&mut store, "Test".to_string(), None, None, None, None).unwrap();

    // Create EPs out of order
    ep_ops::create_ep(
        &mut store,
        &ettle_id,
        5,
        true,
        "5".to_string(),
        String::new(),
        String::new(),
    )
    .unwrap();
    ep_ops::create_ep(
        &mut store,
        &ettle_id,
        2,
        true,
        "2".to_string(),
        String::new(),
        String::new(),
    )
    .unwrap();
    ep_ops::create_ep(
        &mut store,
        &ettle_id,
        10,
        true,
        "10".to_string(),
        String::new(),
        String::new(),
    )
    .unwrap();

    let output = render::render_ettle(&store, &ettle_id).unwrap();

    // Find positions of each EP in output
    let pos_ep0 = output.find("## EP 0").unwrap();
    let pos_ep2 = output.find("## EP 2").unwrap();
    let pos_ep5 = output.find("## EP 5").unwrap();
    let pos_ep10 = output.find("## EP 10").unwrap();

    // Verify they appear in ordinal order
    assert!(pos_ep0 < pos_ep2);
    assert!(pos_ep2 < pos_ep5);
    assert!(pos_ep5 < pos_ep10);
}

// ===== RENDER_LEAF_BUNDLE TESTS =====

#[test]
fn test_render_leaf_bundle_aggregates_ept() {
    let mut store = new_store();
    let (root_id, mid_id, leaf_id) = setup_simple_tree(&mut store);

    // Update EP text for verification
    let root_ep1_id = {
        let root = store.get_ettle(&root_id).unwrap();
        root.ep_ids[1].clone()
    };
    ep_ops::update_ep(
        &mut store,
        &root_ep1_id,
        Some("Root Why".to_string()),
        Some("Root What".to_string()),
        Some("Root How".to_string()),
        None,
    )
    .unwrap();

    let mid_ep1_id = {
        let mid = store.get_ettle(&mid_id).unwrap();
        mid.ep_ids[1].clone()
    };
    ep_ops::update_ep(
        &mut store,
        &mid_ep1_id,
        Some("Mid Why".to_string()),
        Some("Mid What".to_string()),
        Some("Mid How".to_string()),
        None,
    )
    .unwrap();

    let output = render::render_leaf_bundle(&store, &leaf_id, None).unwrap();

    // Verify bundle contains aggregated WHY/WHAT/HOW
    assert!(output.contains("Root Why"));
    assert!(output.contains("Root What"));
    assert!(output.contains("Root How"));
    assert!(output.contains("Mid Why"));
    assert!(output.contains("Mid What"));
    assert!(output.contains("Mid How"));
}

#[test]
fn test_render_leaf_bundle_deterministic() {
    let mut store = new_store();
    let (_, _, leaf_id) = setup_simple_tree(&mut store);

    let output1 = render::render_leaf_bundle(&store, &leaf_id, None).unwrap();
    let output2 = render::render_leaf_bundle(&store, &leaf_id, None).unwrap();
    let output3 = render::render_leaf_bundle(&store, &leaf_id, None).unwrap();

    assert_eq!(output1, output2);
    assert_eq!(output2, output3);
}

#[test]
fn test_render_leaf_bundle_with_specified_ep() {
    let mut store = new_store();
    let root_id =
        ettle_ops::create_ettle(&mut store, "Root".to_string(), None, None, None, None).unwrap();

    ep_ops::create_ep(
        &mut store,
        &root_id,
        1,
        true,
        "EP1 Why".to_string(),
        "EP1 What".to_string(),
        "EP1 How".to_string(),
    )
    .unwrap();

    ep_ops::create_ep(
        &mut store,
        &root_id,
        2,
        false,
        "EP2 Why".to_string(),
        "EP2 What".to_string(),
        "EP2 How".to_string(),
    )
    .unwrap();

    let output = render::render_leaf_bundle(&store, &root_id, Some(2)).unwrap();

    assert!(output.contains("EP2 Why"));
    assert!(output.contains("EP2 What"));
    assert!(output.contains("EP2 How"));
}
