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


------------------------------------------------------------------------

# THREE-PHASE AUTHORING PROTOCOL (MANDATORY)

Seed authoring MUST follow three sequential phases. YAML generation is Phase 2 only.
You MUST stop after Phase 1 and ask for authorisation to proceed with Phase 2.
You MUST stop after Phase 2 and ask for authorisation to proceed with Phase 3.
Phases MUST NOT be collapsed.

------------------------------------------------------------------------


## PHASE 1 — PRE-PROCESSING (OUTSIDE YAML; REVIEW REQUIRED)

Before producing YAML, the authoring agent MUST generate a structured Pre‑Processing Report using the exact table formats below.

No free‑form prose is permitted in Phase 1 except where explicitly allowed.

-----------------------------------------------------------------------

### 1. Scope & Intent Table

| Item | Description |
|------|------------|
| Problem Statement | <one concise paragraph> |
| In Scope | <bullet list> |
| Explicit Non‑Goals | <bullet list> |
| External Interfaces Affected | <list or NONE> |

-----------------------------------------------------------------------

### 2. Normative Clause Extraction Table

If refining a parent specification:

| Source Clause (quoted) | Clause Type (MUST/SHALL/Invariant/etc.) | Applies? (Y/N) | How This Seed Addresses It |
|------------------------|------------------------------------------|----------------|----------------------------|

If greenfield (conversation-based):

| Extracted Behavioural Obligation | Confidence (High/Medium/Low) | Externally Observable? (Y/N) | Notes |
|----------------------------------|-------------------------------|------------------------------|------|

-----------------------------------------------------------------------

### 3. Behavioural Fork Enumeration Table (MANDATORY)

All observable behavioural forks MUST be listed.

| Row ID | Behavioural Domain | Fork Description | Options | External Impact | Alignment With System Invariants | Proposed Selection |
|--------|-------------------|-----------------|---------|-----------------|----------------------------------|--------------------|

Rules:
- Each row MUST have a stable Row ID (e.g., BF-01, BF-02).
- Row IDs MUST be referenced in Phase 3 Fork Resolution Verification.
- No behavioural MAY language is permitted in final WHAT.
- Every externally observable fork MUST have exactly one selected option.
- Unresolved forks block YAML generation.

-----------------------------------------------------------------------

### 4. Decision Confirmation Block

The authoring agent MUST end Phase 1 with:

Decision Confirmation Required:

- All normative clauses accounted for: YES/NO
- All behavioural forks resolved: YES/NO
- Governing invariants declared (if greenfield): YES/NO

YAML generation MUST NOT proceed until all answers are YES.


------------------------------------------------------------------------

## PHASE 2 — YAML PRODUCTION (CONTROLLED TRANSITION)

YAML production is a controlled transition from Phase 1 decisions into enforceable EP structure.

YAML MUST NOT be produced until the Phase 1 Decision Confirmation Block reports all YES.

Before emitting YAML, the authoring agent MUST explicitly state:

Transition to Phase 2 Confirmed:
- Phase 1 tables complete: YES
- All Row IDs (Behavioural Forks) resolved: YES
- No unresolved normative clauses: YES

If any answer is NO, YAML generation MUST NOT proceed.

------------------------------------------------------------------------

### Phase 2 Execution Workflow (MANDATORY)

1. Map each Normative Clause (Phase 1 Table) to a specific EP and section (WHY/WHAT/HOW).
2. Encode behavioural selections (Behavioural Fork Row IDs) as normative WHAT clauses.
3. Apply EP-Level Authoring Discipline rules (WHY / WHAT / HOW).
4. Apply Global Authoring Rules.
5. Validate Leaf EP Scenario-Type Completeness requirements.
6. Validate Reachability Requirement.
7. Validate Monotonic Containment (if links used).

Only after completing steps 1–7 may YAML be finalised.

------------------------------------------------------------------------

### Phase 2 Rules

- YAML schema MUST remain unchanged.
- Selected behavioural options MUST be encoded normatively.
- No behavioural MAY or SHOULD clauses are permitted in WHAT.
- All normative obligations MUST be assertable.
- All parent invariants (if any) MUST be preserved or explicitly scoped out.
- Each Behavioural Fork Row ID MUST be traceable to one or more normative WHAT clauses. Traceability MUST be demonstrated in the Phase 3 Fork Resolution Table. Row IDs MUST NOT appear in YAML.- Scenario-type completeness MUST be satisfied.
- Global Authoring Rules and EP-Level Authoring Discipline sections are binding and MUST be applied during Phase 2.

Failure to satisfy any item blocks progression to Phase 3.

------------------------------------------------------------------------

## PHASE 3 — POST-YAML QUALITY CHECK (FIXED TABLE FORMAT — MANDATORY)

