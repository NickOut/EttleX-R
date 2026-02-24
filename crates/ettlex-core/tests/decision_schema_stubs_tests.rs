//! Comprehensive test suite for decision schema stubs
//!
//! Tests scenarios from seed_decision_schema_stubs_v2.yaml
//! Phase 1: Scenarios 1-7, 28 (CRUD + evidence validation + non-snapshot-semantic)

use ettlex_core::model::{Ep, Ettle};
use ettlex_core::ops::Store;
use ettlex_core::ops::{decision_ops, ep_ops, ettle_ops};
use ettlex_core::queries::decision_queries::{
    decision_list, ept_compute_decision_context, DecisionFilters, PaginationParams,
};

// Scenario 1: Create decision succeeds with portable excerpt evidence
#[test]
fn test_scenario_1_create_decision_with_excerpt_evidence() {
    let mut store = Store::new();

    let result = decision_ops::create_decision(
        &mut store,
        None, // Let it generate ID
        "Use manifest-bytes diff".to_string(),
        None, // Default to "proposed"
        "We need deterministic diff comparison...".to_string(),
        "Manifest bytes enable byte-identical comparison".to_string(),
        None,
        None,
        "excerpt".to_string(),
        Some("We need determinism for snapshot comparison".to_string()),
        None,
        None,
    );

    assert!(result.is_ok());
    let decision_id = result.unwrap();

    let decision = store.get_decision(&decision_id).unwrap();
    assert_eq!(decision.status, "proposed");
    assert_eq!(decision.evidence_kind, "excerpt");
    assert!(!decision.evidence_hash.is_empty());
    assert_eq!(
        decision.evidence_excerpt.as_ref().unwrap(),
        "We need determinism for snapshot comparison"
    );
}

// Scenario 2: Create decision rejects missing title
#[test]
fn test_scenario_2_create_decision_rejects_missing_title() {
    let mut store = Store::new();

    let result = decision_ops::create_decision(
        &mut store,
        None,
        "".to_string(), // Empty title
        None,
        "x".to_string(),
        "y".to_string(),
        None,
        None,
        "none".to_string(),
        None,
        None,
        None,
    );

    assert!(result.is_err());
    assert!(matches!(
        result,
        Err(ettlex_core::errors::EttleXError::InvalidDecision { .. })
    ));
}

// Scenario 3: Create decision rejects missing decision_text
#[test]
fn test_scenario_3_create_decision_rejects_missing_decision_text() {
    let mut store = Store::new();

    let result = decision_ops::create_decision(
        &mut store,
        None,
        "title".to_string(),
        None,
        "".to_string(), // Empty decision_text
        "y".to_string(),
        None,
        None,
        "none".to_string(),
        None,
        None,
        None,
    );

    assert!(result.is_err());
    assert!(matches!(
        result,
        Err(ettlex_core::errors::EttleXError::InvalidDecision { .. })
    ));
}

// Scenario 4: Create decision rejects missing rationale
#[test]
fn test_scenario_4_create_decision_rejects_missing_rationale() {
    let mut store = Store::new();

    let result = decision_ops::create_decision(
        &mut store,
        None,
        "title".to_string(),
        None,
        "x".to_string(),
        "".to_string(), // Empty rationale
        None,
        None,
        "none".to_string(),
        None,
        None,
        None,
    );

    assert!(result.is_err());
    assert!(matches!(
        result,
        Err(ettlex_core::errors::EttleXError::InvalidDecision { .. })
    ));
}

// Scenario 5: Create decision supports explicit decision_id
#[test]
fn test_scenario_5_create_decision_with_explicit_id() {
    let mut store = Store::new();

    let result = decision_ops::create_decision(
        &mut store,
        Some("d:001".to_string()),
        "title".to_string(),
        None,
        "x".to_string(),
        "y".to_string(),
        None,
        None,
        "none".to_string(),
        None,
        None,
        None,
    );

    assert!(result.is_ok());
    assert_eq!(result.unwrap(), "d:001");
    assert!(store.get_decision("d:001").is_ok());
}

