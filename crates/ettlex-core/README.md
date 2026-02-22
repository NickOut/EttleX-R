# ettlex-core

**EttleX Core - Phase 0.5: Canonical In-Memory Semantic Kernel**

Pure Rust library providing the foundational data structures and operations for EttleX architectural transformation.

## Overview

EttleX Core implements a semantic modeling system based on:

- **Ettles**: Fundamental units representing architectural concepts/decisions
- **EPs (Ettle Partitions)**: Refinement relationships with WHY/WHAT/HOW semantics
- **Active EP Projection**: Deterministic filtering of non-deleted EPs with ordinal ordering
- **Deterministic Traversals**: RT (Refinement Traversal) and EPT (EP Traversal)
- **Tree Validation**: Comprehensive invariant checking with 7 mandatory requirements
- **Markdown Rendering**: Export to human-readable format

### Normative Rules (R1-R5)

EttleX Core enforces five normative requirements:

1. **R1: Bidirectional Membership** - Every EP's `ettle_id` must match the owning Ettle's `ep_ids` list
2. **R2: Ordinal Immutability** - EP ordinals cannot be changed or reused (even from deleted EPs)
3. **R3: Active EP Projection** - `active_eps()` returns only non-deleted EPs, sorted by ordinal
4. **R4: Refinement Integrity** - Parent-child relationships must have valid EP mappings
5. **R5: Deletion Safety** - EP0 cannot be deleted; deleting an EP cannot strand children

## Features

✅ **Full CRUD Operations**

- Create, read, update, delete for Ettles and EPs
- Tombstone deletion (soft delete) for referential integrity
- UUID v7 auto-generation for time-ordered IDs

✅ **Refinement Graph**

- Parent-child relationships with `set_parent()`
- One-to-one EP→child mapping with `link_child()`
- DFS cycle detection prevents invalid trees
- Deterministic child ordering by EP ordinal

✅ **Tree Validation**

- Comprehensive `validate_tree()` with invariant checking
- Detects cycles, orphans, duplicate mappings
- No-panic error handling with typed errors

✅ **Deterministic Traversals**

- **RT (Refinement Traversal)**: Root-to-leaf path following parent pointers
- **EPT (EP Traversal)**: Ordered EP sequence from root EP0 to leaf

✅ **Markdown Rendering**

- `render_ettle()`: Individual Ettle with all EPs
- `render_leaf_bundle()`: Aggregated WHY/WHAT/HOW from root to leaf

✅ **Snapshot Manifests**

- `generate_manifest()`: Create canonical snapshot with EPT state
- Constraints envelope with extensible family structure
- Deterministic digest computation (semantic + temporal)
- Ready for persistence to content-addressable storage

## Installation

Add to your `Cargo.toml`:

```toml
[dependencies]
ettlex-core = { path = "../ettlex-core" }
```

## Quick Start

```rust
use ettlex_core::{
    ops::{ettle_ops, ep_ops, refinement_ops},
    render, traversal, Store,
};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Create store
    let mut store = Store::new();

    // Create Ettles with EP0 content
    let root = ettle_ops::create_ettle(
        &mut store,
        "Root".to_string(),
        None,  // metadata
        Some("Why this root exists".to_string()),
        Some("Root concept description".to_string()),
        Some("Root implementation".to_string()),
    )?;
    let child = ettle_ops::create_ettle(
        &mut store,
        "Child".to_string(),
        None, None, None, None,
    )?;

    // Create EP with WHY/WHAT/HOW
    let ep = ep_ops::create_ep(
        &mut store,
        &root,
        1,
        true,
        "Why: Rationale".to_string(),
        "What: Description".to_string(),
        "How: Implementation".to_string(),
    )?;

    // Link them
    refinement_ops::link_child(&mut store, &ep, &child)?;

    // Compute traversals
    let rt = traversal::rt::compute_rt(&store, &child)?;
    let ept = traversal::ept::compute_ept(&store, &child, None)?;

    // Render to Markdown
    let output = render::render_leaf_bundle(&store, &child, None)?;
    println!("{}", output);

    Ok(())
}
```

See `examples/comprehensive_demo.rs` for a complete example using the mutation-based API.

