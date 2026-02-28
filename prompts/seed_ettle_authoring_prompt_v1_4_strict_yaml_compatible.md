# Seed Ettle Authoring Prompt --- v1.4 (Strict, YAML-Compatible)

**Version:** 1.4\
**Generated:** 2026-02-26T18:42:20.202639Z

This prompt governs authoring of Seed files that MUST remain fully
compatible with the existing seed importer and YAML schema.

The Seed YAML structure MUST remain exactly:

schema_version: `<int>`{=html} project: name: `<string>`{=html}
ettles: - id: `<string>`{=html} title: `<string>`{=html} eps: - id:
`<string>`{=html} ordinal: `<int>`{=html} normative: `<bool>`{=html}
why: \|- what: \|- how: \|- links: - parent: `<ettle_id>`{=html}
parent_ep: `<ep_id>`{=html} child: `<ettle_id>`{=html}

No additional top-level keys are permitted. No structural schema changes
are permitted.

All discipline must be expressed inside existing fields.

------------------------------------------------------------------------

# STEP 0 --- DECLARE OPERATION (OUTSIDE YAML, FOR AUTHORING CONTEXT)

Operation (authoring context only): - CREATE_SEED_FILE -
UPDATE_SEED_FILE

This declaration MUST NOT alter the YAML schema.

------------------------------------------------------------------------

# GLOBAL AUTHORING RULES

1.  All structural discipline operates at the EP level.
2.  Refinement relationships MUST be expressed via `links`.
3.  Parent EPs are not implicitly modified by defining children.
4.  All WHAT clauses MUST be assertable.
5.  Constraints MUST be translated into explicit obligations within
    `what` and/or `how`.
6.  At least one EP in the seed file MUST be leaf-driving.
7.  No new YAML keys may be introduced.

------------------------------------------------------------------------

# EP-LEVEL AUTHORING DISCIPLINE

All validation applies per EP inside `ettles[].eps[]`.

------------------------------------------------------------------------

## 1. WHY (ep.why)

Each EP's `why` MUST:

-   State the problem at this refinement level.
-   State non-goals.
-   Clarify scope boundaries relative to parent EP (if any).
-   Avoid behavioural commitments (those belong in `what`).

------------------------------------------------------------------------

## 2. WHAT (ep.what) --- ASSERTABLE CONTRACT

Each EP's `what` MUST:

-   Contain only assertable behavioural statements.
-   Define invariants explicitly.
-   Define error semantics explicitly (trigger + expected outcome).
-   Define state transitions if stateful.
-   Define determinism/ordering guarantees if relevant.
-   Define idempotency/repeatability rules if relevant.
-   Translate constraints into explicit obligations.

Prose-only or vague behavioural language is not permitted.

If a constraint is referenced, enforcement expectations MUST be
described here.

------------------------------------------------------------------------

## 3. HOW (ep.how) --- REPRESENTATION RULES

### Abstract EPs

`how` MAY contain:

-   Decision tables
-   Rule matrices
-   State machines
-   Contracts (pre/postconditions)
-   Structured operational descriptions
-   Property/invariant validation obligations

### Leaf EPs (MANDATORY GHERKIN)

At least one EP across the entire Seed file MUST be leaf-driving.

For each leaf EP:

`how` MUST contain:

-   Gherkin Feature block
-   Scenarios and/or Scenario Outlines
-   Explicit expected outcomes
-   Explicit failure semantics where applicable

The leaf EP MUST be sufficient to drive strict TDD without
interpretation.

------------------------------------------------------------------------

# LEAF EP SCENARIO-TYPE COMPLETENESS

For each leaf EP, the following categories MUST be covered where
applicable (or explicitly justified in comments inside `how`):

1.  Happy path
2.  Negative cases (invalid input/state/forbidden ops)
3.  Explicit error paths
4.  Boundary conditions (min/max; empty; large; ordering edges)
5.  Invariants (hold & reject violations)
6.  Idempotency / repeatability (if relevant)
7.  Determinism / ordering (if relevant)
8.  State transitions (if stateful)
9.  Concurrency / re-entrancy (if relevant)
10. Security / authorisation (if relevant)
11. Observability obligations (if specified)
12. Compatibility/migration behaviour (if relevant)
13. Resource/performance limits (if specified)

If a category is not applicable, a brief justification MUST be included
as a comment in the Gherkin section.

------------------------------------------------------------------------

# MONOTONIC CONTAINMENT (EP → PARENT EP)

If refinement relationships exist via `links`:

-   Child EP scope MUST be a subset of Parent EP scope.
-   Child EP MUST NOT contradict parent invariants.
-   Child EP MUST NOT expand parent exclusions.

Containment justification MUST be described inside the child EP `why`
section.

If containment cannot be justified, the Seed is invalid.

------------------------------------------------------------------------

# REACHABILITY REQUIREMENT

For each EP:

-   Every behavioural clause in `what` MUST be reachable from `how`.

For leaf EPs:

-   Every behavioural obligation MUST be driven by at least one Gherkin
    scenario.

If unreachable clauses exist, the Seed is invalid.

------------------------------------------------------------------------

# UPDATE_SEED_FILE REQUIREMENTS

If updating an existing Seed:

-   Stable Ettle IDs MUST be preserved.
-   EP IDs MUST be preserved unless explicitly versioned.
-   Changes MUST be documented in authoring notes (outside YAML or in
    comments).
-   If behaviour changes:
    -   Before/after semantics MUST be described in authoring notes.
    -   Leaf EP scenarios MUST be updated accordingly.
    -   Scenario-type completeness MUST be revalidated.

The YAML schema itself MUST remain unchanged.

------------------------------------------------------------------------

# GLOBAL VALIDATION CHECKLIST (STRICT)

The Seed YAML is invalid unless:

1.  Schema structure matches canonical shape.
2.  At least one EP is leaf-driving.
3.  All EP `what` clauses are assertable.
4.  All leaf EPs contain valid Gherkin.
5.  Scenario-type completeness validated.
6.  Constraints expressed as enforceable obligations.
7.  Reachability satisfied.
8.  Monotonic containment satisfied (if refinement used).
9.  No new YAML keys introduced.

------------------------------------------------------------------------

# QUALITY STANDARD

The Seed file must:

-   Preserve exact YAML compatibility with importer.
-   Preserve EP-level refinement semantics.
-   Be directly implementable via strict TDD for leaf EPs.
-   Prevent behavioural invention downstream.
-   Remain stable under future refinement or revision.

------------------------------------------------------------------------

End of Seed Authoring Prompt v1.4 (Strict, YAML-Compatible)