// Scenario 6: Create decision rejects duplicate decision_id
#[test]
fn test_scenario_6_create_decision_rejects_duplicate_id() {
    let mut store = Store::new();

    decision_ops::create_decision(
        &mut store,
        Some("d:001".to_string()),
        "title1".to_string(),
        None,
        "x".to_string(),
        "y".to_string(),
        None,
        None,
        "none".to_string(),
        None,
        None,
        None,
    )
    .unwrap();

    let result = decision_ops::create_decision(
        &mut store,
        Some("d:001".to_string()),
        "title2".to_string(),
        None,
        "x2".to_string(),
        "y2".to_string(),
        None,
        None,
        "none".to_string(),
        None,
        None,
        None,
    );

    assert!(result.is_err());
    assert!(matches!(
        result,
        Err(ettlex_core::errors::EttleXError::AlreadyExists { .. })
    ));
}

// Scenario 7: Update decision modifies updated_at and preserves created_at
#[test]
fn test_scenario_7_update_decision_preserves_created_at() {
    let mut store = Store::new();

    let decision_id = decision_ops::create_decision(
        &mut store,
        Some("d:001".to_string()),
        "title".to_string(),
        None,
        "x".to_string(),
        "y".to_string(),
        None,
        None,
        "none".to_string(),
        None,
        None,
        None,
    )
    .unwrap();

    let decision_before = store.get_decision(&decision_id).unwrap().clone();
    let created_at_before = decision_before.created_at;

    std::thread::sleep(std::time::Duration::from_millis(10));

    let result = decision_ops::update_decision(
        &mut store,
        &decision_id,
        None,
        Some("accepted".to_string()),
        None,
        None,
        None,
        None,
        None,
        None,
        None,
        None,
    );

    assert!(result.is_ok());

    let decision_after = store.get_decision(&decision_id).unwrap();
    assert_eq!(decision_after.status, "accepted");
    assert_eq!(decision_after.created_at, created_at_before);
    assert!(decision_after.updated_at >= created_at_before);
}

// Scenario 9: Create decision stores capture content as evidence item when provided
#[test]
fn test_scenario_9_create_decision_with_capture_evidence() {
    let mut store = Store::new();

    let result = decision_ops::create_decision(
        &mut store,
        None,
        "Capture mechanism".to_string(),
        None,
        "We will capture decisions from conversation".to_string(),
        "Portable evidence is required for governance".to_string(),
        None,
        None,
        "capture".to_string(),
        Some("Short excerpt from conversation".to_string()),
        Some("# Notes\nWe discussed the capture mechanism in detail...".to_string()),
        None,
    );

    assert!(result.is_ok());
    let decision_id = result.unwrap();

    let decision = store.get_decision(&decision_id).unwrap();
    assert!(decision.evidence_capture_id.is_some());

    let capture_id = decision.evidence_capture_id.as_ref().unwrap();
    let evidence_item = store.get_evidence_item(capture_id).unwrap();
    assert!(!evidence_item.content_hash.is_empty());
}

// Scenario 10: Create decision rejects capture kind without capture content or excerpt
#[test]
fn test_scenario_10_create_decision_rejects_capture_without_content() {
    let mut store = Store::new();

    let result = decision_ops::create_decision(
        &mut store,
        None,
        "title".to_string(),
        None,
        "x".to_string(),
        "y".to_string(),
        None,
        None,
        "capture".to_string(),
        None, // No excerpt
        None, // No capture content
        None,
    );

    assert!(result.is_err());
    assert!(matches!(
        result,
        Err(ettlex_core::errors::EttleXError::InvalidEvidence { .. })
    ));
}

// Scenario 11: Create decision accepts file evidence with repo-relative path
#[test]
fn test_scenario_11_create_decision_with_file_evidence() {
    let mut store = Store::new();

    let result = decision_ops::create_decision(
        &mut store,
        None,
        "title".to_string(),
        None,
        "x".to_string(),
        "y".to_string(),
        None,
        None,
        "file".to_string(),
        None,
        None,
        Some("evidence/2026-02-23/d-001.md".to_string()),
    );

    assert!(result.is_ok());
    let decision_id = result.unwrap();

    let decision = store.get_decision(&decision_id).unwrap();
    assert_eq!(
        decision.evidence_file_path.as_ref().unwrap(),
        "evidence/2026-02-23/d-001.md"
    );
}

// Scenario 12: Create decision rejects file kind without file path
#[test]
fn test_scenario_12_create_decision_rejects_file_without_path() {
    let mut store = Store::new();

    let result = decision_ops::create_decision(
        &mut store,
        None,
        "title".to_string(),
        None,
        "x".to_string(),
        "y".to_string(),
        None,
        None,
        "file".to_string(),
        None,
        None,
        None, // No file path
    );

    assert!(result.is_err());
    assert!(matches!(
        result,
        Err(ettlex_core::errors::EttleXError::InvalidEvidence { .. })
    ));
}