## Functional-Boundary API (New)

EttleX Core now supports a functional-boundary style API via the `apply()` function, which provides atomic, immutable state transitions.

### Why Use `apply()`?

- **Atomicity**: All-or-nothing updates (returns fully valid state or error)
- **Immutability**: State threading instead of mutation
- **Testability**: Easier to reason about state changes
- **Safety**: Never panics for invalid input

### Quick Start with `apply()`

```rust
use ettlex_core::{
    apply, Command, Store, policy::NeverAnchoredPolicy,
};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Create empty state
    let state = Store::new();
    let policy = NeverAnchoredPolicy;

    // Apply commands to create and transform state
    let state = apply(
        state,
        Command::EttleCreate {
            title: "API Gateway".to_string(),
            metadata: None,
            why: Some("Entry point".to_string()),
            what: Some("HTTP handler".to_string()),
            how: Some("Load balancer".to_string()),
        },
        &policy,
    )?;

    // State threading: old state moved, new state returned
    let state = apply(
        state,
        Command::EttleCreate {
            title: "Auth Service".to_string(),
            metadata: None,
            why: None,
            what: None,
            how: None,
        },
        &policy,
    )?;

    // Final state contains both Ettles
    assert_eq!(state.list_ettles().len(), 2);

    Ok(())
}
```

See `examples/apply_demo.rs` for a comprehensive demonstration.

### Anchored Deletion

The `apply()` API introduces **policy-gated deletion** for EPs:

```rust
use ettlex_core::{
    apply, Command, Store,
    policy::{NeverAnchoredPolicy, SelectedAnchoredPolicy},
};
use std::collections::HashSet;

// Hard delete (churn mode): EP completely removed from storage
let policy = NeverAnchoredPolicy;
let state = apply(state, Command::EpDelete { ep_id }, &policy)?;
// EP no longer exists in storage

// Tombstone (anchored mode): EP preserved with deleted=true flag
let mut anchored_eps = HashSet::new();
anchored_eps.insert(ep_id.clone());
let policy = SelectedAnchoredPolicy::with_eps(anchored_eps);
let state = apply(state, Command::EpDelete { ep_id }, &policy)?;
// EP still exists but is marked as deleted
```

**When to use each:**

- **Hard delete** (`NeverAnchoredPolicy`): Prototyping/design phase - low-value artifacts
- **Tombstone** (`SelectedAnchoredPolicy`): Published/anchored EPs - preserve history

**Note:** Ettle deletion remains tombstone-only in Phase 0.5 (hard delete not supported).

### Commands

All Phase 0.5 operations are available as commands:

- `Command::EttleCreate` - Create new Ettle
- `Command::EttleUpdate` - Update title/metadata
- `Command::EttleDelete` - Delete Ettle (tombstone only)
- `Command::EpCreate` - Create new EP
- `Command::EpUpdate` - Update EP content
- `Command::EpDelete` - Delete EP (policy-gated)
- `Command::RefineLinkChild` - Link child Ettle to parent EP
- `Command::RefineUnlinkChild` - Unlink child from parent EP

### Choosing an API Style

Both APIs are fully supported:

**Mutation API** (`&mut Store`):

- Traditional, familiar style
- Good for simple scripts and learning
- Example: `examples/comprehensive_demo.rs`

**Functional API** (`apply(state, cmd, policy)`):

- Atomic, immutable transformations
- Better for complex operations
- Built-in policy injection
- Example: `examples/apply_demo.rs`

You can mix both styles in the same codebase.

## API Documentation

Generate and view full API documentation:

```bash
cargo doc -p ettlex-core --open
```

**Documentation location:**

The docs are generated in a platform-specific subdirectory due to the `build.target` setting in `~/.cargo/config.toml`:

```
target/aarch64-apple-darwin/doc/ettlex_core/index.html
```

This is standard Cargo behavior when an explicit build target is configured (enables cross-compilation support). If your system uses a different target or no explicit target, the docs will be at `target/doc/ettlex_core/index.html`.

## Architecture

