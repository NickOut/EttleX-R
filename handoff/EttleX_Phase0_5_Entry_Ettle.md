# Ettle (Bootstrap Markdown) — Phase 0.5 Entry (Phase 0.5 Deliverables Integrated)
**Product:** EttleX  
**Purpose:** Implement the canonical in-memory Ettle/EP model including full CRUD semantics, deterministic RT/EPT traversal, invariant enforcement, and deterministic rendering functions to support manual review and Phase 1 bootstrapping (no CLI/binaries in Phase 0.5).  
**Tier:** CORE (Bootstrapping)  
**Status:** Draft (bootstrap spec; generator-ready)  
**Created:** 2026-02-19  
**Ettle-ID:** ettle/bootstrap-phase0-5-canonical-model (bootstrap)  
**EPs:** EP0 only (single-leaf implementable)

> This is a **leaf implementable** Ettle used to bootstrap Phase 0.5. It is intended to be handed to an external AI code generator with human oversight to produce the Tests/Code/Docs triad.
>
> **Semantics note:** This Ettle uses the project’s WHY/WHAT/HOW definitions:
> - **WHY** = purpose/rationale
> - **WHAT** = normative condition/feature that must be met
> - **HOW** = method/process to achieve it (**Gherkin scenarios live here**)

---

## EP0 — Canonical Model + Full CRUD + Deterministic RT/EPT + Bootstrap Inspection/Export

### WHY (purpose / rationale)

EttleX is a data-structure-first semantic engine. Before snapshots, constraints, policies, or diffs can exist, we must be able to:
1) represent Ettles and EPs canonically,
2) refine into children (tree links),
3) compute deterministic Refinement Traversals (RT) and EP Traversals (EPT), and
4) **see** what has been authored so we can dogfood (human review + external AI-assisted implementation).

Phase 0.5 explicitly delivers the in-memory semantic kernel needed to bootstrap the rest of EttleX without depending on DB/CAS/snapshots.

This capability prevents earlier failure modes:
- large text artefacts becoming the source of truth,
- invisible in-memory semantics that cannot support human review,
- non-deterministic traversal leading to unstable semantic anchors,
- inability to bootstrap by modelling EttleX inside EttleX.

### WHAT (normative conditions / features)

#### 1. Phase 0.5 Scope Boundary (Non-negotiable)

Phase 0.5 MUST deliver:
- A pure in-memory Ettle/EP model with full CRUD and invariants.
- A deterministic RT/EPT computation module (no DB, no CAS, no snapshots).
- Typed error taxonomy for invalid refinement structures and traversal failures.
- Unit tests building representative trees verifying determinism and error detection.
- Bootstrap inspection/export (pure functions; no CLI/binaries):
    - `render_ettle(ettle_id) -> string` renders WHY/WHAT/HOW (including HOW Gherkin)
    - `render_leaf_bundle(leaf_id, leaf_ep_id?) -> string` renders a realisation bundle Markdown derived from EPT

Phase 0.5 MUST NOT include (explicitly deferred):
- any CLI or binary command surface (Phase 1)
- Ettle splitting / combining
- EP splitting / combining
- subtree reparent preview bundles
- undo / rollback stacks
- persistence (SQLite)
- CAS
- snapshot ledger
- constraint evaluation (ABB/SBB or otherwise)
- policies/profiles
- multi-user collaboration

These are Phase 1+ concerns.

Phase 0.5 Leaf EP Selection: For Phase 0.5, enforce the single-EP leaf rule (only EP0 exists) **OR** require an explicit `leaf_ep_ordinal` parameter when computing EPT/export. Multi-EP leaf handling policy (including default selection rules, normative gating per EP, and leaf EP disambiguation UX) is deferred to Phase 1.

---

#### 2. Canonical entities (in-memory)

##### 2.1 Ettle

Fields (minimum):
- `id` (string; stable, unique; treat as opaque)
- `title` (string)
- `parent_id` (optional string)
- `ep_ids` (ordered list of EP IDs, includes EP0 and any explicit partitions)
- `metadata` (map / JSON-like object; optional; keep for later constraints)
- `created_at`, `updated_at` (timestamps)
- `deleted` (boolean tombstone; default false)

