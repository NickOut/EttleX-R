# Ettle Authoring Prompt --- Operation-Aware, Phased Protocol

**Version:** 2.1\
**Generated:** 2026-02-26T18:05:23.809343Z

This prompt governs Ettle authoring with explicit separation of: -
**Creation** (CREATE_NODE) - **Refinement** (REFINE_NODE: add children;
parent unchanged) - **Revision** (REVISE_NODE: new version of same node)

It also enforces **phased authoring** and **scenario-type completeness**
for leaf Ettles.

------------------------------------------------------------------------

## STEP 0 --- DECLARE OPERATION (MANDATORY)

You MUST declare exactly one:

-   CREATE_NODE (introduce a new Ettle node)
-   REFINE_NODE (add one or more child Ettles; parent remains unchanged)
-   REVISE_NODE (produce a new version of an existing Ettle; same ID)

If unclear, STOP and request clarification.

------------------------------------------------------------------------

## GLOBAL AUTHORITY RULES (NON-NEGOTIABLE)

1.  You MUST NOT blur scope boundaries between EPs.
2.  You MUST surface ambiguities explicitly (do not guess).
3.  Constraints MUST be translated into explicit obligations (not
    prose-only references).
4.  **Leaf Ettles** MUST be test-driving: HOW MUST include Gherkin.
5.  **Non-leaf Ettles** MAY use other representations in HOW (see
    below).

------------------------------------------------------------------------

# Shared Phases (apply to all operations unless stated otherwise)

These phases define the *process* you must follow.

## PHASE 1 --- Semantic Envelope (Scope First)

For each EP (or for the EPs you are creating/revising):

-   In-scope responsibilities
-   Explicit exclusions (out-of-scope)
-   External dependencies
-   Ambiguities / terms requiring definition
-   Monotonicity notes:
    -   Must not leak outside parent envelope
    -   Must not create sideways responsibilities

Output: **EP Envelope Summary Table**.

Gate: - If envelopes overlap or are unclear, revise before proceeding.

## PHASE 2 --- Construct WHY / WHAT / HOW (Representation-Aware)

### WHY

-   Problem being solved
-   Non-goals
-   Success criteria (measurable if appropriate)

### WHAT (assertable contract)

-   Behavioural contract statements (assertable)
-   Invariants (explicit)
-   Error semantics (explicit triggers + outcomes)
-   State model (if stateful)
-   Determinism/ordering guarantees (if applicable)
-   Idempotency/repeatability rules (if applicable)

Gate: - If any WHAT statement is not assertable, STOP and revise.

### HOW (depends on abstraction level)

**Non-leaf / abstract Ettles** may use one or more of: - Decision tables
/ rule tables - State machine descriptions (states, events, guards,
transitions) - Contracts (pre/post conditions) - Structured narrative
operational models - Invariant/property sets and verification
obligations

**Leaf Ettles** MUST use: - Gherkin Scenarios and/or Scenario Outlines
(with Examples where useful)

Gate: - Every WHAT clause MUST be reachable from HOW representations. -
If leaf: every behavioural obligation MUST be driven by at least one
Gherkin scenario/outcome.

## PHASE 3 --- Leaf Scenario-Type Completeness (Leaf Ettles Only)

If the Ettle is **leaf**, you MUST construct scenarios covering all
applicable types below. Omit only with explicit justification in an
**Omitted Coverage** note.

Mandatory scenario types / coverage categories:

1.  Happy path (representative successful execution)
2.  Negative cases (invalid inputs, invalid state, forbidden operations)
3.  Explicit error paths (each defined error semantics must have a
    driver scenario)
4.  Boundary conditions (min/max; empty/singleton; large; edge ordering;
    unusual-but-valid)
5.  Invariants (scenarios demonstrating invariants hold; and violations
    are rejected if defined)
6.  Idempotency / repeatability (when relevant)
7.  Determinism / ordering (when relevant; stable outputs/ordering
    guarantees)
8.  State transitions (when stateful; legal/illegal transitions;
    intermediate states)
9.  Concurrency / re-entrancy (when relevant; conflicting operations;
    interleavings)
10. Compatibility/migration behaviour (when relevant;
    versioning/backward compatibility constraints)
11. Security / authorisation / access control (when relevant)
12. Observability obligations (only when specified by WHAT/constraints;
    logging/metrics/audit semantics)
13. Resource limits / performance constraints (only when explicitly
    specified; backpressure, caps)

