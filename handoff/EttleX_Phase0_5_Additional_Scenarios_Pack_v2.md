# EttleX Phase 0.5 – Additional Scenarios Pack (Refactoring Aid)

This document is intended to be applied as a **refactoring of existing code and tests**.
It contains ONLY additional normative statements, error taxonomy additions, and **happy + error** Gherkin scenarios
covering gaps identified after the Phase 0.5 kernel build.

It does **not** restate your existing Phase 0.5 spec; it extends it.

---

## Normative Structural Statement (Bidirectional EP Membership)

Ettle membership in EPs is canonical and bidirectional:

- `Ettle.ep_ids` is the ordered membership index.
- `EP.ettle_id` is the ownership reference.
- All operations MUST maintain both directions consistently.
- Rendering, traversal (RT/EPT), and export operate ONLY on the active EP set (`deleted == false`).
- Tombstoned EPs MUST NOT appear in active listings or traversal results, but MUST remain available for historical inspection and future diff/provenance work.

---

## Additional Normative Rules (Refinement + Membership + Active Projection)

### R1. EP ownership is immutable (Phase 0.5)

An EP’s `ettle_id` MUST NOT change after creation. Any future “move EP across Ettles” operation MUST be modelled as
create-new-EP + tombstone-old-EP + provenance link (Phase 1+).

### R2. EP ordinal namespace is per-Ettle and immutable

Within a single Ettle:
- EP ordinals MUST be unique among active EPs.
- Ordinals MUST NOT be mutated once assigned.
- Tombstoned EPs do not “free” an ordinal for reuse in Phase 0.5 (recommended), unless you explicitly add a reuse policy.
  If you choose to permit reuse, it MUST be deterministic and validated; otherwise disallow to simplify provenance later.

### R3. Active EP projection is deterministic

Define an explicit projection function:

`active_eps(ettle) := [ep in ettle.ep_ids where ep.deleted == false] ordered by ep.ordinal ascending`

All listing, rendering, traversal, and validation MUST use `active_eps` (not raw `ep_ids`).

### R4. Refinement link validity is evaluated against active EPs only

If an EP `P` links to child Ettle `C` (`P.child_ettle_id = C.id`), then:
- `P.deleted` MUST be false.
- `C.deleted` MUST be false.
- `C.parent_id` MUST equal `P.ettle_id` (single parent invariant).
- The parent’s active EP set MUST contain exactly one EP mapping to `C` (one-to-one parent EP mapping).

### R5. Tombstoning must not strand active children

If deleting (tombstoning) an EP would remove the only mapping from a parent Ettle to an active child Ettle,
the operation MUST fail (unless performed as part of an atomic composite edit that also tombstones/reparents the child — deferred).

---

## Error Taxonomy Additions (Validation + Projection)

These error types are intended to be added to your Phase 0.5 error taxonomy.

### Membership / Projection

- `ERR_MEMBERSHIP_INCONSISTENT` — EP listed in an Ettle but EP.ettle_id points elsewhere
- `ERR_EP_ORPHANED` — EP.ettle_id points to Ettle but Ettle.ep_ids does not include EP
- `ERR_ACTIVE_EP_ORDER_NONDETERMINISTIC` — active_eps projection differs across runs for identical inputs
- `ERR_EP_ORDINAL_REUSE_FORBIDDEN` — create_ep attempted to reuse an ordinal that previously existed (if reuse forbidden)

### Refinement integrity

- `ERR_CHILD_WITHOUT_EP_MAPPING` — child parent_id set but no parent active EP maps to the child
- `ERR_DUPLICATE_CHILD_MAPPING` — more than one parent active EP maps to the same child
- `ERR_MAPPING_REFERENCES_DELETED_EP` — a child mapping is stored on a tombstoned EP
- `ERR_MAPPING_REFERENCES_DELETED_CHILD` — mapping points at a tombstoned child
- `ERR_TOMBSTONE_STRANDS_CHILD` — attempted EP deletion would strand an active child (violates R5)

### Validation structural