Rules:
- An Ettle MUST have at least one EP (EP0 exists implicitly if not explicitly created).
- An Ettle MUST have at most one parent (except roots).
- Deletion is tombstone only; hard deletion is forbidden.
- An Ettle MUST NOT be considered traversable if tombstoned.

##### 2.2 Ettle Partition (EP)

Fields (minimum):
- `id`
- `ettle_id`
- `ordinal` (int; 0 for EP0)
- `child_ettle_id` (optional string; null if no refinement link)
- `normative` (bool; default true for bootstrap; configurable later)
- `why` (text)
- `what` (text; NOTE: multi-model container comes later; in 0.5 store as text)
- `how` (text; MUST contain Gherkin scenarios for verification)
- `created_at`, `updated_at` (timestamps)
- `deleted` (boolean tombstone; default false)

Rules:
- **All** semantic content MUST reside in EPs (no free-floating Ettle WHY/WHAT/HOW outside EPs).
- EP ordinals MUST be unique per Ettle.
- Ordinals are immutable once assigned; deletion MUST NOT renumber remaining EPs.
- Tombstoned EPs MUST NOT participate in traversal or export.

##### 2.3 Refinement links

A refinement link exists when `EP.child_ettle_id` references a child Ettle.
A refinement edge is defined exclusively by an EP→child link.

---

#### 3. Refinement graph constraints (Phase 0.5)

The refinement structure MUST be a strict **tree**:
- Every Ettle has at most one parent (except a root).
- No cycles.
- Root has no parent.

Mapping constraint:
- If a parent Ettle has **N children**, it MUST have exactly **N explicit EPs** mapping one-to-one to those children.
- EP0 may exist but MUST NOT be used to map multiple children.
- A child MUST NOT be referenced by more than one EP of the same parent.
- A child MUST NOT be referenced by EPs from multiple parents (single-parent rule).

---

#### 4. Full CRUD Operations (in-memory)

Provide an operations layer (library API) with the following functions (names illustrative):

Ettle CRUD:
- `create_ettle(title, metadata?) -> ettle_id`
- `read_ettle(ettle_id) -> Ettle`
- `update_ettle(ettle_id, title?, metadata?) -> Ettle`
- `delete_ettle(ettle_id) -> ()` (tombstone)

EP CRUD:
- `create_ep(ettle_id, ordinal?, why, what, how, normative=true) -> ep_id`
- `read_ep(ep_id) -> EP`
- `update_ep(ep_id, why?, what?, how?, normative?) -> EP`
- `delete_ep(ep_id) -> ()` (tombstone)

Refinement wiring:
- `set_parent(child_ettle_id, parent_ettle_id) -> ()`
- `link_child(parent_ep_id, child_ettle_id) -> ()` (sets `child_ettle_id`)
- `unlink_child(parent_ep_id) -> ()`
- `list_children(ettle_id) -> [ettle_id]` (derived from EP links)

Validation and traversals:
- `validate_tree() -> result`
- `compute_rt(leaf_ettle_id) -> [ettle_id]`
- `compute_ept(leaf_ettle_id, leaf_ep_id?) -> [ep_id]`

Inspection/export:
- `render_ettle(ettle_id) -> string`
- `render_leaf_bundle(leaf_ettle_id, leaf_ep_id?) -> string`

Mutation-time rules (binding):
- `delete_ettle` MUST fail if the ettle has any non-tombstoned children.
- `delete_ep` MUST fail if the EP is currently the sole mapping for an existing child.
- `set_parent` MUST fail if the child already has a parent (unless you explicitly support reparent; Phase 0.5 treats reparent as illegal).
- `link_child` MUST fail if it would:
    - create a cycle,
    - make the child have multiple parents,
    - create duplicate mapping to the same child,
    - violate the one-to-one EP mapping rule.

Phase 0.5 is in-memory only:
- No database persistence required.
- No snapshot ledger required.
- No CAS required.

---

