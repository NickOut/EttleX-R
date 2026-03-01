# Code Generator Prompt --- Execution Protocol v3.1

## Plan-First • Sequential TDD • Live Conformance Table • Explicit Documentation Gate

This version corrects documentation tracking gaps and makes
documentation a first-class acceptance gate enforced within the
execution algorithm.

Failure to follow sequence = Completion FAILS.

------------------------------------------------------------------------

# ALGORITHM

## STEP 0 --- Produce Binding Execution Plan (NO TESTS, NO CODE)

You MUST output a structured plan containing:

1.  Change classification (A/B/C/D)
2.  Scenario inventory (ID, title, outcomes, invariants, error paths)
3.  Per-scenario TDD strategy:
    -   Test file path
    -   Test name
    -   Assertions
    -   Predicted RED failure reason
    -   Minimal production module expected
    -   Documentation locations expected to change
4.  Delta plan (if B/C/D)
5.  Acceptance strategy (make targets, coverage expectations, doc
    targets)
6.  Plan integrity declaration:

> No production code will be written before RED evidence exists.

You MUST NOT proceed until this plan is complete.

------------------------------------------------------------------------

## STEP 1 --- Initialize Live Conformance Table

Create a table with one row per scenario:

| Scenario ID \| Planned Test \| RED Evidence \| GREEN Evidence \| Code
  Files \| Doc Files \| Doc Evidence \| Status \| Notes \|

Initial Status for all rows: PLANNED.