```
ettlex-core/
├── errors.rs          # EttleXError enum (42 variants)
├── model/             # Data structures
│   ├── ettle.rs       # Ettle: id, title, parent_id, ep_ids, metadata
│   ├── ep.rs          # EP: id, ordinal, child_ettle_id, why/what/how
│   └── metadata.rs    # Extensible key-value storage
├── ops/               # Operations
│   ├── store.rs       # In-memory HashMap storage
│   ├── ettle_ops.rs   # create/read/update/delete Ettle
│   ├── ep_ops.rs      # create/read/update/delete EP
│   ├── projection.rs  # active_eps() - deterministic EP filtering
│   └── refinement_ops.rs  # set_parent, link_child, unlink_child
├── rules/             # Validation
│   ├── validation.rs  # validate_tree() - 7 mandatory checks
│   └── invariants.rs  # 14 invariant detection functions
├── traversal/         # Algorithms
│   ├── rt.rs          # compute_rt() - root to leaf
│   └── ept.rs         # compute_ept() - ordered EPs (uses active_eps)
├── snapshot/          # Manifest generation
│   ├── manifest.rs    # Snapshot manifest with constraints envelope
│   └── digest.rs      # Deterministic digest computation
├── render/            # Export
│   ├── ettle_render.rs    # render_ettle() (uses active_eps)
│   └── bundle_render.rs   # render_leaf_bundle()
├── logging_facility/  # Structured logging
│   ├── macros.rs      # log_op_start!, log_op_end!, log_op_error!
│   ├── test_capture.rs # Test utilities for logging verification
│   └── mod.rs         # Facility initialization and configuration
├── coverage/          # Test coverage analysis (stub)
├── diff/              # Snapshot diff engine (stub)
└── tes/               # Test Evidence Schema generation (stub)
```

## Testing

Run tests:

```bash
cargo test -p ettlex-core
```

**161 tests** covering:

- **40 Gherkin scenario tests** (10 scenario files from Additional Scenarios Pack v2)
- **25 Functional-boundary tests** (3 refactor-specific test suites)
  - Metadata handling and EP0 content validation
  - Active EP projection and deterministic ordering
  - Membership integrity and bidirectional consistency
  - Refinement invariants and mapping constraints
  - EP ordinal immutability and reuse prevention
  - Deletion safety (EP0 protection, strand prevention)
  - EPT mapping sensitivity
- **14 Snapshot tests** (NEW)
  - Manifest generation with constraints envelope
  - Deterministic digest computation
  - Semantic vs temporal digest separation
  - Constraints envelope structure and ordering
- **14 Ettle CRUD tests**
- **12 EP CRUD tests** (including ordinal reuse and EP0 deletion)
- **14 Refinement operation tests**
- **13 Tree validation tests** (enhanced with new invariants)
- **7 RT traversal tests**
- **9 EPT traversal tests** (updated to use active_eps)
- **5 Rendering tests** (updated to use active_eps)
- **8 Unit tests** (model, store, projection, invariants)

## Coverage

Generate coverage report:

```bash
make coverage-check    # Verify >80% coverage
make coverage-html     # Generate HTML report
```

**92.71% line coverage** (exceeds 80% minimum)

## Error Handling

All operations return `Result<T, EttleXError>`. Key error categories:

**Entity Errors:**

- `EttleNotFound` / `EpNotFound`: Entity doesn't exist
- `EttleDeleted` / `EpDeleted`: Entity was tombstoned

**Validation Errors:**

- `InvalidTitle`: Empty or whitespace-only title
- `InvalidWhat` / `InvalidHow`: Empty EP content strings
- `CycleDetected`: Operation would create a cycle

**Membership & Projection Errors (New in Pack v2):**

- `MembershipInconsistent`: EP.ettle_id doesn't match owning Ettle
- `EpOrphaned`: EP points to Ettle but isn't in ep_ids list
- `EpListContainsUnknownId`: Ettle references non-existent EP
- `EpOwnershipPointsToUnknownEttle`: EP references non-existent Ettle
- `ActiveEpOrderNonDeterministic`: active_eps() ordering violated

**Refinement Errors (New in Pack v2):**

- `ChildWithoutEpMapping`: Child has parent but no EP maps to it
- `ChildReferencedByMultipleEps`: Multiple EPs map to same child
- `MappingReferencesDeletedEp`: Active mapping uses deleted EP
- `MappingReferencesDeletedChild`: EP maps to deleted child