After YAML generation, the authoring agent MUST produce a Post-YAML Review Report in the exact fixed table formats below.

Free-form narrative is not permitted except in the “Notes” column.

Seed ingestion MUST NOT occur unless all required tables are present and overall status is PASS.

------------------------------------------------------------------------

## 3.1 Invariant Compliance Table (MANDATORY)

Every normative clause extracted in Phase 1 MUST appear as a row in this table.

| Clause ID | Clause (Quoted from Phase 1) | Expected Location (EP / Section) | Found? (Y/N) | YAML Line Number(s) | Verbatim YAML Excerpt | Status (PASS/FAIL) | Notes |
|-----------|-----------------------------|-----------------------------------|--------------|---------------------|-----------------------|--------------------|------|

Rules:

- “Found?” = Y only if the clause is assertable and normatively encoded.
- “Status” = PASS only if:
  - The clause appears in WHAT (not only HOW), OR
  - It is correctly expressed as a scenario obligation where appropriate.
- Line numbers MUST reference the YAML file.
- Verbatim excerpt MUST be included for each row.
- If FAIL, Notes MUST explain precisely why.
- Listing line numbers without excerpts is invalid.

------------------------------------------------------------------------

## 3.2 Behavioural Fork Resolution Table (MANDATORY)

| Behavioural Domain | Selected Option (Phase 1) | YAML Location(s) | Matches Selection? (Y/N) | Unexpected Fork Introduced? (Y/N) | Status | Notes |
|-------------------|---------------------------|------------------|--------------------------|-----------------------------------|--------|------|

Rules:

- If any “Unexpected Fork Introduced?” = Y → overall FAIL.
- If YAML contains discussion-only language (e.g. Option A/B, Phase labels, CORE/MEDIUM/FUTURE labels) → FAIL.
- Behavioural MAY/SHOULD inside WHAT → automatic FAIL.
- Conditional logic inside scenarios is NOT automatically a failure unless it alters external semantics.

------------------------------------------------------------------------

## 3.3 Scenario-Type Completeness Table (PER LEAF EP)

For each leaf EP:

| Scenario Category | Present? (Y/N) | YAML Line Number(s) | Verbatim Scenario Title(s) | Justification if N | Status |
|-------------------|----------------|---------------------|----------------------------|--------------------|--------|

Categories (mandatory evaluation):

1. Happy path  
2. Negative cases  
3. Explicit error paths  
4. Boundary conditions  
5. Invariants  
6. Idempotency / repeatability (if relevant)  
7. Determinism / ordering (if relevant)  
8. State transitions (if stateful)  
9. Concurrency / re-entrancy (if relevant)  
10. Security / authorisation (if relevant)  
11. Observability obligations (if specified)  
12. Compatibility / migration behaviour (if relevant)  
13. Resource / performance limits (if specified)  
14. Explicit prohibition scenarios (behaviour MUST NOT occur)  
15. Deterministic byte-level equivalence (if serialization exists)  
16. Concurrency conflict scenarios (if optimistic gating/head semantics exist)

Rules:

- If a category is not applicable, justification MUST be explicit.
- Missing categories without justification → FAIL.

------------------------------------------------------------------------

## 3.4 No-Invention Audit Table (MANDATORY)

| Behaviour Introduced in YAML | Present in Phase 1 Extraction? (Y/N) | YAML Location(s) | Status | Notes |
|------------------------------|--------------------------------------|------------------|--------|------|

Rules:

- Any externally observable behaviour not present in Phase 1 extraction → FAIL.
- Implementation details internal to HOW that do not affect observable semantics are not considered invention.

------------------------------------------------------------------------

## 3.5 Final Outcome

| Check Category | Status |
|---------------|--------|
| Invariant Compliance | PASS/FAIL |
| Fork Resolution | PASS/FAIL |
| Scenario Completeness | PASS/FAIL |
| No-Invention Audit | PASS/FAIL |
| OVERALL | PASS/FAIL |

OVERALL must be PASS before ingestion.

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

# LEAF EP SCENARIO-TYPE COMPLETENESS (MANDATORY — NOT GUIDANCE)

For each leaf EP, the following categories MUST be covered.
If a category is not applicable, an explicit justification comment MUST appear inside the Gherkin block.
Omission without justification renders the Seed invalid.

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
14. Explicit prohibition scenarios (behaviour MUST NOT occur)
15. Deterministic byte-level equivalence scenarios (where serialization exists)
16. Concurrency conflict scenarios (if optimistic gating or head semantics exist)

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

- Every behavioural clause in `what` MUST be reachable from `how`.
- Every externally observable effect MUST be asserted by at least one scenario.
- No behavioural allowance may exist in `what` without explicit scenario coverage.

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

End of Seed Authoring Prompt v1.7 (Three‑Phase Authoring Protocol with Structured Tables)