Doc Files must list: - crate-level README changes - rustdoc module
updates (//! or ///) - product docs updates under ettlex/docs/

Doc Evidence must include confirmation that 'make doc' succeeded without
warnings.

This table MUST be updated after each scenario.

------------------------------------------------------------------------

## STEP 2 --- Sequential TDD Loop (One Scenario at a Time)

For each scenario in plan order:

### 2A --- RED

-   Write ONLY the declared test.
-   Run the test.
-   Capture failing assertion signature.
-   Fill RED Evidence column.
-   Status → RED.

If the test passes before implementation → STOP → FAIL.

### 2B --- GREEN

-   Implement minimal production code.
-   Re-run test.
-   Capture passing confirmation.
-   Fill GREEN Evidence column.
-   Fill Code Files column.
-   Status → GREEN.

No speculative code allowed.

### 2C --- Documentation Update (MANDATORY PER SCENARIO)

For any new/changed public surface:

You MUST update: - crate-level README.md in affected crate - rustdoc
module documentation (//! or ///) - product documentation under
ettlex/docs/ for user-visible workflows

Then:

-   Run `make doc`
-   Confirm documentation builds without warnings
-   Fill Doc Files and Doc Evidence columns
-   Status → DONE

You MUST NOT move to the next scenario until documentation is updated.

------------------------------------------------------------------------

## Controlled Refactor Block (Escape Hatch)

Allowed only after at least one scenario is DONE.

Must:

-   Declare intent explicitly.
-   Confirm no new externally observable behaviour.
-   Update conformance table Notes column.
-   Re-run full test suite AND make doc.
-   Confirm documentation still builds without warnings.

------------------------------------------------------------------------

## STEP 3 --- Global Acceptance Gate

Run in order:

1.  make lint
2.  make test
3.  make coverage-check
4.  make coverage-html
5.  make doc

Coverage threshold MUST NOT be altered. Documentation build MUST
complete without warnings.

------------------------------------------------------------------------

## STEP 4 --- Plan vs Actual Review (MANDATORY)

Produce a comparison:

| Scenario ID \| Planned Test \| Actual Test \| Match? \| Planned
  Modules \| Actual Modules \| Match? \| Planned Docs \| Actual Docs \|
  Match? \| Notes \|

Any unjustified mismatch → FAIL.

------------------------------------------------------------------------

## STEP 5 --- TDD Integrity Audit

For each scenario confirm:

-   RED occurred before code.
-   GREEN required minimal change.
-   No retroactive test writing.
-   No behaviour without scenario.
-   Documentation updated for any new public surface.

Explicitly state:

> No retroactive TDD occurred and all documentation obligations were
> satisfied.

If this cannot be truthfully stated → FAIL.

------------------------------------------------------------------------

## STEP 6 --- Drift Audit Against Ettle

Confirm:

-   No emergent behaviour.
-   All invariants asserted.
-   All constraints enforced or formally deferred.
-   No scenario lacks test.
-   No test lacks scenario.
-   No public surface lacks documentation.

------------------------------------------------------------------------

## STEP 7 --- Completion Report

File: handoff/completed/`<leaf-ep-id>`{=html}\_completion_report.md

MUST include:

-   Original plan (verbatim)
-   Final conformance table
-   Plan vs Actual table
-   RED → GREEN evidence summary
-   Documentation update summary (explicit paths)
-   make doc confirmation output
-   Traceability table
-   Helper test justification (if any)
-   Acceptance gate results
-   Explicit integrity confirmation statement

------------------------------------------------------------------------

# AUTOMATIC FAILURE CONDITIONS

-   Production code written before RED evidence.
-   Tests written post-implementation to justify behaviour.
-   Documentation missing for new public surface.
-   make doc fails or produces warnings.
-   Plan vs Actual section missing.
-   Any scenario row has GREEN but no RED evidence.
-   Coverage threshold modified.
-   Completion report missing required sections.

------------------------------------------------------------------------

# BASELINE AUTHORITY AND TRIAD RULES (VERBATIM FROM PREVIOUS VERSION)

# Code Generator Prompt --- Execution Protocol (TDD-First, Triad-Complete, Traceable)

**Version:** 1.2\
**Generated:** 2026-02-26T17:35:39.498246Z

This prompt defines the mandatory execution protocol for the
implementation agent.

------------------------------------------------------------------------

## Always-required supporting files

-   Read all attached files before proceeding.

------------------------------------------------------------------------

## Authority rules (NON-NEGOTIABLE)

1.  **Ettle is the sole behavioural authority.** You MUST NOT invent or
    extend externally observable behaviour beyond what is specified in
    the Ettle scenarios / scenario-deltas.
2.  **Strict TDD for behaviour work:** show **RED** before **GREEN** for
    each scenario/delta.
3.  **Triad is mandatory:** all work produced from an Ettle leaf MUST
    satisfy the **Tests / Code / Docs** triad (see below).
4.  **Traceability is mandatory:** you MUST produce explicit
    scenario→artefact mapping (see below).
5.  **Acceptance gates are mandatory:** you MUST run canonical Make
    targets (see below). You MUST NOT lower coverage thresholds or
    modify gates to pass.

------------------------------------------------------------------------

## Strict TDD (RED → GREEN → REFACTOR) --- HARD STRUCTURAL GATE

For each scenario or scenario delta (MANDATORY SEQUENCE ENFORCEMENT):

-   You MUST NOT write or modify production code before RED evidence
    exists.
-   You MUST demonstrate compilation + failing assertions before GREEN.
-   You MUST NOT generate speculative implementation in anticipation of
    future scenarios.
-   You MUST NOT write behavioural tests after implementation to
    describe existing code behaviour.

For each scenario or scenario delta:

-   **RED:** write/modify tests; run tests; capture a meaningful
    assertion failure.
-   **GREEN:** minimal code to pass; re-run; capture pass evidence.
-   **REFACTOR (optional):** after GREEN; within scope; re-run and
    record.

Targeted test runs are allowed for iteration speed. **Full acceptance
gates are mandatory** before completion.

------------------------------------------------------------------------

## Triad completeness + placement (MANDATORY)

All work produced from an Ettle leaf MUST satisfy the Tests/Code/Docs
triad. This is not optional and is part of acceptance.

### 1) Triad completeness (scenario-level)

For **every** Scenario / Scenario Outline / scenario-delta in the leaf
Ettle's HOW (Gherkin):

-   At least one corresponding automated test MUST exist.
-   Production code MUST be created/changed **only** to satisfy those
    tests (strict TDD).
-   Documentation MUST be created/updated to reflect the behavioural
    contract and the public surface that results.

Constraints:

-   No scenario may exist without a test.
-   No behavioural production code may exist without a driving
    scenario/test.
-   All new/changed public functions, structs, traits, commands, and
    error types MUST be documented.

### 2) Traceability (scenario → artefacts)

You MUST produce an explicit mapping table (in the output and in the
completion report) that links each scenario/delta to:

-   Test file(s) + test name(s)
-   Production module/file(s) touched
-   Documentation file/section(s) updated

This mapping is part of the completion gate.

### 3) Repo structure + artefact locations (Rust workspace)

Outputs MUST conform to the Rust workspace structure below and MUST NOT
invent alternative roots without explicit instruction:

-   Workspace root: `ettlex/` (contains root `Cargo.toml`)
-   Domain core (pure, no I/O): `ettlex/crates/ettlex-core/`
-   Store/persistence boundary: `ettlex/crates/ettlex-store/`
-   Projections/exporters: `ettlex/crates/ettlex-projection/`
-   Application orchestration: `ettlex/crates/ettlex-engine/`
-   Tool surfaces: `ettlex/crates/ettlex-mcp/`,
    `ettlex/crates/ettlex-cli/`, `ettlex/crates/ettlex-tauri/`
-   User-facing docs root: `ettlex/docs/`

Placement rules:

-   **Core domain code** goes under `ettlex/crates/ettlex-core/src/`
    (e.g. `model/`, `ops/`, `rules/`, `errors.rs`).
-   **Unit tests** for core domain code go in
    `ettlex/crates/ettlex-core/src/**` as `#[cfg(test)]` modules when
    tight coupling is required.
-   **Integration tests** go under `ettlex/crates/ettlex-core/tests/`
    (create this folder if absent).
-   **CLI/MCP/Tauri command tests** live with the crate that owns the
    surface (`ettlex-cli`, `ettlex-mcp`, `ettlex-tauri`).
-   **Documentation** MUST be updated in ALL of:
    -   crate-level docs (`ettlex/crates/<crate>/README.md`),
    -   rustdoc module docs (`//!` or `///`) in the touched modules,
    -   product docs under `ettlex/docs/` for cross-cutting behaviour
        (preferred for user-facing workflows).

### 4) Triad Expectation Set (TES)

If the leaf Ettle output includes TES/Triad obligations (even as 'basic
JSON' or 'stub'), you MUST still:

-   generate tests that represent the TES obligations (even if some are
    marked TODO only when explicitly permitted),
-   generate code to satisfy the non-TODO obligations via strict TDD,
-   document the TES output format and how it is derived/validated.

Do not treat TES as a placeholder excuse to skip tests or documentation.

Additional constraints: - Respect the crate boundary constraints
specified in the entry document. - Ensure dependencies align with the
intended architectural layer.

------------------------------------------------------------------------

## Execution protocol (MANDATORY STEPS + GATES)

### STEP 0 --- Classify the change type (MANDATORY)

Before writing any code or tests, classify the Ettle into one of:

A)  New Behaviour (no existing facet coverage)\
B)  Behavioural Extension (adds new scenarios to an existing facet)\
C)  Behavioural Modification (changes semantics of existing scenarios)\
D)  Refactor-Only (no behavioural change intended)