#### 5. Traversals (RT and EPT)

##### 5.1 Refinement Traversal (RT)

Given a leaf Ettle ID, RT is the ordered list of Ettles from root to leaf following parent pointers.

RT MUST fail with a typed error if:
- a parent pointer is missing,
- a cycle is detected,
- a tombstoned Ettle is encountered in the chain.

##### 5.2 EP Traversal (EPT)

Given a leaf Ettle ID, EPT is the ordered list of EPs that connect each parent to the next child along RT, plus a selected leaf EP.

Deterministic selection rules:
- For each non-leaf step, choose the parent EP whose `child_ettle_id` matches the next Ettle in RT.
- If no such EP exists: error (missing mapping).
- If more than one EP matches: error (duplicate mapping).
- For the leaf:
    - If the leaf has exactly one non-tombstoned EP, select it.
    - If the leaf has multiple non-tombstoned EPs, selection MUST be explicit (parameter) or the operation MUST fail with a clear error explaining how to select.

Determinism guarantee:
- Given identical in-memory inputs, `compute_rt` and `compute_ept` MUST return identical sequences in identical order.
- The textual export derived from EPT MUST be byte-for-byte identical across runs.

---

#### 6. Error taxonomy (typed, explicit)

The implementation MUST provide a typed error enum (or equivalent) with at least the following classes.

Structural / validation:
- `ERR_PARENT_NOT_FOUND`
- `ERR_MULTIPLE_PARENTS`
- `ERR_CYCLE_DETECTED`
- `ERR_ROOT_HAS_PARENT`
- `ERR_CHILD_WITHOUT_EP_MAPPING`
- `ERR_DUPLICATE_EP_ORDINAL`
- `ERR_CHILD_REFERENCED_BY_MULTIPLE_EPS`

Traversal:
- `ERR_RT_PARENT_CHAIN_BROKEN`
- `ERR_EPT_MISSING_MAPPING`
- `ERR_EPT_DUPLICATE_MAPPING`
- `ERR_EPT_AMBIGUOUS_LEAF_EP`
- `ERR_EPT_NO_NORMATIVE_EP` (reserved; normative gating becomes relevant later)
- `ERR_DELETED_NODE_IN_TRAVERSAL`

Mutation:
- `ERR_DELETE_WITH_CHILDREN`
- `ERR_DELETE_REFERENCED_EP`
- `ERR_ILLEGAL_REPARENT`

Include comprehensive error types for all EPT computation failure modes: missing mappings, duplicate mappings, cycles, orphaned nodes, and ambiguous leaf selection (including cases caused by tombstoned nodes/EPs).

All errors MUST be:
- deterministic,
- non-panicking,
- test-verifiable via matching error kind (not string matching),
- rendered with a human-readable message (secondary, non-normative).

---

#### 7. Inspection & export for dogfooding (bootstrap)

Phase 0.5 MUST include deterministic, pure **rendering functions** (library API) that provide the minimum visibility needed for human review and Phase 1 bootstrapping. These are not tool surfaces and MUST NOT require a CLI or any binary.

##### 7.1 `render_ettle(ettle_id) -> string`

Returns a stable-order Markdown (or Markdown-like) representation including:
- Ettle ID + title
- parent id (if any)
- EP list in ordinal order
- for each EP: ordinal, normative flag, child link (if present), WHY/WHAT/HOW
- HOW MUST include the full Gherkin blocks (no truncation)

Output MUST be stable-order and suitable for human review and copy/paste into external AI-assisted implementation workflows.

##### 7.2 `render_leaf_bundle(leaf_id, leaf_ep_id?) -> string`

Returns a realisation bundle Markdown derived from EPT.

Bundle MUST include:
- RT (root → leaf) as a list of Ettle IDs + titles
- EPT as a list of EP identifiers (including leaf EP selection)
- Aggregated WHY/WHAT/HOW sections in traversal order
- Full HOW Gherkin scenarios as authored

Rendering MUST fail explicitly on traversal/validation errors and MUST be deterministic (byte-identical for same input).

---

### HOW (method / process, including Gherkin scenarios)