- `ERR_INVALID_PARENT_POINTER` — child.parent_id points to missing/tombstoned parent
- `ERR_EP_LIST_CONTAINS_UNKNOWN_ID` — Ettle.ep_ids references a missing EP
- `ERR_EP_OWNERSHIP_POINTS_TO_UNKNOWN_ETTLE` — EP.ettle_id references missing Ettle

---

## Validate_tree: Deterministic Validation Contract (Checklist)

`validate_tree()` MUST be deterministic and MUST validate, at minimum:

1. All Ettles and EPs referenced by IDs exist (or are explicitly tombstoned and excluded).
2. Bidirectional membership consistency:
   - For every active EP: EP.ettle_id exists and that Ettle.ep_ids includes EP
   - For every active EP ID in Ettle.ep_ids: EP exists and EP.ettle_id == Ettle.id
3. Active EP projection determinism (ordering rule applied consistently).
4. Parent chain integrity (parent_id points to existing non-tombstoned Ettle).
5. No multiple parents (each child has at most one parent_id).
6. Refinement mapping integrity (R4): one-to-one mapping from parent active EP set to each child.
7. Deletion safety (R5): tombstoning rules do not strand active children (checked on delete operations and optionally by validation).

---

# Scenarios

## 1. Create Ettle With Metadata

### Happy Path

```gherkin
Feature: Create Ettle With Metadata

  Scenario: Create ettle with valid metadata
    Given no existing ettle with title "Snapshot Engine"
    When ettle.create is called with:
      | title    | Snapshot Engine |
      | metadata | {"layer":"core","owner":"platform"} |
    Then an ettle is created
    And the ettle contains EP0
    And EP0.deleted is false
    And read_ettle returns metadata {"layer":"core","owner":"platform"}
    And validate_tree returns success
```

### Error Paths

```gherkin
  Scenario: Create ettle with invalid metadata type
    When ettle.create is called with metadata that is not valid JSON
    Then ERR_INVALID_METADATA is raised

  Scenario: Create ettle with duplicate title constraint
    Given an existing ettle titled "Snapshot Engine"
    When ettle.create is called with title "Snapshot Engine"
    Then ERR_DUPLICATE_ETTLE_TITLE is raised
```

---

## 2. Create Ettle With Explicit EP0 Content (WHY/WHAT/HOW)

### API Semantics

- `ettle.create(title, metadata?, why, what, how)` creates Ettle with EP0 populated.
- `ettle.update(ettle_id, ep_ordinal, why?, what?, how?)` updates EP content.

### Happy Path

```gherkin
Feature: Create Ettle With EP0 Content

  Scenario: Create ettle with WHY WHAT HOW
    When ettle.create is called with:
      | title | Snapshot Commit |
      | why   | Persist semantic state |
      | what  | Store deterministic manifest |
      | how   | Narrative steps for commit |
    Then EP0 is created
    And EP0.why equals "Persist semantic state"
    And EP0.what equals "Store deterministic manifest"
    And EP0.how equals "Narrative steps for commit"
    And EP0.ordinal equals 0
    And EP0.deleted is false
```

### Error Paths

```gherkin
  Scenario: Missing HOW section
    When ettle.create is called without how
    Then ERR_INVALID_HOW is raised

  Scenario: Empty WHAT section
    When ettle.create is called with empty what
    Then ERR_INVALID_WHAT is raised
```

(Note: Phase 0.5 does NOT require HOW to contain Gherkin except where your own leaf-gating rules explicitly apply.)

---

## 3. Add EP To Ettle (and list it via active_eps)

### Happy Path

```gherkin
Feature: Add EP To Ettle

  Scenario: Add EP1 to ettle and list in active EPs
    Given an existing ettle E1 with EP0
    When create_ep is called on E1 with ordinal 1 and content:
      | why  | Additional rationale |
      | what | Additional condition |
      | how  | Additional method |
    Then EP1 is created
    And EP1.ettle_id equals E1
    And E1.ep_ids includes EP1
    And active_eps(E1) lists [EP0, EP1] in ordinal order
    And validate_tree returns success
```