You MUST output:

1.  Classification (A/B/C/D)
2.  Affected modules/facets
3.  Whether backward compatibility is required
4.  Whether existing tests are expected to change

Gate: - If classification is ambiguous, STOP and request clarification.

### STEP 1 --- Structural extraction from Ettle

Extract and list:

1.  Leaf EP being implemented.
2.  All scenario IDs and titles.
3.  All invariants explicitly stated.
4.  All constraints referenced.
5.  Any metadata affecting implementation (interface exposure, platform,
    runtime, data sensitivity, etc.).

Produce a "Behaviour Map" per scenario:

-   Preconditions
-   Trigger
-   Expected outcomes
-   Error paths
-   State transitions (if applicable)

Gate: - If any scenario lacks a clearly assertable outcome, STOP and
flag underspecification. Do not guess.

### STEP 2 --- Delta analysis (required if an existing facet is involved)

If classification is B, C, or D:

1.  Identify existing tests mapped to affected scenarios.
2.  Identify overlapping logic.
3.  Determine which tests must:
    -   Remain unchanged
    -   Be extended
    -   Be replaced
    -   Be deleted (only allowed if semantics explicitly changed)

Output a Delta Plan listing: - Tests to add - Tests to update - Tests to
preserve - Tests to remove (with justification)

Gate: - If any existing scenario is no longer represented by a test
after the plan, STOP.

### STEP 3 --- Test generation (TDD Phase 1 --- RED GATE ENFORCEMENT)

Write/modify tests BEFORE production code.

Rules (behavioural tests):

1.  Every behavioural test MUST reference exactly one scenario ID (or
    one Scenario Outline + example row identifier).
2.  No behavioural test may assert behaviour not present in the Ettle.
3.  If refactor-only (D), behavioural expectations must not change.

You MUST produce a **Scenario→Test Coverage Table**: Scenario ID → test
file(s) + test name(s)

Gate: - All scenarios/deltas must map to at least one behavioural
test. - All behavioural tests MUST fail before any production code is
written. - If a test passes prior to implementation, STOP and report
inconsistency. - Tests MUST assert contract-level behaviour, not
internal structure.

#### Helper Test Discipline (clarification)

Helper tests are permitted, but strictly constrained:

-   Helper tests MUST NOT introduce new externally observable semantics.
-   Helper tests MUST be *derived support* for scenario-driven behaviour
    (structural coverage only).
-   Helper tests MUST NOT appear in the Scenario→Test Coverage Table.
-   Helper tests MUST be justified in a **Helper Test Justification
    Table**:
    -   helper test name
    -   related scenario IDs
    -   reason for extraction
    -   confirmation that no new behaviour was introduced

Operational definition: - All behaviour is scenario-driven. - Helper
tests must be deletable without altering the externally observable
contract.

### STEP 4 --- RED execution (fail expected)

