# Ettle Authoring Prompt --- Behavioural Modification Mode (Phased, Operation-Aware)

**Version:** 2.0\
**Generated:** 2026-02-26T18:13:45.016247Z

This replaces v1.1 and incorporates phased authoring, scenario-type
completeness, and strict separation of revision vs refinement semantics.

A Behavioural Modification Ettle produces a **new version of the same
Ettle ID** and changes externally observable behaviour.

Refinement (adding children) is NOT performed in this mode.

------------------------------------------------------------------------

## STEP 0 --- DECLARE OPERATION (MANDATORY)

Operation: REVISE_NODE\
Revision Type (mandatory):

-   Clarification-only
-   Coverage expansion
-   Partition adjustment
-   Semantic modification

If Revision Type is "Semantic modification", full delta and
compatibility analysis is required.

------------------------------------------------------------------------

# PHASE 1 --- Baseline Behaviour Extraction

You MUST extract from the prior version:

1.  Existing scenarios (leaf) OR contracts/models (non-leaf).
2.  Expected outcomes (observable behaviour).
3.  Invariants.
4.  Constraint enforcement semantics.
5.  Public surface classification.

Output: **Baseline Behaviour Matrix**.

Gate: - If baseline behaviour cannot be precisely described, STOP.

------------------------------------------------------------------------

# PHASE 2 --- Behavioural Delta Definition

For each impacted behaviour:

-   Previous behaviour (assertable statement).
-   New behaviour (assertable statement).
-   Rationale for change.
-   Breaking vs non-breaking classification.
-   Migration implications (if applicable).

Output: **Behavioural Delta Table**.

All changes MUST be explicit. No implicit drift allowed.

------------------------------------------------------------------------

# PHASE 3 --- Updated WHY / WHAT

## WHY

-   Reason for semantic change.
-   Improvement justification.
-   Risk/impact assessment.

## WHAT

Must:

-   Clearly reflect the new behavioural contract.
-   Explicitly mark removed semantics.
-   Explicitly list modified invariants.
-   Explicitly list new or changed error semantics.
-   Maintain assertability of all clauses.

Gate: - If any WHAT clause is ambiguous or non-assertable, STOP.

------------------------------------------------------------------------

# PHASE 4 --- Updated HOW (Representation-Aware)

If the Ettle is **leaf**:

HOW MUST: - Include updated Gherkin scenarios/outlines. - Explicitly
mark changed scenarios. - Include before/after expected outcome notes. -
Revalidate full scenario-type completeness (see below).

If the Ettle is **non-leaf**:

HOW MUST: - Update contracts/models/tables. - Clearly mark changed
clauses. - Indicate how leaf-driving behaviour will change downstream.

------------------------------------------------------------------------

## Leaf Scenario-Type Completeness (Revalidation Required)

All applicable categories MUST be revalidated after modification:

1.  Happy path
2.  Negative cases
3.  Explicit error paths
4.  Boundary conditions
5.  Invariants
6.  Idempotency / repeatability (if relevant)
7.  Determinism / ordering (if relevant)
8.  State transitions (if stateful)
9.  Concurrency / re-entrancy (if relevant)
10. Security / authorisation (if relevant)
11. Observability obligations (if specified)
12. Compatibility/migration behaviour (if relevant)
13. Resource/performance limits (if specified)

If a category becomes newly relevant due to the modification, it MUST be
added.

------------------------------------------------------------------------

# PHASE 5 --- Constraint Delta Review

For each constraint:

-   Previous enforcement semantics.
-   New enforcement semantics.
-   New obligations (if any).
-   Risk of regression.

Output: **Constraint Delta Table**.

Constraints must not remain prose-only.

------------------------------------------------------------------------

# PHASE 6 --- Reachability & Drift Validation

Produce:

A. Reachability Matrix (WHAT → HOW references)\
B. Drift Confirmation Statement\
C. Underspecification Report

Gate: - If removed behaviours are not explicitly documented, STOP. - If
new behaviours lack scenario drivers (leaf), STOP.

------------------------------------------------------------------------

# MANDATORY COMPLETENESS CRITERIA

Invalid unless:

1.  Revision type declared.
2.  Baseline Behaviour Matrix present.
3.  Behavioural Delta Table present.
4.  Updated WHAT fully assertable.
5.  Leaf: scenario-type completeness revalidated.
6.  Constraint Delta Table present.
7.  Reachability Matrix present.
8.  Explicit declaration of semantic change scope.

------------------------------------------------------------------------

End of Behavioural Modification Mode v2.0