### Error Paths

```gherkin
  Scenario: Duplicate ordinal among active EPs
    Given E1 already has an active EP with ordinal 1
    When create_ep is called on E1 with ordinal 1
    Then ERR_DUPLICATE_EP_ORDINAL is raised

  Scenario: Add EP to tombstoned ettle
    Given E1.deleted is true
    When create_ep is called on E1 with ordinal 1
    Then ERR_ETTLE_DELETED is raised
```

---

## 4. Remove (Tombstone) EP From Ettle (and exclude from active listing)

### Happy Path

```gherkin
Feature: Tombstone EP and exclude from active listing

  Scenario: Tombstone EP1 and it disappears from active_eps
    Given E1 has EP0 and EP1 and EP1 has no child mapping
    When delete_ep is called for EP1
    Then EP1.deleted is true
    And active_eps(E1) lists [EP0] only
    And validate_tree returns success
```

### Error Paths

```gherkin
  Scenario: Delete EP that carries the only mapping to an active child
    Given EP1 maps to active child ettle C1
    And EP1 is the only active EP in E1 that maps to C1
    When delete_ep is called for EP1
    Then ERR_TOMBSTONE_STRANDS_CHILD is raised

  Scenario: Delete EP0 when forbidden
    Given the system forbids deleting EP0
    When delete_ep is called for EP0
    Then ERR_CANNOT_DELETE_EP0 is raised
```

---

## 5. Membership Integrity: Bidirectional Linkage

### Happy Paths

```gherkin
Feature: Bidirectional Membership Integrity

  Scenario: EP ownership and listing consistent
    Given EP1.ettle_id = E1
    And E1.ep_ids includes EP1
    When validate_tree is executed
    Then validation succeeds
```

### Error Paths

```gherkin
  Scenario: EP listed but ownership mismatch
    Given E1.ep_ids includes EP1
    And EP1.ettle_id = E2
    When validate_tree is executed
    Then ERR_MEMBERSHIP_INCONSISTENT is raised

  Scenario: EP orphaned (no listing in owning ettle)
    Given EP1.ettle_id = E1
    And E1.ep_ids does not include EP1
    When validate_tree is executed
    Then ERR_EP_ORPHANED is raised
```

---

## 6. Cross-Ettle Refinement Invariants (Parent/Child + EP mapping)

### Happy Path

```gherkin
Feature: Refinement link integrity

  Scenario: Child is reachable via exactly one active parent EP mapping
    Given parent ettle P exists and is active
    And child ettle C exists and is active
    And C.parent_id equals P.id
    And P has active EP1 with child_ettle_id = C.id
    When validate_tree is executed
    Then validation succeeds
```

### Error Paths

```gherkin
  Scenario: Child has parent pointer but no active EP mapping exists
    Given child ettle C has parent_id = P.id
    And P has no active EP whose child_ettle_id equals C.id
    When validate_tree is executed
    Then ERR_CHILD_WITHOUT_EP_MAPPING is raised

  Scenario: Duplicate mappings from parent to same child
    Given P has active EP1 with child_ettle_id = C.id
    And P has active EP2 with child_ettle_id = C.id
    When validate_tree is executed
    Then ERR_DUPLICATE_CHILD_MAPPING is raised

  Scenario: Mapping references tombstoned EP
    Given EP1.deleted is true
    And EP1.child_ettle_id = C.id
    When validate_tree is executed
    Then ERR_MAPPING_REFERENCES_DELETED_EP is raised

  Scenario: Mapping references tombstoned child
    Given EP1.deleted is false
    And EP1.child_ettle_id = C.id
    And child ettle C.deleted is true
    When validate_tree is executed
    Then ERR_MAPPING_REFERENCES_DELETED_CHILD is raised
```

---

## 7. EP Ordinal Revalidation Rules

### Happy Path

```gherkin
Feature: EP ordinal uniqueness and immutability

  Scenario: Ordinals are unique and stable
    Given E1 has EP0 ordinal 0 and EP1 ordinal 1 and both are active
    When I read E1 and its active_eps
    Then the ordinals are [0, 1] and remain unchanged
```