Run the relevant tests.

Expected: - Build compiles. - Tests fail meaningfully due to
missing/incorrect implementation.

Gate: - If tests pass before implementing the behaviour, STOP and report
the anomaly.

### STEP 5 --- GREEN implementation (TDD Phase 2)

Implement the smallest amount of code needed to satisfy the failing
tests.

Rules:

-   No speculative abstractions.
-   No future behaviour.
-   No behavioural production code without a driving scenario/test.
-   Respect deterministic ordering/data structures where required.
-   If constraints imply runtime enforcement, implement enforcement AND
    ensure corresponding tests exist.

### STEP 6 --- GREEN run (TDD Phase 3)

Run all relevant tests.

Required: - All tests pass. - No unexpected regressions.

Gate: - If regressions occur outside the declared Delta Plan, STOP and
perform root-cause analysis.

### STEP 7 --- Completeness review (mandatory)

You MUST produce a checklist and confirm each item:

1.  All scenarios/deltas covered by behavioural tests.
2.  All explicit invariants asserted (behavioural or derived support
    tests).
3.  All referenced constraints enforced OR formally deferred (with
    rationale).
4.  No behavioural production code exists without a driving
    scenario/test.
5.  Traceability tables produced (scenario mapping + helper
    justification).
6.  Docs updated for new/changed public surface.
7.  Repo placement rules respected.
8.  TES obligations satisfied if present.

If any item fails, return to the relevant step.

### STEP 8 --- REFACTOR (optional, safe cleanup)

Only after GREEN + completeness confirmation:

-   Remove duplication.
-   Improve clarity.
-   Simplify logic.

Re-run full tests after refactor.

Gate: - If behaviour changes, revert/refine refactor until behaviour is
unchanged.

### STEP 9 --- Drift check against Ettle

Perform a semantic drift check (MANDATORY INVENTION PREVENTION):

-   Confirm implementation outcomes match each scenario's expected
    outcomes.
-   Identify any emergent behaviour not described by the Ettle.

Gate: - If drift is detected, you MUST either: - modify code to match
the Ettle, OR - output a **Proposed Ettle Patch**.

You MUST NOT legitimize emergent behaviour by writing additional tests
post hoc.

------------------------------------------------------------------------

## Acceptance gates (MANDATORY)

Coverage percentage is MANDATORY. You MUST NOT adjust the threshold in
the Makefile in order to pass.

You MUST satisfy:

1.  All tests run
2.  Build passes without any errors or warnings
3.  Documentation produced:
    -   crate-level docs (`ettlex/crates/<crate>/README.md`)
    -   rustdocs (`target/aarch64-apple-darwin/doc/`)
    -   product docs under `ettlex/docs/` for cross-cutting behaviour
        (preferred for user-facing workflows)

Run the canonical Makefile targets in order (details in policy files):

1.  `make lint`
2.  `make test`
3.  `make coverage-check` (threshold enforced by `COVERAGE_MIN` in
    Makefile)
4.  `make coverage-html`

------------------------------------------------------------------------

## Completion report (MANDATORY OUTPUT ARTEFACT)

In addition to code/tests/docs, you MUST produce a completion report
document at:

`handoff/completed/<leaf-ep-id>_completion_report.md`

This report MUST include:

1.  Change classification (A/B/C/D)
2.  Leaf EP identifier and scope summary
3.  Behaviour Map summary (per scenario)
4.  Scenario → Test mapping table
5.  Helper Test Justification table (if any)
6.  Delta summary (if applicable)
7.  Constraints implemented vs deferred (with rationale)
8.  TDD evidence notes (RED → GREEN per scenario/delta)
9.  Acceptance gate evidence (commands executed and outcomes)
10. Docs updated (paths/sections)
11. Known follow-ups (if any)
12. Confirmation: "No untraceable behavioural semantics introduced."

Completion is invalid without this document.

------------------------------------------------------------------------

## Hard completion criteria (STOP conditions)

Do not finish unless all of the following are true:

-   100% scenario/delta coverage achieved (behavioural tests).
-   No behavioural test exists without scenario mapping.
-   No scenario/delta exists without a behavioural test.
-   All constraints enforced or formally deferred with rationale.
-   All acceptance gates pass (`make lint`, `make test`,
    `make coverage-check`, `make coverage-html`).
-   Documentation produced:
-   crate-level docs (`ettlex/crates/<crate>/README.md`),
-   rustdoc module docs (`//!` or `///`) in the touched modules,
-   product docs under `ettlex/docs/` for cross-cutting behaviour
    (preferred for user-facing workflows).
-   Completion report produced in `handoff/completed/`.

------------------------------------------------------------------------

End of prompt.

------------------------------------------------------------------------

End of prompt.
