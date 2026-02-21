# Ettle (Refactoring Markdown) — Phase 0.5 Kernel Refactor: Functional-Boundary Transitions + Anchored Deletion Gate

**Product:** EttleX  
**Purpose:** Refactor the existing Phase 0.5 in-memory kernel implementation to a functional-boundary transition style (state-in/state-out) and introduce an anchored deletion gate enabling **hard delete for non-anchored** EPs while preserving **tombstone semantics for anchored** EPs.  
**Tier:** CORE (Refactoring leaf; Phase 0.5-compatible)  
**Status:** Draft (refactoring spec; generator-ready)  
**Created:** 2026-02-21  
**Ettle-ID:** ettle/refactor-phase0-5-functional-boundary-and-anchored-delete (bootstrap)  
**EPs:** EP0 only (single-leaf implementable)

> This Ettle is a **refactoring** handoff. It assumes the Phase 0.5 kernel already satisfies the existing Phase 0.5 Entry Ettle and related scenarios (CRUD, RT/EPT, deterministic rendering, mutation rules, error taxonomy, etc.).  
> This document contains **only** the additional normative requirements and scenarios needed for the refactor. It deliberately does **not** restate scenarios that are already implemented.

---

## EP0 — Functional-Boundary Apply + Anchored EP Deletion Policy (Hard Delete vs Tombstone)

### WHY (purpose / rationale)

The Phase 0.5 kernel is already correct for baseline CRUD, tree invariants, RT/EPT computation, and deterministic rendering. However, the current mutation model (public operations mutating shared state in place) increases the risk of partial-update bugs and makes atomic multi-structure changes harder to reason about during high-churn refactoring.

Additionally, Phase 0.5 currently treats deletions as tombstone-only, which is acceptable for anchored history but undesirable for pre-realisation churn because it can accumulate low-value prototype artefacts. We therefore introduce a Phase 0.5-compatible **anchored deletion gate**: tombstone only when an EP is considered “anchored”, otherwise hard delete.

This refactor:
- improves atomicity (all-or-nothing updates),
- preserves determinism and validation behaviour,
- enables churn-friendly deletion without undermining future anchored history semantics.

### WHAT (normative conditions / features)

#### 1. Scope boundary (refactor-only; no new Phase 1 surfaces)

This work MUST:
- refactor only the Phase 0.5 kernel implementation style;
- preserve all externally observable semantics already mandated by the existing Phase 0.5 specs and tests.

This work MUST NOT:
- add CLI/tool surfaces or binaries,
- add persistence, CAS, snapshots, constraints, policies/profiles,
- implement split/merge/reparent/undo/preview bundles,
- change RT/EPT selection policy beyond what Phase 0.5 already mandates.

All existing tests/scenarios remain valid and MUST continue to pass.

#### 2. Functional-boundary transition API

#### 2.1 Command inventory (explicit; Phase 0.5 operational set)

`apply(...)` MUST support **the complete existing Phase 0.5 operational set** already implemented in your kernel.
This refactor MUST NOT reduce functionality by leaving some existing operations “out of apply”.

At minimum, the following commands (or equivalent) MUST be expressible via `Command` and routed through `apply(...)`:

Ettle operations
- `EttleCreate` (with metadata; EP0 auto-created; optionally EP0 content populated)
- `EttleRead`
- `EttleUpdate` (including metadata update and EP-scoped updates via ordinal where supported)
- `EttleDelete` (tombstone-only; Phase 0.5 rule unchanged)

EP operations (within an Ettle)
- `EpCreate` (create EPn with ordinal + content)
- `EpRead`
- `EpUpdate`
- `EpDelete` (policy-gated: hard delete when not anchored; tombstone when anchored)

Refinement wiring operations (Phase 0.5 set only; no split/merge/reparent)
- `RefineLinkChild` (set parent EP child mapping to child Ettle; set child.parent_id accordingly)
- `RefineUnlinkChild` (remove mapping and parent pointer as per your existing invariants)
- `TreeValidate` (optional command wrapper; or `apply` may call `validate_tree` automatically for structural commands)

If your current code has additional Phase 0.5 commands beyond the above, they MUST also be included in `Command` and routed via `apply`.



Introduce a functional-boundary transition layer that becomes the canonical way to perform any behavioural mutation.

Required API shape (normative):

- A `Command` (or equivalent) family representing the set of supported operations already implemented in Phase 0.5 (ettle CRUD, ep CRUD, refinement wiring).
- A single entry function:

Option A:
- `apply(state: &MemState, cmd: Command, policy: &dyn AnchorPolicy) -> Result<MemState, Error>`

Option B (recommended for minimal churn and fewer clones):
- `apply(state: MemState, cmd: Command, policy: &dyn AnchorPolicy) -> Result<MemState, Error>`

Normative semantics:
- `apply(...)` MUST be **atomic**: either it returns a fully valid new state, or it returns an error and the caller’s prior state remains usable and valid.
- `apply(...)` MUST NOT panic for any invalid input; all invalid inputs must be surfaced as typed errors.
- `apply(...)` MUST run (or ensure) deterministic validation equivalent to calling `validate_tree()` after the mutation, for all commands that can affect structure/traversal.

Important clarification:
- “Returns a new state” is a boundary contract. It does **not** mean all intermediate states must be retained. The old state is expected to be dropped/overwritten by the caller in normal use.

#### 3. Anchored deletion gate (Phase 0.5-compatible)

Introduce an injectable policy interface:

- `trait AnchorPolicy { fn is_anchored_ep(&self, ep_id) -> bool; fn is_anchored_ettle(&self, ettle_id) -> bool; }`

Phase 0.5 MUST include at least two test policies:
- `NeverAnchoredPolicy` (always returns false)
- `SelectedAnchoredPolicy` (returns true for a declared set of ids)

Deletion semantics (EPs):

- If `is_anchored_ep(ep_id) == true`:
  - hard delete MUST be forbidden
  - delete operation MUST be tombstone (set `deleted=true`) and MUST preserve the EP record
- If `is_anchored_ep(ep_id) == false`:
  - delete operation MUST be a **hard delete** by default:
    - EP record removed from EP store
    - EP id removed from owning Ettle’s `ep_ids`
  - (Optional) tombstone may still exist as an internal helper, but the externally visible behaviour MUST be hard delete

Deletion semantics (Ettles):

- Ettle hard delete remains forbidden in Phase 0.5 (unchanged).
- Ettle delete remains tombstone-only (unchanged).
- `is_anchored_ettle` is present for forward compatibility but is not used to enable Ettle hard delete in Phase 0.5.

#### 4. Error taxonomy additions (only where genuinely new)

Add typed errors only for new behaviours introduced by this refactor.

Minimum additions:

- `ERR_APPLY_ATOMICITY_BREACH` (reserved; should never occur if implemented correctly; used only for internal assertions converted into errors)
- `ERR_HARD_DELETE_FORBIDDEN_ANCHORED_EP` (attempted hard delete of anchored EP)
- `ERR_DELETE_REFERENCES_MISSING_EP_IN_OWNING_ETTLE` (hard delete cannot remove because membership list is inconsistent; this should normally be caught by validation, but deletion must surface a deterministic error if encountered)

(Do not duplicate existing error kinds already defined in Phase 0.5 for “not found”, “deleted”, “referenced”, “strands child”, etc. Reuse existing kinds.)

#### 5. Deterministic rendering and traversal remain unchanged

- `render_ettle` and `render_leaf_bundle` remain pure library functions.
- RT/EPT behaviour remains unchanged.
- Tombstoned EPs remain excluded from traversal and export (already implemented); hard-deleted EPs are simply absent.

### HOW (method / process, including Gherkin scenarios)

#### Implementation approach (narrative, non-normative but guiding)

1) Introduce `Command` enums/structs for the already-supported Phase 0.5 operations (do not add new operations beyond those already implemented).
2) Implement `apply(...)` as the only mutation entry point used by new tests, then progressively migrate existing tests to go through `apply(...)` (or at minimum, ensure `apply` is exercised and correct).
3) Wrap existing `&mut` mutation code inside `apply` initially (Option B makes this very straightforward), then tighten atomicity and validation guarantees.
4) Introduce `AnchorPolicy` and refactor EP deletion to choose tombstone vs hard delete using the policy.
5) Ensure that all existing tests still pass, then add the new scenarios below.

---

## Gherkin scenarios (normative acceptance tests for the refactor only)

> These scenarios are intentionally limited to the **new** refactor contract: functional-boundary apply and anchored deletion gating.  
> They should be implemented in addition to the existing Phase 0.5 test suite, without deleting or weakening existing tests.

### Feature: Functional-boundary apply is atomic and deterministic