#### Implementation approach (narrative)

1) Implement canonical structs/classes for Ettle and EP.
2) Implement an in-memory store/repository (hash maps keyed by IDs are acceptable).
3) Implement operations API with validation:
    - prevent cycles on `set_parent` (or validate lazily in `validate_tree()`).
    - enforce one-parent rule.
    - enforce one-to-one child mapping by EP.
    - enforce ordinal immutability and uniqueness.
4) Implement traversal computation:
    - RT from leaf by following parent pointers, then reverse.
    - EPT by matching parent EP child pointers along RT.
5) Implement deterministic rendering:
    - stable ordering: EPs in ordinal order.
    - stable formatting to support copy/paste into external AI implementation.
6) Implement deterministic rendering functions (library API):
    - `render_ettle(ettle_id) -> string`
    - `render_leaf_bundle(leaf_ettle_id, leaf_ep_id?) -> string`

## Gherkin scenarios (normative acceptance tests)

### Feature: Create, read, update, delete Ettles

Scenario: Create Ettle fails on empty title  
Given a new in-memory store  
When I create an Ettle titled ""  
Then the operation fails with ERR_INVALID_TITLE  
And no Ettle is created

Scenario: Create Ettle fails on whitespace-only title  
Given a new in-memory store  
When I create an Ettle titled "   "  
Then the operation fails with ERR_INVALID_TITLE  
And no Ettle is created

Scenario: Read Ettle fails for unknown id  
Given a new in-memory store  
When I read_ettle for id "missing"  
Then the operation fails with ERR_ETTLE_NOT_FOUND

Scenario: Update Ettle fails for unknown id  
Given a new in-memory store  
When I update_ettle for id "missing" with title "X"  
Then the operation fails with ERR_ETTLE_NOT_FOUND

Scenario: Update Ettle fails for tombstoned Ettle  
Given a new in-memory store  
And an Ettle titled "A" exists  
And I delete Ettle "A"  
When I update Ettle "A" title to "A2"  
Then the operation fails with ERR_ETTLE_DELETED

Scenario: Delete Ettle fails for unknown id  
Given a new in-memory store  
When I delete_ettle for id "missing"  
Then the operation fails with ERR_ETTLE_NOT_FOUND

Scenario: Delete Ettle fails when Ettle is referenced by an EP child mapping  
Given a new in-memory store  
And an Ettle "A" exists  
And an Ettle "B" exists  
And I set B's parent to A  
And I create EP1 in A and link it to B  
When I attempt to delete Ettle "B"  
Then the operation fails with ERR_DELETE_REFERENCED_CHILD  
And validate_tree succeeds


Scenario: Create Ettle creates EP0 and is readable  
Given a new in-memory store  
When I create an Ettle titled "A"  
Then read_ettle for that id succeeds  
And the Ettle has at least one EP  
And EP0 exists (ordinal 0)

Scenario: Update Ettle title changes only title and updated_at  
Given a new in-memory store  
And an Ettle titled "A" exists  
When I update that Ettle title to "A2"  
Then read_ettle shows title "A2"  
And created_at is unchanged  
And updated_at is later than created_at

Scenario: Update Ettle metadata is persisted and does not change structure  
Given a new in-memory store  
And an Ettle titled "A" exists  
When I update metadata to include {{"priority":"high"}}  
Then read_ettle shows metadata {{"priority":"high"}}  
And validate_tree succeeds

Scenario: Delete Ettle tombstones it when it has no children  
Given a new in-memory store  
And an Ettle titled "A" exists  
When I delete Ettle "A"  
Then read_ettle still returns the Ettle  
And the Ettle is marked deleted  
When I render_ettle "A"  
Then output indicates deleted status

Scenario: Delete Ettle fails when Ettle has children  
Given a new in-memory store  
And an Ettle "A" exists with child Ettle "B"  
When I delete Ettle "A"  
Then the operation fails with ERR_DELETE_WITH_CHILDREN  
And validate_tree still succeeds

