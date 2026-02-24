//! Decision query test suite
//!
//! Tests scenarios 22-23, 25 from seed_decision_schema_stubs_v2.yaml
//! Phase 3: Query Layer Foundation (deterministic ordering + pagination)

use ettlex_core::model::{Decision, Ep, Ettle};
use ettlex_core::ops::Store;
use ettlex_core::ops::{decision_ops, ep_ops, ettle_ops};
use ettlex_core::queries::decision_queries::{
    decision_list, ep_list_decisions, ep_list_decisions_with_ancestors,
    ept_compute_decision_context, DecisionFilters, PaginationParams,
};

fn create_test_decision(store: &mut Store, decision_id: String, status: String) -> String {
    decision_ops::create_decision(
        store,
        Some(decision_id.clone()),
        format!("Decision {}", decision_id),
        Some(status),
        "Decision text".to_string(),
        "Rationale".to_string(),
        None,
        None,
        "none".to_string(),
        None,
        None,
        None,
    )
    .unwrap();
    decision_id
}

// Scenario 22: decision.list ordering is deterministic under repeated calls
#[test]
fn test_scenario_22_deterministic_ordering() {
    let mut store = Store::new();

    // Create decisions with different created_at timestamps
    for i in 0..50 {
        create_test_decision(&mut store, format!("d:{:03}", i), "proposed".to_string());
        // Small delay to ensure different timestamps
        std::thread::sleep(std::time::Duration::from_micros(100));
    }

    // Query twice
    let filters = DecisionFilters {
        status_filter: None,
        relation_filter: None,
        include_tombstoned: false,
    };
    let pagination = PaginationParams {
        cursor: None,
        limit: 100,
    };

    let result1 = decision_list(&store, &filters, &pagination).unwrap();
    let result2 = decision_list(&store, &filters, &pagination).unwrap();

    // Results should be identical
    assert_eq!(result1.items.len(), result2.items.len());
    for (d1, d2) in result1.items.iter().zip(result2.items.iter()) {
        assert_eq!(d1.decision_id, d2.decision_id);
        assert_eq!(d1.created_at, d2.created_at);
    }

    // Verify ordering: (created_at ASC, decision_id ASC)
    for i in 1..result1.items.len() {
        let prev = &result1.items[i - 1];
        let curr = &result1.items[i];

        // Either created_at is earlier, or same created_at but decision_id is lexicographically earlier
        assert!(
            prev.created_at < curr.created_at
                || (prev.created_at == curr.created_at && prev.decision_id < curr.decision_id)
        );
    }

    // Serialize to JSON and verify byte-identical
    let json1 = serde_json::to_string(&result1.items).unwrap();
    let json2 = serde_json::to_string(&result2.items).unwrap();
    assert_eq!(json1, json2, "JSON serialization must be deterministic");
}

// Scenario 23: decision.list supports cursor-based pagination deterministically
#[test]
fn test_scenario_23_cursor_pagination() {
    let mut store = Store::new();

    // Create 250 decisions
    for i in 0..250 {
        create_test_decision(&mut store, format!("d:{:03}", i), "proposed".to_string());
        std::thread::sleep(std::time::Duration::from_micros(100));
    }

    let filters = DecisionFilters {
        status_filter: None,
        relation_filter: None,
        include_tombstoned: false,
    };

    // First page: limit 100
    let page1_result = decision_list(
        &store,
        &filters,
        &PaginationParams {
            cursor: None,
            limit: 100,
        },
    )
    .unwrap();

    assert_eq!(page1_result.items.len(), 100);
    assert!(page1_result.has_more);
    assert!(page1_result.cursor.is_some());

    let cursor_1 = page1_result.cursor.unwrap();

    // Second page: limit 100, use cursor from page 1
    let page2_result = decision_list(
        &store,
        &filters,
        &PaginationParams {
            cursor: Some(cursor_1.clone()),
            limit: 100,
        },
    )
    .unwrap();

    assert_eq!(page2_result.items.len(), 100);
    assert!(page2_result.has_more);
    assert!(page2_result.cursor.is_some());

    // Third page: should have remaining 50
    let cursor_2 = page2_result.cursor.unwrap();
    let page3_result = decision_list(
        &store,
        &filters,
        &PaginationParams {
            cursor: Some(cursor_2),
            limit: 100,
        },
    )
    .unwrap();

    assert_eq!(page3_result.items.len(), 50);
    assert!(!page3_result.has_more);
    assert!(page3_result.cursor.is_none());

    // Verify no duplicates between pages
    let mut all_ids = std::collections::HashSet::new();
    for decision in &page1_result.items {
        assert!(all_ids.insert(decision.decision_id.clone()));
    }
    for decision in &page2_result.items {
        assert!(all_ids.insert(decision.decision_id.clone()));
    }
    for decision in &page3_result.items {
        assert!(all_ids.insert(decision.decision_id.clone()));
    }
    assert_eq!(all_ids.len(), 250);

    // Verify pagination is deterministic - repeat with same cursors
    let page1_repeat = decision_list(
        &store,
        &filters,
        &PaginationParams {
            cursor: None,
            limit: 100,
        },
    )
    .unwrap();

    assert_eq!(page1_result.items.len(), page1_repeat.items.len());
    for (d1, d2) in page1_result.items.iter().zip(page1_repeat.items.iter()) {
        assert_eq!(d1.decision_id, d2.decision_id);
    }
}

