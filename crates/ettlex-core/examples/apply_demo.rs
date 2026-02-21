//! Apply API Demonstration
//!
//! This example demonstrates the functional-boundary `apply()` API for EttleX operations.
#![allow(clippy::unwrap_used, clippy::expect_used)]
//!
//! Key concepts illustrated:
//! 1. Immutable state threading (apply returns new state)
//! 2. Command-based operations
//! 3. Anchored deletion policies (hard delete vs tombstone)
//! 4. Atomicity guarantees
//! 5. Chaining operations

use ettlex_core::{
    apply,
    ops::active_eps,
    policy::{NeverAnchoredPolicy, SelectedAnchoredPolicy},
    Command, Store,
};
use std::collections::HashSet;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("=== EttleX Apply API Demo ===\n");

    // ===== Part 1: Basic Ettle Creation with State Threading =====
    println!("## Part 1: State Threading\n");

    let state = Store::new();
    println!("Created empty store");

    let policy = NeverAnchoredPolicy;

    // Create first Ettle
    let cmd = Command::EttleCreate {
        title: "API Gateway".to_string(),
        metadata: None,
        why: Some("Entry point for external requests".to_string()),
        what: Some("HTTP/HTTPS endpoint handler".to_string()),
        how: Some("Load balancer configuration".to_string()),
    };

    let state = apply(state, cmd, &policy)?;
    println!("✓ Created 'API Gateway' ettle");

    // Create second Ettle (state threading)
    let cmd = Command::EttleCreate {
        title: "Authentication Service".to_string(),
        metadata: None,
        why: None,
        what: None,
        how: None,
    };

    let state = apply(state, cmd, &policy)?;
    println!("✓ Created 'Authentication Service' ettle");

    assert_eq!(state.list_ettles().len(), 2);
    println!("State now contains {} Ettles\n", state.list_ettles().len());

    // ===== Part 2: Adding EPs and Building Refinement =====
    println!("## Part 2: Refinement Structure\n");

    // Extract IDs before moving state
    let api_gateway_id = state.list_ettles()[0].id.clone();
    let auth_service_id = state.list_ettles()[1].id.clone();

    // Add EP to API Gateway
    let cmd = Command::EpCreate {
        ettle_id: api_gateway_id.clone(),
        ordinal: 1,
        normative: true,
        why: "Require authentication for all requests".to_string(),
        what: "Auth middleware layer".to_string(),
        how: "JWT validation filter".to_string(),
    };

    let state = apply(state, cmd, &policy)?;
    println!("✓ Added EP1 to API Gateway");

    // Link auth service as child
    let api_gateway = state.get_ettle(&api_gateway_id)?;
    let api_active = active_eps(&state, api_gateway)?;
    let ep1_id = api_active
        .iter()
        .find(|ep| ep.ordinal == 1)
        .unwrap()
        .id
        .clone();

    let cmd = Command::RefineLinkChild {
        parent_ep_id: ep1_id,
        child_ettle_id: auth_service_id.clone(),
    };

    let state = apply(state, cmd, &policy)?;
    println!("✓ Linked Authentication Service to API Gateway EP1");

    // Verify linkage
    let auth_service = state.get_ettle(&auth_service_id)?;
    assert!(auth_service.parent_id.is_some());
    println!(
        "  Auth Service now has parent: {}\n",
        auth_service.parent_id.is_some()
    );

    // ===== Part 3: Hard Delete vs Tombstone =====
    println!("## Part 3: Deletion Strategies\n");

    // Create two test Ettles with EPs for deletion demo
    let cmd = Command::EttleCreate {
        title: "Draft Concept".to_string(),
        metadata: None,
        why: None,
        what: None,
        how: None,
    };
    let state = apply(state, cmd, &policy)?;

    // Capture the draft ID immediately after creation
    let draft_id = state
        .list_ettles()
        .iter()
        .find(|e| e.title == "Draft Concept")
        .unwrap()
        .id
        .clone();

    let cmd = Command::EttleCreate {
        title: "Published Design".to_string(),
        metadata: None,
        why: None,
        what: None,
        how: None,
    };
    let state = apply(state, cmd, &policy)?;

    // Capture the published ID immediately after creation
    let published_id = state
        .list_ettles()
        .iter()
        .find(|e| e.title == "Published Design")
        .unwrap()
        .id
        .clone();

    // Add EPs to both
    let cmd = Command::EpCreate {
        ettle_id: draft_id.clone(),
        ordinal: 1,
        normative: true,
        why: String::new(),
        what: String::new(),
        how: String::new(),
    };
    let state = apply(state, cmd, &policy)?;

    let cmd = Command::EpCreate {
        ettle_id: published_id.clone(),
        ordinal: 1,
        normative: true,
        why: String::new(),
        what: String::new(),
        how: String::new(),
    };
    let state = apply(state, cmd, &policy)?;

    // Extract EP IDs
    let draft_ep1_id = {
        let draft = state.get_ettle(&draft_id)?;
        draft
            .ep_ids
            .iter()
            .filter_map(|id| state.get_ep(id).ok())
            .find(|ep| ep.ordinal == 1)
            .unwrap()
            .id
            .clone()
    };

    let published_ep1_id = {
        let published = state.get_ettle(&published_id)?;
        published
            .ep_ids
            .iter()
            .filter_map(|id| state.get_ep(id).ok())
            .find(|ep| ep.ordinal == 1)
            .unwrap()
            .id
            .clone()
    };

    println!("Created draft and published Ettles with EPs");

    // Delete draft EP with NeverAnchoredPolicy (hard delete)
    let churn_policy = NeverAnchoredPolicy;
    let cmd = Command::EpDelete {
        ep_id: draft_ep1_id.clone(),
    };
    let state = apply(state, cmd, &churn_policy)?;

    println!("✓ Hard deleted draft EP (churn mode)");
    assert!(
        !state.ep_exists_in_storage(&draft_ep1_id),
        "Draft EP should be completely removed"
    );

    // Delete published EP with SelectedAnchoredPolicy (tombstone)
    let mut anchored_eps = HashSet::new();
    anchored_eps.insert(published_ep1_id.clone());
    let anchored_policy = SelectedAnchoredPolicy::with_eps(anchored_eps);

    let cmd = Command::EpDelete {
        ep_id: published_ep1_id.clone(),
    };
    let state = apply(state, cmd, &anchored_policy)?;

    println!("✓ Tombstoned published EP (anchored)");
    assert!(
        state.ep_exists_in_storage(&published_ep1_id),
        "Published EP should still exist in storage"
    );
    assert!(
        state.get_ep_raw(&published_ep1_id).unwrap().deleted,
        "Published EP should be marked as deleted"
    );

    println!("\nDeletion comparison:");
    println!(
        "  Draft EP (hard): exists = {}",
        state.ep_exists_in_storage(&draft_ep1_id)
    );
    println!(
        "  Published EP (tombstone): exists = {}, deleted = {}",
        state.ep_exists_in_storage(&published_ep1_id),
        state.get_ep_raw(&published_ep1_id).unwrap().deleted
    );

    // ===== Part 4: Atomicity on Error =====
    println!("\n## Part 4: Atomicity Guarantee\n");

    let state_before = state.clone();
    let ettle_count_before = state_before.list_ettles().len();

    // Attempt invalid operation
    let cmd = Command::EttleCreate {
        title: "".to_string(), // Invalid: empty title
        metadata: None,
        why: None,
        what: None,
        how: None,
    };

    let result = apply(state, cmd, &NeverAnchoredPolicy);

    println!("Attempted to create Ettle with empty title");
    assert!(result.is_err(), "Operation should fail");
    println!("✗ Operation failed as expected");

    // Original state is still valid
    println!("  Ettle count before: {}", ettle_count_before);
    println!(
        "  Ettle count in preserved state: {}",
        state_before.list_ettles().len()
    );
    assert_eq!(state_before.list_ettles().len(), ettle_count_before);
    println!("✓ Original state remains valid and unchanged\n");

    // ===== Part 5: Chained Operations =====
    println!("## Part 5: Operation Chaining\n");

    let state = state_before; // Use preserved state

    // Chain multiple operations
    let state = apply(
        state,
        Command::EttleCreate {
            title: "Database Layer".to_string(),
            metadata: None,
            why: None,
            what: None,
            how: None,
        },
        &NeverAnchoredPolicy,
    )?;

    let state = apply(
        state,
        Command::EttleCreate {
            title: "Cache Layer".to_string(),
            metadata: None,
            why: None,
            what: None,
            how: None,
        },
        &NeverAnchoredPolicy,
    )?;

    let state = apply(
        state,
        Command::EttleCreate {
            title: "Queue Service".to_string(),
            metadata: None,
            why: None,
            what: None,
            how: None,
        },
        &NeverAnchoredPolicy,
    )?;

    println!("✓ Chained creation of 3 Ettles");
    println!(
        "Final state contains {} Ettles\n",
        state.list_ettles().len()
    );

    // ===== Summary =====
    println!("## Summary\n");
    println!("Demonstrated:");
    println!("  ✓ Functional state threading (immutable)");
    println!("  ✓ Command-based operations");
    println!("  ✓ Hard delete vs tombstone policies");
    println!("  ✓ Atomicity on error");
    println!("  ✓ Operation chaining");
    println!("\nAll {} Ettles in final state:", state.list_ettles().len());
    for ettle in state.list_ettles() {
        println!("  - {} ({})", ettle.title, ettle.id);
    }

    Ok(())
}