Scenario: Setting a second parent is rejected  
Given a new in-memory store  
And Ettles "P1", "P2", and "C" exist  
When I set C's parent to P1  
Then setting C's parent to P2 fails with ERR_MULTIPLE_PARENTS

Scenario: Cycles are rejected (direct)  
Given a new in-memory store  
And Ettles "A" and "B" exist  
When I set B's parent to A  
Then setting A's parent to B fails with ERR_CYCLE_DETECTED

Scenario: Cycles are rejected (indirect, depth>1)  
Given a new in-memory store  
And Ettles "A", "B", "C" exist  
When I set B's parent to A  
And I set C's parent to B  
Then setting A's parent to C fails with ERR_CYCLE_DETECTED

---

### Feature: Create, read, update, delete EPs

Scenario: Create EP fails for unknown Ettle  
Given a new in-memory store  
When I create_ep in ettle_id "missing" with ordinal 1  
Then the operation fails with ERR_ETTLE_NOT_FOUND

Scenario: Create EP fails for tombstoned Ettle  
Given a new in-memory store  
And an Ettle "A" exists  
And I delete Ettle "A"  
When I create_ep in "A" with ordinal 1  
Then the operation fails with ERR_ETTLE_DELETED

Scenario: Create EP fails for negative ordinal  
Given a new in-memory store  
And an Ettle "A" exists  
When I create_ep in "A" with ordinal -1  
Then the operation fails with ERR_INVALID_EP_ORDINAL

Scenario: Create EP fails when ordinal 0 is re-used for EP0  
Given a new in-memory store  
And an Ettle "A" exists  
When I create_ep in "A" with ordinal 0  
Then the operation fails with ERR_DUPLICATE_EP_ORDINAL

Scenario: Read EP fails for unknown id  
Given a new in-memory store  
When I read_ep for id "missing"  
Then the operation fails with ERR_EP_NOT_FOUND

Scenario: Update EP fails for unknown id  
Given a new in-memory store  
When I update_ep for id "missing" with WHY "x"  
Then the operation fails with ERR_EP_NOT_FOUND

Scenario: Update EP fails for tombstoned EP  
Given a new in-memory store  
And an Ettle "A" exists  
And EP1 exists in "A"  
And I delete EP1  
When I update EP1 WHY to "new why"  
Then the operation fails with ERR_EP_DELETED

Scenario: Delete EP fails for unknown id  
Given a new in-memory store  
When I delete_ep for id "missing"  
Then the operation fails with ERR_EP_NOT_FOUND

Scenario: Create EP enforces ordinal uniqueness  
Given a new in-memory store  
And an Ettle "A" exists  
When I create EP1 in "A" with ordinal 1  
Then creating another EP in "A" with ordinal 1 fails with ERR_DUPLICATE_EP_ORDINAL

Scenario: Update EP WHY/WHAT/HOW is reflected in inspection output  
Given a new in-memory store  
And an Ettle "A" exists  
And EP1 exists in "A"  
When I update EP1 WHY to "new why"  
And I update EP1 WHAT to "new what"  
And I update EP1 HOW to include a new Gherkin scenario  
Then inspect show "A" includes "new why"  
And includes "new what"  
And includes the new Gherkin scenario in HOW

Scenario: Delete EP tombstones it when not mapped to a child  
Given a new in-memory store  
And an Ettle "A" exists  
And EP1 exists in "A" with no child mapping  
When I delete EP1  
Then read_ep EP1 returns deleted=true  
And EP1 does not appear in compute_ept for any leaf

Scenario: Delete EP fails when EP maps to an existing child  
Given a new in-memory store  
And an Ettle "A" exists  
And an Ettle "B" exists  
And EP1 in "A" maps to child "B"  
When I delete EP1  
Then the operation fails with ERR_DELETE_REFERENCED_EP  
And validate_tree succeeds

---

### Feature: Refinement mapping rule (one-to-one EP mapping)

Scenario: link_child fails when child Ettle does not exist  
Given a new in-memory store  
And an Ettle "A" exists  
And EP1 exists in "A"  
When I link EP1 to child Ettle "missing"  
Then the operation fails with ERR_CHILD_NOT_FOUND