// Scenario 25: ep.list_decisions filters by status
#[test]
fn test_scenario_25_filter_by_status() {
    let mut store = Store::new();

    // Create ettle and EP
    let ettle_id = ettle_ops::create_ettle(
        &mut store,
        "Test Ettle".to_string(),
        None,
        None,
        Some("what".to_string()),
        Some("how".to_string()),
    )
    .unwrap();

    let ep_id = ep_ops::create_ep(
        &mut store,
        &ettle_id,
        1,
        false,
        "why".to_string(),
        "what".to_string(),
        "how".to_string(),
    )
    .unwrap();

    // Create decisions with different statuses
    let d1 = create_test_decision(&mut store, "d:001".to_string(), "proposed".to_string());
    let d2 = create_test_decision(&mut store, "d:002".to_string(), "accepted".to_string());
    let d3 = create_test_decision(&mut store, "d:003".to_string(), "proposed".to_string());
    let d4 = create_test_decision(&mut store, "d:004".to_string(), "rejected".to_string());

    // Link all to EP
    for (i, decision_id) in [&d1, &d2, &d3, &d4].iter().enumerate() {
        decision_ops::attach_decision_to_target(
            &mut store,
            decision_id,
            "ep".to_string(),
            ep_id.clone(),
            "grounds".to_string(),
            i as i32,
        )
        .unwrap();
    }

    // Query without filter - should get all 4
    let all_decisions = ep_list_decisions(
        &store,
        &ep_id,
        &DecisionFilters {
            status_filter: None,
            relation_filter: None,
            include_tombstoned: false,
        },
    )
    .unwrap();
    assert_eq!(all_decisions.len(), 4);

    // Query with status filter "accepted" - should get 1
    let accepted_decisions = ep_list_decisions(
        &store,
        &ep_id,
        &DecisionFilters {
            status_filter: Some("accepted".to_string()),
            relation_filter: None,
            include_tombstoned: false,
        },
    )
    .unwrap();
    assert_eq!(accepted_decisions.len(), 1);
    assert_eq!(accepted_decisions[0].decision_id, "d:002");
    assert_eq!(accepted_decisions[0].status, "accepted");

    // Query with status filter "proposed" - should get 2
    let proposed_decisions = ep_list_decisions(
        &store,
        &ep_id,
        &DecisionFilters {
            status_filter: Some("proposed".to_string()),
            relation_filter: None,
            include_tombstoned: false,
        },
    )
    .unwrap();
    assert_eq!(proposed_decisions.len(), 2);
    assert_eq!(proposed_decisions[0].status, "proposed");
    assert_eq!(proposed_decisions[1].status, "proposed");

    // Verify ordering is still deterministic (ordinal ASC)
    assert_eq!(proposed_decisions[0].decision_id, "d:001"); // ordinal 0
    assert_eq!(proposed_decisions[1].decision_id, "d:003"); // ordinal 2
}

// ===== Phase 4: EPT Context Queries (Simplified) =====

// Scenario 24: ep.list_decisions include_ancestors (simplified)
#[test]
fn test_scenario_24_include_ancestors_simple() {
    let mut store = Store::new();

    // Create simple refinement chain using direct model construction
    let mut root_ettle = Ettle::new("root".to_string(), "Root".to_string());
    let root_ep = Ep::new(
        "root:ep0".to_string(),
        "root".to_string(),
        0,
        false,
        "why".to_string(),
        "what".to_string(),
        "how".to_string(),
    );
    root_ettle.add_ep_id("root:ep0".to_string());
    store.insert_ettle(root_ettle);
    store.insert_ep(root_ep);

    // Create child ettle
    let mut child_ettle = Ettle::new("child".to_string(), "Child".to_string());
    child_ettle.parent_id = Some("root".to_string());
    let child_ep0 = Ep::new(
        "child:ep0".to_string(),
        "child".to_string(),
        0,
        false,
        "why".to_string(),
        "what".to_string(),
        "how".to_string(),
    );
    child_ettle.add_ep_id("child:ep0".to_string());
    store.insert_ettle(child_ettle);
    store.insert_ep(child_ep0.clone());

    // Link root EP to child
    {
        let root_ep = store.get_ep_mut("root:ep0").unwrap();
        root_ep.child_ettle_id = Some("child".to_string());
    }

    // Create decisions
    let d_root = Decision::new(
        "d:200".to_string(),
        "Root Decision".to_string(),
        "accepted".to_string(),
        "text".to_string(),
        "rationale".to_string(),
        None,
        None,
        "none".to_string(),
        None,
        None,
        None,
    );
    let d_child = Decision::new(
        "d:201".to_string(),
        "Child Decision".to_string(),
        "accepted".to_string(),
        "text".to_string(),
        "rationale".to_string(),
        None,
        None,
        "none".to_string(),
        None,
        None,
        None,
    );

    store.insert_decision(d_root);
    store.insert_decision(d_child);

    // Link decisions to EPs
    decision_ops::attach_decision_to_target(
        &mut store,
        "d:200",
        "ep".to_string(),
        "root:ep0".to_string(),
        "grounds".to_string(),
        0,
    )
    .unwrap();

    decision_ops::attach_decision_to_target(
        &mut store,
        "d:201",
        "ep".to_string(),
        "child:ep0".to_string(),
        "grounds".to_string(),
        0,
    )
    .unwrap();

    // Query with include_ancestors=true
    let results = ep_list_decisions_with_ancestors(
        &store,
        "child:ep0",
        true,
        &DecisionFilters {
            status_filter: None,
            relation_filter: None,
            include_tombstoned: false,
        },
    )
    .unwrap();

    // Should include both decisions
    assert_eq!(results.len(), 2);
    let ids: std::collections::HashSet<String> =
        results.iter().map(|d| d.decision_id.clone()).collect();
    assert!(ids.contains("d:200"));
    assert!(ids.contains("d:201"));
}