Scenario: apply returns a new valid state on success  
Given an existing valid MemState S0  
And S0 contains an Ettle "A" with EP0  
When I call apply(S0, Command::CreateEp{ettle:"A", ordinal:1, ...}, NeverAnchoredPolicy) producing S1  
Then apply returns success  
And validate_tree(S1) succeeds  
And S1 differs from S0 by the presence of EP1 in "A"  
And S0 remains readable and validate_tree(S0) still succeeds

Scenario: apply fails without partially mutating state  
Given an existing valid MemState S0  
And S0 contains an Ettle "A" with EP0  
And S0 already contains EP1 in "A" with ordinal 1  
When I call apply(S0, Command::CreateEp{ettle:"A", ordinal:1, ...}, NeverAnchoredPolicy)  
Then the operation fails with ERR_DUPLICATE_EP_ORDINAL  
And S0 is still readable and validate_tree(S0) succeeds  
And reading Ettle "A" from S0 shows it still has exactly one EP with ordinal 1

Scenario: apply surfaces typed errors and never panics  
Given an existing valid MemState S0  
When I call apply(S0, Command::ReadEttle{ettle_id:"missing"}, NeverAnchoredPolicy)  
Then the operation fails with ERR_ETTLE_NOT_FOUND  
And the process does not panic



### Feature: Functional-boundary apply supports the full Phase 0.5 command set

Scenario: apply supports Ettle update without in-place mutation leakage  
Given an existing valid MemState S0  
And S0 contains an Ettle "A" with metadata {"owner":"platform"}  
When I call apply(S0, Command::EttleUpdate{ettle:"A", metadata:{"owner":"core"}}, NeverAnchoredPolicy) producing S1  
Then apply returns success  
And reading Ettle "A" from S1 shows metadata {"owner":"core"}  
And validate_tree(S1) succeeds  
And reading Ettle "A" from S0 still shows metadata {"owner":"platform"}  
And validate_tree(S0) succeeds

Error path: apply fails on invalid Ettle update input without partial mutation  
Given an existing valid MemState S0  
And S0 contains an Ettle "A"  
When I call apply(S0, Command::EttleUpdate{ettle:"A", metadata:INVALID}, NeverAnchoredPolicy)  
Then the operation fails with ERR_INVALID_METADATA (or the existing invalid-metadata error)  
And S0 remains readable and validate_tree(S0) succeeds

Scenario: apply supports EP update (content update)  
Given an existing valid MemState S0  
And S0 contains an Ettle "A" with EP0  
When I call apply(S0, Command::EpUpdate{ep:EP0, what:"Updated condition"}, NeverAnchoredPolicy) producing S1  
Then apply returns success  
And reading EP0 from S1 shows what == "Updated condition"  
And validate_tree(S1) succeeds  
And reading EP0 from S0 does not show what == "Updated condition"

Error path: apply fails on EP update when EP is missing  
Given an existing valid MemState S0  
When I call apply(S0, Command::EpUpdate{ep:"missing", what:"X"}, NeverAnchoredPolicy)  
Then the operation fails with ERR_EP_NOT_FOUND  
And S0 remains valid

Scenario: apply supports refinement wiring (link child)  
Given an existing valid MemState S0  
And S0 contains parent ettle P with EP1 active  
And S0 contains child ettle C with no parent_id set  
When I call apply(S0, Command::RefineLinkChild{parent_ep:EP1, child_ettle:C}, NeverAnchoredPolicy) producing S1  
Then apply returns success  
And EP1.child_ettle_id == C.id in S1  
And C.parent_id == P.id in S1  
And validate_tree(S1) succeeds

Error path: apply fails on refinement wiring when mapping would violate invariants  
Given an existing valid MemState S0  
And EP1 already maps to child C1  
When I call apply(S0, Command::RefineLinkChild{parent_ep:EP1, child_ettle:C2}, NeverAnchoredPolicy)  
Then the operation fails with the existing invariant error for “duplicate/illegal mapping”  
And S0 remains valid

Scenario: apply supports refinement unwiring (unlink child)  
Given an existing valid MemState S0  
And EP1 maps to child ettle C  
And C.parent_id == P.id  
When I call apply(S0, Command::RefineUnlinkChild{parent_ep:EP1, child_ettle:C}, NeverAnchoredPolicy) producing S1  
Then apply returns success  
And EP1.child_ettle_id is empty in S1  
And C.parent_id is empty in S1  
And validate_tree(S1) succeeds