**Deletion Safety Errors (New in Pack v2):**

- `CannotDeleteEp0`: EP0 (ordinal 0) is protected
- `TombstoneStrandsChild`: Deleting EP would orphan a child
- `DeleteWithChildren`: Cannot delete Ettle with active children

**Ordinal Errors (New in Pack v2):**

- `EpOrdinalReuseForbidden`: Attempting to reuse tombstoned EP's ordinal
- `OrdinalAlreadyExists`: Ordinal in use by active EP
- `OrdinalImmutable`: Cannot change EP ordinal after creation

**Traversal Errors:**

- `EptAmbiguousLeafEp`: Leaf has multiple EPs, ordinal required
- `EptMissingMapping`: Parent has no EP mapping to child
- `EptDuplicateMapping`: Multiple EPs map to same child

See full error taxonomy (42 variants) in `errors.rs`.

## Active EP Projection

The `active_eps()` function is central to Phase 0.5 Additional Scenarios Pack v2:

```rust
pub fn active_eps<'a>(store: &'a Store, ettle: &Ettle) -> Result<Vec<&'a Ep>>
```

**Purpose:** Returns a deterministic, ordered view of non-deleted EPs for an Ettle.

**Guarantees:**

1. **Filters deleted EPs** - Only returns EPs where `deleted == false`
2. **Deterministic ordering** - Sorted by `ordinal` ascending
3. **Membership validation** - Verifies bidirectional consistency (R1)
4. **Stable across calls** - Same input always produces same output

**Usage:** All traversal, rendering, and refinement operations use `active_eps()` instead of accessing `ettle.ep_ids` directly. This ensures deleted EPs are never included in computations.

## Design Decisions

### UUID v7

Auto-generated time-ordered IDs for debugging and determinism.

### In-Memory Storage

Simple `HashMap<String, T>` - fast, single-threaded, suitable for Phase 0.5.

### Tombstone Deletion

`deleted: bool` flag preserves referential integrity without cascading deletes.

### Ordinal Immutability (R2)

EP ordinals cannot be changed after creation. Tombstoned EPs do not free their ordinals for reuse. This prevents:

- Ambiguity in historical references
- Reordering side effects
- Confusion in EPT computation

### Deletion Safety (R5)

- **EP0 Protection**: EP0 (ordinal 0) cannot be deleted - it represents the Ettle's core identity
- **Strand Prevention**: Deleting an EP fails if it's the only active mapping to a child

### Fail-Fast Validation

Operations enforce critical invariants (e.g., cycle detection in `set_parent`, ordinal reuse in `create_ep`).
Comprehensive validation available via `validate_tree()` with 7 mandatory checks.

## Phase 0.5 Scope

**In Scope:**

- ✅ Pure library (no CLI/binaries)
- ✅ In-memory storage (no persistence)
- ✅ Full CRUD with validation
- ✅ Deterministic traversals
- ✅ Markdown rendering
- ✅ Snapshot manifest generation

**Out of Scope (Future Phases):**

- ❌ Persistence layer (SQLite, CAS) - handled by ettlex-store
- ❌ CLI/binaries - handled by ettlex-cli
- ❌ Constraint evaluation (runtime)
- ❌ Ettle/EP splitting or combining
- ❌ Undo/rollback

## Contributing

This is Phase 0.5 of the EttleX project. See specifications:

- `handoff/EttleX_Phase0_5_Entry_Ettle.md` - Core Phase 0.5 specification
- `handoff/EttleX_Phase0_5_Additional_Scenarios_Pack_v2.md` - Enhanced validation and scenarios

## License

(To be determined)

## Acknowledgments

Phase 0.5+ implementation following strict TDD methodology:

- **161 tests** across 24 test files
- **40 new Gherkin scenarios** from Additional Scenarios Pack v2
- **14 snapshot tests** for manifest generation and constraints envelope
- **12 new error types** for comprehensive error handling
- **5 normative rules** (R1-R5) with enforcement
- **7 mandatory validation checks** in `validate_tree()`
- **14 invariant detection functions** for tree integrity

All tests follow RED → GREEN → REFACTOR discipline.