Scenario construction rules: - Every scenario must have: Preconditions,
Trigger, Expected outcome, and Failure outcome where applicable. -
Prefer Scenario Outlines for matrices (boundary/negative
combinations). - Avoid duplicate scenarios; parameterise instead.

Gate: - If an applicable category is missing without justification, STOP
and revise.

## PHASE 4 --- Constraint Obligation Formalisation

For each referenced constraint: - Constraint ID - Obligations
(MUST/SHALL where relevant) - Enforcement expectations - Failure
semantics - Verification/test strategy (leaf) or verification plan
(non-leaf)

Output: **Constraint Obligation Table**.

Gate: - If a constraint remains prose-only, STOP and revise.

## PHASE 5 --- Coverage & Reachability Review

Output the following:

A. **Reachability Matrix** - Non-leaf: WHAT clause →
model/table/contract reference(s) - Leaf: WHAT clause → Scenario ID(s)

B. **Monotonicity Review Notes** - Confirm child EPs do not expand
parent scope - Confirm exclusions are preserved

C. **Underspecification Report** - Ambiguous terms - Missing error
semantics - Unbounded behaviour - Leaf: missing scenario-type coverage
(if any)

Gate: - If critical underspecification exists, revise before finalising.

## PHASE 6 --- Optional Partition Validation (when EP split is non-trivial)

Generate up to 3 alternative EP partitions and select one based on: -
Coupling minimisation - Testability clarity (leaf viability) -
Constraint alignment - Extensibility

Output: **Partition Justification** (if used).

------------------------------------------------------------------------

# Operation-Specific Requirements

## OPERATION: CREATE_NODE

Goal: introduce a new Ettle node.

Apply PHASES 1--6 to the new node.

Outputs (minimum): - Operation declaration - EP Envelope Summary Table -
Full WHY / WHAT / HOW - Constraint Obligation Table - Reachability
Matrix - Monotonicity Review Notes - Underspecification Report - Leaf
only: Scenario-type coverage notes + omitted coverage justification (if
any) - Partition Justification (if applicable)

## OPERATION: REFINE_NODE (Add Children; Parent Unchanged)

Goal: add one or more child Ettles beneath an existing parent.

Inputs required: - Canonical parent Ettle text (treated as immutable for
this operation) - Refinement objective (what concretisation is needed)

Rules / invariants: 1. Parent content MUST NOT be modified. 2. For each
child: Child scope ⊆ Parent scope (monotonic containment). 3. Child
invariants MUST NOT contradict parent invariants. 4. Child exclusions
MUST respect parent exclusions. 5. Child may introduce additional
detail, not contradictory semantics.

Process: - Apply PHASES 1--6 to each child Ettle you create. -
Additionally produce a **Containment Validation Statement** for each
child: - Explicitly justify why child obligations are within parent
envelope. - List any parent statements not yet covered by children
(allowed, but must be explicit).

Outputs (minimum): - Operation declaration - Parent reference (ID/digest
if available) - New child Ettle(s) (full text) - Containment Validation
Statement(s) - Child review artefacts (reachability, obligations,
underspec)

## OPERATION: REVISE_NODE (New Version; Same ID)

Goal: update an existing Ettle (new version). This is not refinement.

Inputs required: - Existing Ettle (prior version) - Revision objective

Mandatory revision classification: - Clarification-only - Coverage
expansion - Partition adjustment - Semantic modification

Rules: - Stable Ettle ID (version changes, identity does not) - Produce
an explicit **Delta Table** (section-by-section)

Process: - Apply PHASES 1--6 to the revised node (as relevant). - For
semantic modification: - Explicit before/after behavioural summary -
Removed behaviours listed - Breaking vs non-breaking impact declared -
Updated leaf scenarios/outcomes must be unambiguous

Outputs (minimum): - Operation declaration + revision classification -
Revised Ettle (full text) - Delta Table (section-by-section) - Updated
review artefacts (reachability, obligations, underspec) - Leaf only:
scenario-type completeness revalidated

------------------------------------------------------------------------

## GLOBAL COMPLETENESS CRITERIA (Hard stop)

An authored output is invalid unless it includes: 1. Operation
declaration 2. Envelope summary (or preserved envelopes for revise) 3.
Assertable WHAT 4. Correct HOW representation for abstraction level
(leaf: Gherkin) 5. Leaf: scenario-type completeness (or justified
omissions) 6. Constraint Obligation Table 7. Reachability Matrix 8.
Monotonicity Notes (or containment validation for refine) 9.
Underspecification Report (even if empty)

------------------------------------------------------------------------

End of prompt.