Error path: apply fails on unlink when link is missing  
Given an existing valid MemState S0  
And EP1 has no child_ettle_id  
When I call apply(S0, Command::RefineUnlinkChild{parent_ep:EP1, child_ettle:C}, NeverAnchoredPolicy)  
Then the operation fails with the existing “link not found” error  
And S0 remains valid



---

### Feature: EP deletion chooses tombstone vs hard delete based on anchored policy

Scenario: deleting a non-anchored EP performs hard delete  
Given an existing valid MemState S0  
And S0 contains an Ettle "A" with EP0 and EP1 (EP1 has no child mapping)  
And NeverAnchoredPolicy reports EP1 is not anchored  
When I call apply(S0, Command::DeleteEp{ep_id:EP1}, NeverAnchoredPolicy) producing S1  
Then apply returns success  
And reading EP1 from S1 fails with ERR_EP_NOT_FOUND  
And reading Ettle "A" from S1 shows EP1 is not present in its EP membership listing  
And validate_tree(S1) succeeds

Error path: deleting a non-anchored EP fails if deletion would violate existing Phase 0.5 rules  
Given an existing valid MemState S0  
And S0 contains Ettle "A" and child Ettle "B"  
And EP1 in "A" is the only EP mapping to child "B"  
And NeverAnchoredPolicy reports EP1 is not anchored  
When I call apply(S0, Command::DeleteEp{ep_id:EP1}, NeverAnchoredPolicy)  
Then the operation fails with ERR_DELETE_REFERENCED_EP or ERR_TOMBSTONE_STRANDS_CHILD (whichever is already used by the implementation)  
And validate_tree(S0) still succeeds

Scenario: deleting an anchored EP performs tombstone and forbids hard delete  
Given an existing valid MemState S0  
And S0 contains an Ettle "A" with EP0 and EP1 (EP1 has no child mapping)  
And SelectedAnchoredPolicy reports EP1 is anchored  
When I call apply(S0, Command::DeleteEp{ep_id:EP1}, SelectedAnchoredPolicy) producing S1  
Then apply returns success  
And reading EP1 from S1 succeeds  
And EP1.deleted is true  
And EP1 does not participate in traversal or export (i.e., EPT for any leaf does not include EP1)  
And validate_tree(S1) succeeds

Error path: hard delete is forbidden for anchored EP  
Given an existing valid MemState S0  
And S0 contains an Ettle "A" with EP0 and EP1  
And SelectedAnchoredPolicy reports EP1 is anchored  
When I call apply(S0, Command::HardDeleteEp{ep_id:EP1}, SelectedAnchoredPolicy)  
Then the operation fails with ERR_HARD_DELETE_FORBIDDEN_ANCHORED_EP  
And S0 remains valid

(If you do not expose a separate `HardDeleteEp` command, implement this scenario by:
- attempting to call an internal hard-delete path via a test hook, OR
- asserting that `DeleteEp` never removes anchored EP records from the EP store.)

---

### Feature: apply + deletion maintains membership integrity under refactor boundary

Scenario: hard delete removes EP from owning Ettle membership listing  
Given an existing valid MemState S0  
And S0 contains Ettle "A" with EP0 and EP1  
And NeverAnchoredPolicy reports EP1 is not anchored  
When I apply DeleteEp(EP1) producing S1  
Then Ettle "A" membership listing contains EP0 but not EP1  
And validate_tree(S1) succeeds

Error path: delete fails deterministically if membership is inconsistent (defensive)  
Given a MemState S0 that is otherwise valid except:  
And EP1.ettle_id points to Ettle "A"  
But Ettle "A".ep_ids does not contain EP1  
When I call apply(DeleteEp(EP1), NeverAnchoredPolicy)  
Then the operation fails with ERR_DELETE_REFERENCES_MISSING_EP_IN_OWNING_ETTLE (or an existing membership inconsistency error if already present)  
And the operation does not panic

---

## Notes for the external AI coding agent (non-normative but binding as delivery discipline)

- This is a refactor. Preserve all existing semantics and tests; add only what is required for the new scenarios.
- Use strict TDD: implement each new scenario as a test first, then change code to pass, then update docs.
- Do not introduce new Phase 1 surfaces (CLI/binaries); keep everything as library APIs and unit tests.
- Keep deterministic ordering and explicit typed errors. Do not rely on string matching for error checks.

---

**End of refactoring Ettle.**