Scenario: link_child fails when parent EP is tombstoned  
Given a new in-memory store  
And an Ettle "A" exists  
And EP1 exists in "A"  
And I delete EP1  
And an Ettle "B" exists  
When I link EP1 to child Ettle B  
Then the operation fails with ERR_EP_DELETED

Scenario: link_child fails when it would create a cycle  
Given a new in-memory store  
And Ettles "A" and "B" exist  
And I set B's parent to A  
And EP1 exists in "B"  
When I link EP1 in "B" to child Ettle "A"  
Then the operation fails with ERR_CYCLE_DETECTED

Scenario: link_child fails when it would create multiple parents for a child  
Given a new in-memory store  
And Ettles "P1", "P2", and "C" exist  
And I set C's parent to P1  
And EP1 exists in P2  
When I link EP1 in P2 to child Ettle C  
Then the operation fails with ERR_MULTIPLE_PARENTS

Scenario: unlink_child fails for unknown EP id  
Given a new in-memory store  
When I unlink_child for EP id "missing"  
Then the operation fails with ERR_EP_NOT_FOUND

Scenario: set_parent fails when parent does not exist  
Given a new in-memory store  
And an Ettle "C" exists  
When I set C's parent to "missing"  
Then the operation fails with ERR_PARENT_NOT_FOUND

Scenario: Parent with one child uses one explicit EP mapping  
Given a new in-memory store  
And an Ettle "A" exists  
And an Ettle "B" exists  
When I set B's parent to A  
And I create EP1 in A  
And I link EP1 to child Ettle B  
Then validate_tree succeeds  
And list_children(A) returns [B]

Scenario: Parent with two children must have two explicit EP mappings  
Given a new in-memory store  
And an Ettle "A" exists  
And Ettles "B" and "C" exist  
When I set B's parent to A  
And I set C's parent to A  
And I create EP1 in A and link it to B  
And I create EP2 in A and link it to C  
Then validate_tree succeeds

Scenario: Parent with two children but missing one mapping EP fails validation  
Given a new in-memory store  
And an Ettle "A" exists  
And Ettles "B" and "C" exist  
When I set B's parent to A  
And I set C's parent to A  
And I create EP1 in A and link it to B  
Then validate_tree fails with ERR_CHILD_WITHOUT_EP_MAPPING

Scenario: Duplicate mapping to same child is detected  
Given a new in-memory store  
And an Ettle "A" exists  
And an Ettle "B" exists  
When I set B's parent to A  
And I create EP1 in A linked to B  
And I create EP2 in A linked to B  
Then validate_tree fails with ERR_CHILD_REFERENCED_BY_MULTIPLE_EPS

---

### Feature: RT computation

Scenario: RT fails when leaf Ettle is tombstoned  
Given a new in-memory store  
And an Ettle "Leaf" exists  
And I delete Ettle "Leaf"  
When I compute RT for Leaf  
Then the operation fails with ERR_DELETED_NODE_IN_TRAVERSAL

Scenario: RT fails when a cycle exists even if parent pointers are present  
Given a new in-memory store  
And Ettles "A" and "B" exist  
And I set B's parent to A  
And I set A's parent to B  
When I compute RT for A  
Then the operation fails with ERR_CYCLE_DETECTED

Scenario: RT fails for unknown leaf id  
Given a new in-memory store  
When I compute RT for ettle_id "missing"  
Then the operation fails with ERR_ETTLE_NOT_FOUND

Scenario: RT is root-to-leaf order  
Given a new in-memory store  
And Ettles "Root", "Mid", "Leaf" exist  
And Mid's parent is Root  
And Leaf's parent is Mid  
When I compute RT for Leaf  
Then the RT is [Root, Mid, Leaf]

Scenario: RT fails when a parent is missing  
Given a new in-memory store  
And an Ettle "Leaf" exists  
And Leaf's parent_id points to "missing"  
When I compute RT for Leaf  
Then the operation fails with ERR_RT_PARENT_CHAIN_BROKEN

---

### Feature: EPT computation (deterministic)