### Error Paths

```gherkin
  Scenario: Create EP reuses ordinal when reuse is forbidden
    Given EP1 ordinal 1 existed previously and is tombstoned
    And the system forbids ordinal reuse in Phase 0.5
    When create_ep is called on E1 with ordinal 1
    Then ERR_EP_ORDINAL_REUSE_FORBIDDEN is raised

  Scenario: Attempt to mutate an EP ordinal
    Given EP1 exists with ordinal 1
    When update_ep attempts to set EP1.ordinal = 2
    Then ERR_EP_ORDINAL_IMMUTABLE is raised
```

---

## 8. Deterministic Active EP Projection

### Happy Path

```gherkin
Feature: Deterministic active_eps projection

  Scenario: active_eps is stable and sorted by ordinal
    Given E1.ep_ids contains [EP1, EP0] (out of order storage)
    And EP0.ordinal = 0 and EP1.ordinal = 1 and both are active
    When active_eps(E1) is computed twice
    Then both results are identical and ordered [EP0, EP1]
```

### Error Paths

```gherkin
  Scenario: active_eps includes tombstoned EP
    Given EP1.deleted is true
    And E1.ep_ids includes EP1
    When active_eps(E1) is computed
    Then ERR_ACTIVE_EP_INCLUDES_DELETED is raised

  Scenario: active_eps non-determinism is detected
    Given identical in-memory state is loaded twice
    When active_eps(E1) is computed
    Then outputs are byte-identical
    And if not, ERR_ACTIVE_EP_ORDER_NONDETERMINISTIC is raised
```

---

## 9. EPT / Mapping sensitivity to membership inconsistencies (defensive)

### Happy Path

```gherkin
Feature: EPT uses active_eps and refuses inconsistent membership

  Scenario: EPT computes when membership is consistent
    Given an RT exists Root -> Leaf
    And Root has active EP1 mapping to Leaf
    And membership invariants hold for Root and Leaf EPs
    When EPT is computed for Leaf
    Then EPT is produced deterministically
```

### Error Paths

```gherkin
  Scenario: EPT fails when an EP referenced in ep_ids is missing
    Given E1.ep_ids contains an EP id that does not exist
    When validate_tree is executed
    Then ERR_EP_LIST_CONTAINS_UNKNOWN_ID is raised

  Scenario: EPT fails when a parent mapping EP is orphaned
    Given Root EP1 exists and is active
    And EP1.ettle_id = Root.id
    And Root.ep_ids does not include EP1
    When EPT is computed for Leaf
    Then ERR_EP_ORPHANED is raised
```

---

## 10. Deletion Safety: Do not strand active children (R5)

### Happy Path

```gherkin
Feature: Tombstone does not strand children

  Scenario: Delete EP that does not carry required mapping succeeds
    Given parent P has two EPs EP1 and EP2
    And EP1 maps to child C
    And EP2 has no child mapping
    When delete_ep is called for EP2
    Then EP2.deleted is true
    And child C remains reachable via EP1
    And validate_tree returns success
```

### Error Paths

```gherkin
  Scenario: Delete the only mapping EP fails
    Given parent P has EP1 mapping to child C
    And EP1 is the only active EP mapping to C
    When delete_ep is called for EP1
    Then ERR_TOMBSTONE_STRANDS_CHILD is raised

  Scenario: Delete child ettle without updating parent mapping is rejected
    Given parent P has active EP1 mapping to child C
    When delete_ettle is called for child C only
    Then ERR_DELETE_REFERENCED_CHILD is raised
```

---

## Implementation Notes for Refactoring

- If your existing code stores `ep_ids` but does not keep it consistent, treat `validate_tree` as the enforcement point and drive fixes until all scenarios pass.
- If your existing code treats “delete EP” as hard delete, refactor to tombstone semantics and update active projection accordingly.
- If your existing EPT uses raw ep_ids ordering, refactor to use `active_eps` and sort by ordinal.