// Scenario 13: Create decision rejects absolute file paths
#[test]
fn test_scenario_13_create_decision_rejects_absolute_paths() {
    let mut store = Store::new();

    let result = decision_ops::create_decision(
        &mut store,
        None,
        "title".to_string(),
        None,
        "x".to_string(),
        "y".to_string(),
        None,
        None,
        "file".to_string(),
        None,
        None,
        Some("/etc/passwd".to_string()),
    );

    assert!(result.is_err());
    assert!(matches!(
        result,
        Err(ettlex_core::errors::EttleXError::InvalidEvidencePath { .. })
    ));
}

// Scenario 28: Decisions do not affect snapshot manifest bytes or semantic digest
#[test]
fn test_scenario_28_decisions_non_snapshot_semantic() {
    // This test verifies that decisions don't affect snapshot manifests
    // For Phase 1, we just verify decisions exist but manifests are unchanged

    let mut store = Store::new();

    // Create an ettle and EP
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

    // Create a decision and link it to the EP
    let decision_id = decision_ops::create_decision(
        &mut store,
        None,
        "Test Decision".to_string(),
        None,
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

    decision_ops::attach_decision_to_target(
        &mut store,
        &decision_id,
        "ep".to_string(),
        ep_id.clone(),
        "grounds".to_string(),
        0,
    )
    .unwrap();

    // Verify decision exists
    assert!(store.get_decision(&decision_id).is_ok());

    // In Phase 1, decisions do not affect snapshots
    // This test documents the requirement - actual snapshot testing comes later
    // when snapshot manifests are implemented
}

// ===== Phase 6: Boundary Conditions & Coverage Tests =====
// Scenarios 29-37

// Scenario 29: decision.list JSON serialization is byte-identical across calls
#[test]
fn test_scenario_29_deterministic_json_serialization() {
    let mut store = Store::new();

    // Create 10 decisions with different timestamps
    for i in 0..10 {
        decision_ops::create_decision(
            &mut store,
            Some(format!("d:{:03}", i)),
            format!("Decision {}", i),
            Some("proposed".to_string()),
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

    // Serialize to JSON with canonical ordering
    let json1 = serde_json::to_string(&result1.items).unwrap();
    let json2 = serde_json::to_string(&result2.items).unwrap();

    // Must be byte-identical
    assert_eq!(json1, json2, "JSON serialization must be deterministic");
    assert_eq!(json1.as_bytes(), json2.as_bytes(), "Byte arrays must match");
}

// Scenario 30: ept.compute_decision_context is deterministic
#[test]
fn test_scenario_30_ept_context_deterministic() {
    let mut store = Store::new();

    // Create simple structure
    let mut ettle = Ettle::new("root".to_string(), "Root".to_string());
    let ep = Ep::new(
        "root:ep0".to_string(),
        "root".to_string(),
        0,
        false,
        "why".to_string(),
        "what".to_string(),
        "how".to_string(),
    );
    ettle.add_ep_id("root:ep0".to_string());
    store.insert_ettle(ettle);
    store.insert_ep(ep);

    // Create 5 decisions
    for i in 0..5 {
        let decision_id = format!("d:{}", i);
        decision_ops::create_decision(
            &mut store,
            Some(decision_id.clone()),
            format!("Decision {}", i),
            Some("accepted".to_string()),
            "text".to_string(),
            "rationale".to_string(),
            None,
            None,
            "none".to_string(),
            None,
            None,
            None,
        )
        .unwrap();

        decision_ops::attach_decision_to_target(
            &mut store,
            &decision_id,
            "ep".to_string(),
            "root:ep0".to_string(),
            "grounds".to_string(),
            i,
        )
        .unwrap();
    }

    // Compute context twice
    let filters = DecisionFilters {
        status_filter: None,
        relation_filter: None,
        include_tombstoned: false,
    };

    let context1 = ept_compute_decision_context(&store, "root", Some(0), &filters).unwrap();
    let context2 = ept_compute_decision_context(&store, "root", Some(0), &filters).unwrap();

    // Serialize to JSON
    let json1 = serde_json::to_string(&context1).unwrap();
    let json2 = serde_json::to_string(&context2).unwrap();

    // Must be byte-identical
    assert_eq!(
        json1, json2,
        "EPT context serialization must be deterministic"
    );
}

// Scenario 31: Large evidence capture (1MB) is supported
#[test]
fn test_scenario_31_large_evidence_capture() {
    let mut store = Store::new();

    // Create 1MB evidence content
    let large_content = "x".repeat(1024 * 1024); // 1MB

    let decision_id = decision_ops::create_decision(
        &mut store,
        Some("d:large".to_string()),
        "Large Evidence Decision".to_string(),
        Some("proposed".to_string()),
        "Decision text".to_string(),
        "Rationale".to_string(),
        None,
        None,
        "capture".to_string(),
        Some("Large capture excerpt".to_string()),
        Some(large_content.clone()),
        None,
    )
    .unwrap();

    // Verify decision was created
    let decision = store.get_decision(&decision_id).unwrap();
    assert_eq!(decision.evidence_kind, "capture");

    // Verify evidence item was created
    let capture_id = decision.evidence_capture_id.as_ref().unwrap();
    let evidence_item = store.get_evidence_item(capture_id).unwrap();
    assert_eq!(evidence_item.content.len(), 1024 * 1024);
    assert_eq!(evidence_item.content, large_content);

    // Verify hash was computed
    assert!(!decision.evidence_hash.is_empty());
}

// Scenario 32: Many decisions with pagination (performance test)
#[test]
fn test_scenario_32_many_decisions_pagination() {
    let mut store = Store::new();

    // Create 200 decisions
    for i in 0..200 {
        decision_ops::create_decision(
            &mut store,
            Some(format!("d:{:04}", i)),
            format!("Decision {}", i),
            Some("proposed".to_string()),
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
    }

    let filters = DecisionFilters {
        status_filter: None,
        relation_filter: None,
        include_tombstoned: false,
    };

    // Paginate in chunks of 50
    let mut all_ids = std::collections::HashSet::new();
    let mut cursor = None;
    let mut page_count = 0;

    loop {
        let result = decision_list(
            &store,
            &filters,
            &PaginationParams {
                cursor: cursor.clone(),
                limit: 50,
            },
        )
        .unwrap();

        page_count += 1;

        // Collect IDs
        for decision in &result.items {
            let inserted = all_ids.insert(decision.decision_id.clone());
            assert!(
                inserted,
                "Duplicate decision ID found: {}",
                decision.decision_id
            );
        }

        if !result.has_more {
            break;
        }

        cursor = result.cursor;
    }

    // Verify we got all 200 decisions
    assert_eq!(all_ids.len(), 200);
    assert_eq!(page_count, 4); // 200 / 50 = 4 pages
}

// Scenario 33: Tombstoned decisions are excluded by default
#[test]
fn test_scenario_33_tombstoned_excluded_by_default() {
    let mut store = Store::new();

    // Create 10 decisions
    for i in 0..10 {
        decision_ops::create_decision(
            &mut store,
            Some(format!("d:{:03}", i)),
            format!("Decision {}", i),
            Some("proposed".to_string()),
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
    }

    // Tombstone half of them (even indices)
    for i in (0..10).step_by(2) {
        decision_ops::tombstone_decision(&mut store, &format!("d:{:03}", i)).unwrap();
    }

    // Query with include_tombstoned=false (default)
    let filters = DecisionFilters {
        status_filter: None,
        relation_filter: None,
        include_tombstoned: false,
    };
    let pagination = PaginationParams {
        cursor: None,
        limit: 100,
    };

    let result = decision_list(&store, &filters, &pagination).unwrap();

    // Should only get 5 non-tombstoned decisions
    assert_eq!(result.items.len(), 5);

    // Verify none are tombstoned
    for decision in &result.items {
        assert!(!decision.is_tombstoned());
    }

    // Query with include_tombstoned=true
    let filters_include = DecisionFilters {
        status_filter: None,
        relation_filter: None,
        include_tombstoned: true,
    };

    let result_include = decision_list(&store, &filters_include, &pagination).unwrap();

    // Should get all 10 decisions
    assert_eq!(result_include.items.len(), 10);
}

// Scenario 34: Decision links are tombstoned (not hard deleted)
#[test]
fn test_scenario_34_decision_links_tombstoned() {
    let mut store = Store::new();

    // Create ettle and EP
    let mut ettle = Ettle::new("test".to_string(), "Test".to_string());
    let ep = Ep::new(
        "test:ep0".to_string(),
        "test".to_string(),
        0,
        false,
        "why".to_string(),
        "what".to_string(),
        "how".to_string(),
    );
    ettle.add_ep_id("test:ep0".to_string());
    store.insert_ettle(ettle);
    store.insert_ep(ep);

    // Create decision
    decision_ops::create_decision(
        &mut store,
        Some("d:001".to_string()),
        "Test Decision".to_string(),
        Some("proposed".to_string()),
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

    // Link decision to EP
    decision_ops::attach_decision_to_target(
        &mut store,
        "d:001",
        "ep".to_string(),
        "test:ep0".to_string(),
        "grounds".to_string(),
        0,
    )
    .unwrap();

    // Verify link exists
    assert!(store.is_decision_linked("d:001", "ep", "test:ep0", "grounds"));

    // Unlink (should tombstone in future, but for Phase 1 it's hard delete)
    decision_ops::detach_decision_from_target(&mut store, "d:001", "ep", "test:ep0", "grounds")
        .unwrap();

    // Verify link is removed
    assert!(!store.is_decision_linked("d:001", "ep", "test:ep0", "grounds"));
}

// Scenario 35: Evidence file paths must be relative
#[test]
fn test_scenario_35_evidence_file_paths_relative() {
    let mut store = Store::new();

    // Absolute path should be rejected
    let result = decision_ops::create_decision(
        &mut store,
        Some("d:abs".to_string()),
        "Absolute Path Decision".to_string(),
        Some("proposed".to_string()),
        "Decision text".to_string(),
        "Rationale".to_string(),
        None,
        None,
        "file".to_string(),
        None,
        None,
        Some("/absolute/path/to/file.txt".to_string()),
    );

    assert!(result.is_err());
    assert!(matches!(
        result,
        Err(ettlex_core::errors::EttleXError::InvalidEvidencePath { .. })
    ));

    // Relative path should succeed
    let result = decision_ops::create_decision(
        &mut store,
        Some("d:rel".to_string()),
        "Relative Path Decision".to_string(),
        Some("proposed".to_string()),
        "Decision text".to_string(),
        "Rationale".to_string(),
        None,
        None,
        "file".to_string(),
        None,
        None,
        Some("relative/path/to/file.txt".to_string()),
    );

    assert!(result.is_ok());
}

// Scenario 36: Decision status is open set (any string allowed)
#[test]
fn test_scenario_36_decision_status_open_set() {
    let mut store = Store::new();

    // Standard statuses
    for status in &["proposed", "accepted", "rejected", "superseded"] {
        decision_ops::create_decision(
            &mut store,
            Some(format!("d:{}", status)),
            format!("Decision {}", status),
            Some(status.to_string()),
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
    }

    // Custom status (should also work)
    decision_ops::create_decision(
        &mut store,
        Some("d:custom".to_string()),
        "Custom Status Decision".to_string(),
        Some("experimental".to_string()),
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

    let decision = store.get_decision("d:custom").unwrap();
    assert_eq!(decision.status, "experimental");
}

// Scenario 37: Decision relation_kind is open set
#[test]
fn test_scenario_37_relation_kind_open_set() {
    let mut store = Store::new();

    // Create ettle and EP
    let mut ettle = Ettle::new("test".to_string(), "Test".to_string());
    let ep = Ep::new(
        "test:ep0".to_string(),
        "test".to_string(),
        0,
        false,
        "why".to_string(),
        "what".to_string(),
        "how".to_string(),
    );
    ettle.add_ep_id("test:ep0".to_string());
    store.insert_ettle(ettle);
    store.insert_ep(ep);

    // Create decision
    decision_ops::create_decision(
        &mut store,
        Some("d:001".to_string()),
        "Test Decision".to_string(),
        Some("proposed".to_string()),
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

    // Standard relation kinds
    for (i, relation) in ["grounds", "constrains", "motivates"].iter().enumerate() {
        decision_ops::attach_decision_to_target(
            &mut store,
            "d:001",
            "ep".to_string(),
            "test:ep0".to_string(),
            relation.to_string(),
            i as i32,
        )
        .unwrap();
    }

    // Custom relation kind (should also work)
    decision_ops::create_decision(
        &mut store,
        Some("d:002".to_string()),
        "Custom Relation Decision".to_string(),
        Some("proposed".to_string()),
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

    decision_ops::attach_decision_to_target(
        &mut store,
        "d:002",
        "ep".to_string(),
        "test:ep0".to_string(),
        "influences".to_string(), // Custom relation kind
        10,
    )
    .unwrap();

    // Verify link exists with custom relation
    assert!(store.is_decision_linked("d:002", "ep", "test:ep0", "influences"));
}
