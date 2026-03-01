# Ettle Authoring Prompt --- Refactor-Only Mode (Phased, Operation-Aware)

**Version:** 2.0\
**Generated:** 2026-02-26T18:13:45.016247Z

This replaces v1.1 and enforces phased validation and scenario
preservation.

A Refactor-Only Ettle produces a **new version of the same Ettle ID**
without changing externally observable behaviour.

Refinement (adding children) is NOT performed in this mode.

------------------------------------------------------------------------

## STEP 0 --- DECLARE OPERATION (MANDATORY)

Operation: REVISE_NODE\
Revision Type: Refactor-only

If behavioural change is detected at any phase, STOP and reclassify as
Behavioural Modification.

------------------------------------------------------------------------

# PHASE 1 --- Behavioural Baseline Capture

Extract from prior version:

1.  All scenarios (leaf) or behavioural contracts/models (non-leaf).
2.  Observable outcomes.
3.  Invariants.
4.  Constraint enforcement semantics.
5.  Public surface classification.

Output: **Behavioural Baseline Summary**.

Gate: - If baseline cannot be precisely restated, STOP.

------------------------------------------------------------------------

# PHASE 2 --- Refactor Scope Definition

Define:

-   Structural changes (module moves, abstraction extraction, naming
    changes).
-   Architectural realignment.
-   Determinism/performance improvements (only if semantics preserved).
-   Explicit declaration: "No behavioural change intended."

Also define: - Out-of-scope behavioural areas. - Compatibility
guarantees.

------------------------------------------------------------------------

# PHASE 3 --- Behaviour Preservation Validation

For each scenario/contract:

-   Confirm expected outcomes unchanged.
-   Confirm invariants unchanged.
-   Confirm error semantics unchanged.
-   Confirm public surface unchanged.

If leaf: - Confirm Gherkin scenarios unchanged except editorial
clarifications. - Scenario-type completeness remains satisfied.

Output: **Behaviour Preservation Matrix**.

Gate: - If any semantic change detected, STOP and reclassify.

------------------------------------------------------------------------

# PHASE 4 --- Constraint Stability Review

For each constraint:

-   Confirm enforcement unchanged.
-   Confirm no obligations added.
-   Confirm no obligations weakened.

Output: Constraint Stability Statement.

Gate: - If enforcement semantics change, STOP and reclassify.

------------------------------------------------------------------------

# PHASE 5 --- Reachability Confirmation

Produce:

-   Confirmation that WHAT clauses remain reachable.
-   Confirmation that scenario drivers (leaf) unchanged.
-   Underspecification Report (even if empty).

------------------------------------------------------------------------

# MANDATORY COMPLETENESS CRITERIA

Invalid unless:

1.  Operation + Revision Type declared.
2.  Behavioural Baseline Summary present.
3.  Refactor Scope Definition present.
4.  Behaviour Preservation Matrix present.
5.  Constraint Stability Statement present.
6.  Explicit declaration of semantic preservation.
7.  Leaf: scenario-type completeness still satisfied.

------------------------------------------------------------------------

End of Refactor-Only Mode v2.0