// Scenario 26: ept.compute_decision_context (simplified)
#[test]
fn test_scenario_26_ept_context_simple() {
    let mut store = Store::new();

    let mut root_ettle = Ettle::new("root".to_string(), "Root".to_string());
    let root_ep = Ep::new(
        "root:ep0".to_string(),
        "root".to_string(),
        0,
        false,
        "why".to_string(),
        "what".to_string(),
        "how".to_string(),
    );
    root_ettle.add_ep_id("root:ep0".to_string());
    store.insert_ettle(root_ettle);
    store.insert_ep(root_ep);

    // Create decision
    let decision = Decision::new(
        "d:300".to_string(),
        "Decision".to_string(),
        "accepted".to_string(),
        "text".to_string(),
        "rationale".to_string(),
        None,
        None,
        "none".to_string(),
        None,
        None,
        None,
    );
    store.insert_decision(decision);

    decision_ops::attach_decision_to_target(
        &mut store,
        "d:300",
        "ep".to_string(),
        "root:ep0".to_string(),
        "grounds".to_string(),
        0,
    )
    .unwrap();

    // Compute context
    let context = ept_compute_decision_context(
        &store,
        "root",
        Some(0),
        &DecisionFilters {
            status_filter: None,
            relation_filter: None,
            include_tombstoned: false,
        },
    )
    .unwrap();

    assert!(context.direct_by_ep.contains_key("root:ep0"));
    assert_eq!(context.direct_by_ep["root:ep0"].len(), 1);
}

// Scenario 27: Rejects ambiguous graphs (simplified)
#[test]
fn test_scenario_27_ambiguous_graph_simple() {
    let mut store = Store::new();

    // Create parent ettle with TWO EPs
    let mut parent = Ettle::new("parent".to_string(), "Parent".to_string());
    let parent_ep0 = Ep::new(
        "parent:ep0".to_string(),
        "parent".to_string(),
        0,
        false,
        "why".to_string(),
        "what".to_string(),
        "how".to_string(),
    );
    let parent_ep1 = Ep::new(
        "parent:ep1".to_string(),
        "parent".to_string(),
        1,
        false,
        "why".to_string(),
        "what".to_string(),
        "how".to_string(),
    );
    parent.add_ep_id("parent:ep0".to_string());
    parent.add_ep_id("parent:ep1".to_string());

    // Create child
    let mut child = Ettle::new("child".to_string(), "Child".to_string());
    child.parent_id = Some("parent".to_string());
    let child_ep = Ep::new(
        "child:ep0".to_string(),
        "child".to_string(),
        0,
        false,
        "why".to_string(),
        "what".to_string(),
        "how".to_string(),
    );
    child.add_ep_id("child:ep0".to_string());

    store.insert_ettle(parent);
    store.insert_ettle(child);
    store.insert_ep(parent_ep0);
    store.insert_ep(parent_ep1.clone());
    store.insert_ep(child_ep);

    // Make BOTH parent EPs map to the same child (ambiguous!)
    {
        let ep0 = store.get_ep_mut("parent:ep0").unwrap();
        ep0.child_ettle_id = Some("child".to_string());
    }
    {
        let ep1 = store.get_ep_mut("parent:ep1").unwrap();
        ep1.child_ettle_id = Some("child".to_string());
    }

    // This should fail with EptDuplicateMapping
    let result = ep_list_decisions_with_ancestors(
        &store,
        "child:ep0",
        true,
        &DecisionFilters {
            status_filter: None,
            relation_filter: None,
            include_tombstoned: false,
        },
    );

    assert!(result.is_err());
    assert!(matches!(
        result,
        Err(ettlex_core::errors::EttleXError::EptDuplicateMapping { .. })
    ));
}