Scenario: EPT fails when leaf Ettle is tombstoned  
Given a new in-memory store  
And an Ettle "Leaf" exists with only EP0  
And I delete Ettle "Leaf"  
When I compute EPT for Leaf  
Then the operation fails with ERR_DELETED_NODE_IN_TRAVERSAL

Scenario: EPT fails when a parent EP is tombstoned along the traversal  
Given a new in-memory store  
And an Ettle "Root" exists  
And an Ettle "Leaf" exists  
And Leaf's parent is Root  
And Root has EP1 linked to Leaf  
And I delete EP1  
When I compute EPT for Leaf  
Then the operation fails with ERR_EPT_MISSING_MAPPING

Scenario: EPT fails when the leaf has multiple EPs and no leaf_ep_ordinal is provided  
Given a new in-memory store  
And an Ettle "Leaf" exists with EP0 and EP1  
When I compute EPT for Leaf without selecting leaf_ep_ordinal  
Then the operation fails with ERR_EPT_AMBIGUOUS_LEAF_EP  
And the error message includes guidance to pass leaf_ep_ordinal

Scenario: EPT fails when the provided leaf_ep_ordinal does not exist  
Given a new in-memory store  
And an Ettle "Leaf" exists with only EP0  
When I compute EPT for Leaf selecting leaf_ep_ordinal 1  
Then the operation fails with ERR_EP_NOT_FOUND

Scenario: EPT fails on orphaned node (child has parent pointer but parent has no EP mapping)  
Given a new in-memory store  
And an Ettle "Root" exists  
And an Ettle "Leaf" exists  
And Leaf's parent is Root  
And Root has no EP linked to Leaf  
When I compute EPT for Leaf  
Then the operation fails with ERR_EPT_MISSING_MAPPING  
And validate_tree fails with ERR_CHILD_WITHOUT_EP_MAPPING

Scenario: EPT fails when a cycle exists in the structure  
Given a new in-memory store  
And Ettles "A" and "B" exist  
And I set B's parent to A  
And I set A's parent to B  
When I compute EPT for A  
Then the operation fails with ERR_CYCLE_DETECTED

Scenario: EPT fails when leaf EP exists but is tombstoned  
Given a new in-memory store  
And an Ettle "Leaf" exists with only EP0  
And I delete EP0  
When I compute EPT for Leaf  
Then the operation fails with ERR_EPT_AMBIGUOUS_LEAF_EP  
And the error message includes guidance to create a non-deleted EP or restore EP0

Scenario: EPT includes parent EP links plus selected leaf EP  
Given a new in-memory store  
And an Ettle "Root" exists with EP0  
And an Ettle "Leaf" exists with EP0  
And Leaf's parent is Root  
And Root has EP1 linked to Leaf  
When I compute EPT for Leaf  
Then the EPT is [Root.EP1, Leaf.EP0]

Scenario: EPT fails when mapping is missing  
Given a new in-memory store  
And an Ettle "Root" exists  
And an Ettle "Leaf" exists  
And Leaf's parent is Root  
And Root has no EP linked to Leaf  
When I compute EPT for Leaf  
Then the operation fails with ERR_EPT_MISSING_MAPPING

Scenario: EPT fails when mapping is duplicated  
Given a new in-memory store  
And an Ettle "Root" exists  
And an Ettle "Leaf" exists  
And Leaf's parent is Root  
And Root has EP1 linked to Leaf  
And Root has EP2 linked to Leaf  
When I compute EPT for Leaf  
Then the operation fails with ERR_EPT_DUPLICATE_MAPPING

Scenario: Leaf EP selection is automatic for single-EP leaf  
Given a new in-memory store  
And an Ettle "Leaf" exists with only EP0  
When I compute EPT for Leaf  
Then the EPT ends with Leaf.EP0

Scenario: Leaf EP selection is explicit when leaf has multiple EPs  
Given a new in-memory store  
And an Ettle "Leaf" exists with EP0 and EP1  
When I compute EPT for Leaf without selecting a leaf EP  
Then the operation fails with ERR_EPT_AMBIGUOUS_LEAF_EP  
When I compute EPT for Leaf selecting EP1  
Then the EPT ends with Leaf.EP1

