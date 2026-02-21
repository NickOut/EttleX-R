//! Comprehensive Demo of Phase 0.5 Additional Scenarios Pack v2
//!
//! This example demonstrates all key features:
//! - Creating Ettles with metadata and EP0 content
//! - Active EP projection (R3)
//! - Bidirectional membership integrity (R1)
//! - Ordinal immutability (R2)
//! - Refinement integrity (R4)
//! - Deletion safety (R5)
//! - Tree validation
//! - Traversal computation (RT/EPT)
//! - Markdown rendering

use ettlex_core::{
    ops::{active_eps, ep_ops, ettle_ops, refinement_ops, Store},
    render, rules,
    traversal::{compute_ept, compute_rt},
};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("╔══════════════════════════════════════════════════════════╗");
    println!("║  EttleX Phase 0.5 - Comprehensive Feature Demo          ║");
    println!("╚══════════════════════════════════════════════════════════╝\n");

    let mut store = Store::new();

    // ═══════════════════════════════════════════════════════════
    // SECTION 1: Create Root Ettle with Metadata and EP0 Content
    // ═══════════════════════════════════════════════════════════
    println!("📦 SECTION 1: Creating Root Ettle with Rich Content\n");

    let mut metadata = std::collections::HashMap::new();
    metadata.insert("author".to_string(), "Alice".into());
    metadata.insert("version".to_string(), "1.0".into());
    metadata.insert("priority".to_string(), "high".into());

    let root_id = ettle_ops::create_ettle(
        &mut store,
        "System Architecture".to_string(),
        Some(metadata.into()),
        Some("We need a scalable microservices architecture".to_string()),
        Some("Event-driven system with independent services".to_string()),
        Some("Use Kafka for event bus, Docker for containerization".to_string()),
    )?;

    println!("✓ Created root Ettle: {}", root_id);
    let root = store.get_ettle(&root_id)?;
    println!("  Title: {}", root.title);
    println!("  Metadata: {:?}", root.metadata);
    println!();

    // ═══════════════════════════════════════════════════════════
    // SECTION 2: Active EP Projection (R3 Requirement)
    // ═══════════════════════════════════════════════════════════
    println!("🔍 SECTION 2: Active EP Projection (R3)\n");

    // Add multiple EPs to demonstrate active projection
    let ep1_id = ep_ops::create_ep(
        &mut store,
        &root_id,
        1,
        true,
        "Separate concerns for maintainability".to_string(),
        "API Gateway service".to_string(),
        "Implement with Kong or Nginx".to_string(),
    )?;

    let ep2_id = ep_ops::create_ep(
        &mut store,
        &root_id,
        2,
        false,
        "Optional: improve observability".to_string(),
        "Logging service".to_string(),
        "Use ELK stack".to_string(),
    )?;

    let ep3_id = ep_ops::create_ep(
        &mut store,
        &root_id,
        5, // Note: gap in ordinals is allowed
        true,
        "Data persistence layer".to_string(),
        "Database service".to_string(),
        "PostgreSQL with read replicas".to_string(),
    )?;

    println!("✓ Created EPs with ordinals: 1, 2, 5");

    // Demonstrate active_eps() - deterministic, ordinal-sorted
    let root = store.get_ettle(&root_id)?;
    let active = active_eps(&store, root)?;

    println!("\n📋 Active EPs (ordinal-sorted, deleted filtered):");
    for ep in &active {
        println!(
            "  EP{}: {} (normative: {})",
            ep.ordinal, ep.what, ep.normative
        );
    }
    println!("  Total active EPs: {}", active.len());
    println!();

    // ═══════════════════════════════════════════════════════════
    // SECTION 3: Ordinal Immutability (R2 Requirement)
    // ═══════════════════════════════════════════════════════════
    println!("🔒 SECTION 3: Ordinal Immutability (R2)\n");

    // Try to reuse ordinal 2 - should fail
    println!("❌ Attempting to create EP with duplicate ordinal 2...");
    match ep_ops::create_ep(
        &mut store,
        &root_id,
        2, // Duplicate!
        false,
        "".to_string(),
        "Duplicate".to_string(),
        "".to_string(),
    ) {
        Err(e) => println!("   ✓ Correctly rejected: {}\n", e),
        Ok(_) => println!("   ✗ ERROR: Should have been rejected!\n"),
    }

    // Delete EP2 and try to reuse its ordinal
    ep_ops::delete_ep(&mut store, &ep2_id)?;
    println!("✓ Deleted EP2 (tombstoned)");

    println!("❌ Attempting to reuse tombstoned ordinal 2...");
    match ep_ops::create_ep(
        &mut store,
        &root_id,
        2, // Reuse tombstoned ordinal
        false,
        "".to_string(),
        "Reuse attempt".to_string(),
        "".to_string(),
    ) {
        Err(e) => println!("   ✓ Correctly rejected: {}\n", e),
        Ok(_) => println!("   ✗ ERROR: Should have been rejected!\n"),
    }

    // Verify EP2 is gone from active projection
    let root = store.get_ettle(&root_id)?;
    let active = active_eps(&store, root)?;
    println!("📋 Active EPs after deletion:");
    for ep in &active {
        println!("  EP{}: {}", ep.ordinal, ep.what);
    }
    println!();

    // ═══════════════════════════════════════════════════════════
    // SECTION 4: Refinement Integrity (R4 Requirement)
    // ═══════════════════════════════════════════════════════════
    println!("🌳 SECTION 4: Refinement Integrity (R4)\n");

    // Create child Ettles and link them properly
    let api_gateway_id = ettle_ops::create_ettle(
        &mut store,
        "API Gateway Service".to_string(),
        None,
        Some("Centralized entry point for all API requests".to_string()),
        Some("HTTP/gRPC gateway with auth and rate limiting".to_string()),
        Some("Deploy Kong with JWT plugin".to_string()),
    )?;

    let database_id = ettle_ops::create_ettle(
        &mut store,
        "Database Service".to_string(),
        None,
        Some("Persistent storage for application data".to_string()),
        Some("PostgreSQL cluster with automatic failover".to_string()),
        Some("Use Patroni for HA, pgBouncer for connection pooling".to_string()),
    )?;

    // Link via EPs - this sets parent_id AND creates EP mapping
    refinement_ops::link_child(&mut store, &ep1_id, &api_gateway_id)?;
    refinement_ops::link_child(&mut store, &ep3_id, &database_id)?;

    println!("✓ Linked API Gateway via EP1");
    println!("✓ Linked Database via EP5");
    println!();

    // Demonstrate refinement validation
    let validation_result = rules::validation::validate_tree(&store);
    match validation_result {
        Ok(_) => println!("✓ Tree validation passed (all 7 checks)\n"),
        Err(e) => println!("✗ Validation failed: {}\n", e),
    }

    // ═══════════════════════════════════════════════════════════
    // SECTION 5: Deletion Safety (R5 Requirement)
    // ═══════════════════════════════════════════════════════════
    println!("🛡️  SECTION 5: Deletion Safety (R5)\n");

    // Try to delete EP0 - should fail
    let ep0_id = {
        let root = store.get_ettle(&root_id)?;
        root.ep_ids[0].clone()
    };

    println!("❌ Attempting to delete EP0 (protected)...");
    match ep_ops::delete_ep(&mut store, &ep0_id) {
        Err(e) => println!("   ✓ Correctly rejected: {}\n", e),
        Ok(_) => println!("   ✗ ERROR: Should have been rejected!\n"),
    }

    // Try to delete EP1 (only mapping to API Gateway) - should fail
    println!("❌ Attempting to delete EP1 (only mapping to child)...");
    match ep_ops::delete_ep(&mut store, &ep1_id) {
        Err(e) => println!("   ✓ Correctly rejected: {}\n", e),
        Ok(_) => println!("   ✗ ERROR: Should have been rejected!\n"),
    }

    // Demonstrate safe deletion: unlink first, then can delete
    println!("✓ Unlinking child from EP1...");
    refinement_ops::unlink_child(&mut store, &ep1_id)?;

    println!("✓ Now EP1 can be deleted (no longer maps to a child)...");
    ep_ops::delete_ep(&mut store, &ep1_id)?;
    println!("  Deletion succeeded!\n");

    // Re-link the child for remaining demos
    let ep6_id = ep_ops::create_ep(
        &mut store,
        &root_id,
        6,
        false,
        "Replacement refinement".to_string(),
        "API Gateway service".to_string(),
        "Kong-based implementation".to_string(),
    )?;
    refinement_ops::link_child(&mut store, &ep6_id, &api_gateway_id)?;
    println!("✓ Re-linked API Gateway via new EP6\n");

    // ═══════════════════════════════════════════════════════════
    // SECTION 6: Bidirectional Membership (R1 Requirement)
    // ═══════════════════════════════════════════════════════════
    println!("🔗 SECTION 6: Bidirectional Membership Integrity (R1)\n");

    // Verify membership consistency for API Gateway
    let api_gateway = store.get_ettle(&api_gateway_id)?;
    let api_active = active_eps(&store, api_gateway)?;

    println!("API Gateway Service:");
    println!("  Ettle.ep_ids count: {}", api_gateway.ep_ids.len());
    println!("  Active EPs count: {}", api_active.len());

    for ep in &api_active {
        let ep_points_back = ep.ettle_id == api_gateway.id;
        println!("    EP{}: ettle_id matches? {}", ep.ordinal, ep_points_back);
    }
    println!();

    // ═══════════════════════════════════════════════════════════
    // SECTION 7: Tree Traversal (RT and EPT)
    // ═══════════════════════════════════════════════════════════
    println!("🗺️  SECTION 7: Tree Traversal\n");

    // Compute RT (Refinement Traversal) from root to leaf
    let api_rt = compute_rt(&store, &api_gateway_id)?;
    println!("RT (Root to API Gateway):");
    for (i, ettle_id) in api_rt.iter().enumerate() {
        let ettle = store.get_ettle(ettle_id)?;
        println!("  {}. {}", i + 1, ettle.title);
    }
    println!();

    // Compute EPT (EP Traversal) - active EPs only
    let api_ept = compute_ept(&store, &api_gateway_id, None)?;
    println!("EPT (Root EP0 -> Mapping EP -> Leaf EP0):");
    for ep_id in &api_ept {
        let ep = store.get_ep(ep_id)?;
        println!("  EP{}: {}", ep.ordinal, ep.what);
    }
    println!("  Total EPs in path: {}\n", api_ept.len());

    // ═══════════════════════════════════════════════════════════
    // SECTION 8: Markdown Rendering
    // ═══════════════════════════════════════════════════════════
    println!("📝 SECTION 8: Markdown Rendering\n");

    // Render individual Ettle
    println!("─────────────────────────────────────────────────────");
    println!("Rendering root Ettle:");
    println!("─────────────────────────────────────────────────────");
    let root_md = render::render_ettle(&store, &root_id)?;
    println!("{}", root_md);

    // Render leaf bundle (aggregates WHY/WHAT/HOW from root to leaf)
    println!("─────────────────────────────────────────────────────");
    println!("Rendering API Gateway leaf bundle:");
    println!("─────────────────────────────────────────────────────");
    let bundle_md = render::render_leaf_bundle(&store, &api_gateway_id, None)?;
    println!("{}", bundle_md);

    // ═══════════════════════════════════════════════════════════
    // SECTION 9: Comprehensive Validation
    // ═══════════════════════════════════════════════════════════
    println!("✅ SECTION 9: Comprehensive Tree Validation\n");

    println!("Running all 7 validation checks:");
    println!("  1. All referenced Ettles/EPs exist");
    println!("  2. Bidirectional membership consistency");
    println!("  3. Active EP projection determinism");
    println!("  4. Parent chain integrity");
    println!("  5. No multiple parents");
    println!("  6. Refinement mapping integrity");
    println!("  7. Deletion safety constraints");
    println!();

    match rules::validation::validate_tree(&store) {
        Ok(_) => {
            println!("✓ All validation checks passed!");
            println!("  Tree is structurally sound and semantically valid.\n");
        }
        Err(e) => {
            println!("✗ Validation failed: {}\n", e);
        }
    }

    // ═══════════════════════════════════════════════════════════
    // SECTION 10: Error Taxonomy Demonstration
    // ═══════════════════════════════════════════════════════════
    println!("🚨 SECTION 10: Error Taxonomy Examples\n");

    println!("Testing various error conditions:");

    // InvalidWhat error
    match ettle_ops::create_ettle(
        &mut store,
        "Test".to_string(),
        None,
        None,
        Some("".to_string()), // Empty WHAT
        None,
    ) {
        Err(e) => println!("  ✓ InvalidWhat: {}", e),
        Ok(_) => println!("  ✗ Should have failed"),
    }

    // InvalidHow error
    match ettle_ops::create_ettle(
        &mut store,
        "Test".to_string(),
        None,
        None,
        None,
        Some("".to_string()), // Empty HOW
    ) {
        Err(e) => println!("  ✓ InvalidHow: {}", e),
        Ok(_) => println!("  ✗ Should have failed"),
    }

    // EttleDeleted error
    let temp_id = ettle_ops::create_ettle(&mut store, "Temp".to_string(), None, None, None, None)?;
    ettle_ops::delete_ettle(&mut store, &temp_id)?;
    match store.get_ettle(&temp_id) {
        Err(e) => println!("  ✓ EttleDeleted: {}", e),
        Ok(_) => println!("  ✗ Should have failed"),
    }

    // CannotDeleteEp0 error (already demonstrated above)
    println!("  ✓ CannotDeleteEp0: Cannot delete EP with ordinal 0");

    // TombstoneStrandsChild error (already demonstrated above)
    println!("  ✓ TombstoneStrandsChild: Cannot delete only active mapping");

    println!();

    // ═══════════════════════════════════════════════════════════
    // FINAL SUMMARY
    // ═══════════════════════════════════════════════════════════
    println!("╔══════════════════════════════════════════════════════════╗");
    println!("║                    DEMO COMPLETE                         ║");
    println!("╠══════════════════════════════════════════════════════════╣");
    println!("║  Demonstrated Features:                                  ║");
    println!("║  ✓ R1: Bidirectional Membership                          ║");
    println!("║  ✓ R2: Ordinal Immutability                              ║");
    println!("║  ✓ R3: Active EP Projection                              ║");
    println!("║  ✓ R4: Refinement Integrity                              ║");
    println!("║  ✓ R5: Deletion Safety                                   ║");
    println!("║  ✓ Metadata & EP0 Content                                ║");
    println!("║  ✓ Tree Validation (7 checks)                            ║");
    println!("║  ✓ RT/EPT Traversal                                      ║");
    println!("║  ✓ Markdown Rendering                                    ║");
    println!("║  ✓ Comprehensive Error Handling                          ║");
    println!("╚══════════════════════════════════════════════════════════╝");

    Ok(())
}