---

### Feature: Deterministic export

Scenario: render_ettle fails for unknown ettle id  
Given a new in-memory store  
When I call render_ettle for "missing"  
Then the operation fails with ERR_ETTLE_NOT_FOUND

Scenario: render_ettle fails for tombstoned ettle id  
Given a new in-memory store  
And an Ettle "A" exists  
And I delete Ettle "A"  
When I call render_ettle for "A"  
Then the operation fails with ERR_DELETED_NODE_IN_TRAVERSAL

Scenario: render_leaf_bundle fails when EPT is ambiguous  
Given a new in-memory store  
And an Ettle "Leaf" exists with EP0 and EP1  
When I call render_leaf_bundle for "Leaf" without leaf_ep_ordinal  
Then the operation fails with ERR_EPT_AMBIGUOUS_LEAF_EP

Scenario: render_leaf_bundle fails when EPT contains a missing mapping  
Given a new in-memory store  
And an Ettle "Root" exists  
And an Ettle "Leaf" exists  
And Leaf's parent is Root  
And Root has no EP linked to Leaf  
When I call render_leaf_bundle for "Leaf"  
Then the operation fails with ERR_EPT_MISSING_MAPPING

Scenario: render_leaf_bundle output includes full HOW Gherkin blocks without truncation  
Given a new in-memory store  
And an Ettle "Root" exists  
And an Ettle "Leaf" exists  
And Leaf's parent is Root  
And Root has EP1 linked to Leaf  
And Leaf EP0 HOW contains a Scenario Outline with 10 examples  
When I call render_leaf_bundle for "Leaf"  
Then the rendered output contains the entire Scenario Outline and all 10 examples

Scenario: Exporting the same leaf twice yields identical bytes  
Given a new in-memory store  
And an Ettle "X" exists with EP0  
When I render_leaf_bundle for X twice  
Then the two rendered outputs are identical

Scenario: Export fails if EPT is invalid  
Given a new in-memory store  
And a leaf "Leaf" exists whose parent mapping is missing  
When I call render_leaf_bundle for Leaf  
Then the operation fails with ERR_EPT_MISSING_MAPPING

---

### Feature: Unit tests construct representative trees (explicit Phase 0.5 tests)

Scenario: Unit tests verify deterministic EPT output  
Given a test tree Root → Mid → Leaf  
And Root EP1 links to Mid  
And Mid EP2 links to Leaf  
When compute_ept(Leaf) is executed twice  
Then both results are identical  
And render_leaf_bundle(Leaf) is identical byte-for-byte for both runs

Scenario: Unit tests detect missing parent→child EP mapping  
Given a test tree Root → Leaf  
And Root has no EP linking to Leaf  
When compute_ept(Leaf) is executed  
Then the error is ERR_EPT_MISSING_MAPPING

Scenario: Unit tests detect duplicate mapping  
Given a test tree Root → Leaf  
And Root EP1 links to Leaf  
And Root EP2 links to Leaf  
When compute_ept(Leaf) is executed  
Then the error is ERR_EPT_DUPLICATE_MAPPING

Scenario: Unit tests handle single-EP leaf  
Given a leaf with only EP0  
When compute_ept(Leaf) is executed  
Then EP0 is selected automatically

---

## Notes for the external AI code generator (non-normative but operational)

- Generate Rust code (library + tests) to satisfy the acceptance scenarios.
- Prioritise clarity and determinism over performance.
- Use stable ordering and explicit typed errors.
- Keep persistence out of scope (no SQLite yet).
- Ensure the inspector/export binary compiles and can render Ettles reliably.
- Document how to run tests and the inspector.
- Triad generation is mandatory: for each scenario/delta, generate (a) tests, (b) minimal code to pass via strict TDD, and (c) documentation updates. No scenario may exist without a test. No behavioural code may exist without a driving scenario test. All new/changed code must be documented.

---

**End of Phase 0.5 entry Ettle (deliverables integrated, structurally rewritten).**
