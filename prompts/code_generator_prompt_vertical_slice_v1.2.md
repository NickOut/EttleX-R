# Code Generator Prompt — Vertical Slice Execution Protocol v1.2

## Slice-First • Sequential TDD • Slice Registry • Explicit Layer Coverage Gate

This prompt governs implementation of vertical slices through the EttleX
codebase. Each slice covers all layers from MCP to Store for a defined
set of behaviour. Existing code outside the slice boundary is retained
unchanged until a future slice explicitly addresses it.

Failure to follow sequence = Completion FAILS.

---

# AUTHORITY

## The Ettle is the sole behavioural authority

The Ettle provided for this slice is the complete specification. It
covers all layers the slice touches. You MUST NOT invent or extend
externally observable behaviour beyond what is specified in the Ettle.

The Ettle may span Store, Engine/Action, and MCP concerns within a
single document. This is intentional. Treat the full Ettle content as
the normative source for all layers.

The Ettle already incorporates all applicable constraints. Constraints
direct what goes into an Ettle at authoring time; by the time a slice
reaches a code generator, constraint content is embedded in the Ettle
itself. The code generator reads the Ettle only.

## The Slice Registry is the sole authority on slice scope

`handoff/slice_registry.toml` records every slice that has been
implemented. Before writing any plan, you MUST read this file to
understand what previous slices declared as in scope and which tests
they registered. You MUST NOT duplicate test names already registered
by a prior slice.

---

# ALGORITHM
READ THE ENTIRE PROMPT BEFORE EXECUTING AND ENSURE THAT ALL STEPS ARE FOLLOWED.

## STEP 0 — Read Supporting Files (MANDATORY FIRST STEP)

Before producing any plan, you MUST read all of the following:

1. The Ettle for this slice (provided as the input specification).
2. `handoff/slice_registry.toml` — prior slice scope and registered tests.
3. `makefile` — existing targets before modifying them.
4. `CLAUDE.md` at the workspace root — project conventions.
5. `handoff/EttleX_Logging_Facility_Rust_FULL_Ettle_v3.md` — the
   canonical logging facility specification. All new code MUST conform
   to the logging conventions defined here: single initialisation point,
   canonical event schema, boundary ownership rules, correlation
   propagation, and redaction requirements.
6. `handoff/EttleX_Rust_Common_Error_Handling_Facility_FULL_Ettle_v3.md`
   — the canonical error handling facility specification. All new code
   MUST use `ExError` and `ExErrorKind` as defined here. No ad-hoc
   error types at public boundaries. Determinism rules apply.

You MUST NOT proceed to Step 1 until all six files have been read.

The logging and error handling facility documents are implementation
references. Any scenario whose implementation would violate the
conventions in these documents is underspecified and MUST be flagged
before planning begins.

---

## STEP 1 — Produce Binding Execution Plan (NO TESTS, NO CODE)

You MUST output a structured plan containing:

1. **Slice identifier** — a short kebab-case name for this slice
   (e.g. `ettle-crud`). This will be used as the slice key in the
   registry.
2. **Change classification** (A/B/C/D):
   - A: New behaviour — no existing coverage for this behaviour.
   - B: Behavioural extension — adds new behaviour alongside existing.
   - C: Behavioural modification — changes semantics of existing behaviour.
   - D: Refactor-only — no behavioural change intended.
3. **Slice boundary declaration** — explicit list of:
   - Crates in scope (e.g. `ettlex-store`, `ettlex-engine`, `ettlex-mcp`).
   - Modules in scope within each crate.
   - Crates and modules that are read-only (outside the boundary).
4. **Replacement targets** — for any existing functions, modules, or
   dispatch logic being replaced (not extended) by this slice:
   - File path and function/module name.
   - Statement that it is superseded, not extended.
   - The post-slice structural invariant that must hold
     (e.g. "dispatch_mcp_command MUST contain no Ettle business logic
     after this slice").
5. **Layer coverage declaration** — explicit statement of which layers
   this slice covers (Store / Engine / Action / MCP / CLI) and
   confirmation that all declared layers will be represented in the
   test suite.
6. **Pre-Authorised Failure Registry (PAFR)** — list of existing tests
   that will fail as a direct consequence of this slice. For each:
   - Full test path (crate + file + test function name).
   - Reason for failure.
   - Confirmation that the test logic will not be modified.
7. **Deferred Items** — list of architectural obligations, layer
   violations, or known debt items surfaced during planning or
   implementation that are explicitly deferred to a future slice.
   For each:
   - Description of the issue.
   - Why it is deferred (not in scope, depends on future work, etc.).
   - The future slice or trigger condition that owns the resolution
     (e.g. "snapshot-commit slice", "Agent API slice").
   This section MUST be present in every plan. If no items are
   deferred, state "None identified." Deferred items are not failures
   — they are forward obligations. They are distinct from PAFR entries,
   which are accepted test failures for this slice only.
8. **Scenario inventory** — for each scenario in the Ettle:
   - Scenario ID and title.
   - Layer(s) the scenario exercises.
   - Preconditions, trigger, expected outcomes, error paths.
   - Predicted RED failure reason.
   - Minimal production module expected to satisfy it.
9. **Makefile update plan** — explicit description of:
   - New `test-slice` target to be added.
   - How existing `test` target will be handled.
   - Coverage target scoping.
10. **Slice Registry update plan** — the entry that will be appended to
   `handoff/slice_registry.toml` on completion.
11. **Acceptance strategy** — make targets, coverage scope, doc targets.
12. **Plan integrity declaration**:

> No production code will be written before RED evidence exists.
> No code outside the declared slice boundary will be modified
> except: the Makefile and handoff/slice_registry.toml.
> All replacement targets have been identified and their post-slice
> structural invariants declared.

You MUST NOT proceed until this plan is confirmed complete.

Once the plan is confirmed by the user, you MUST write the full approved
plan to `handoff/<slice-id>_plan.md` before proceeding to Step 2. The
file MUST contain the complete plan as confirmed — all sections, verbatim.
This is a mandatory artefact. Failure to write the plan file before
proceeding to Step 2 → FAIL.

---

## STEP 2 — Initialize Live Conformance Table

Create a table with one row per scenario:

| Scenario ID | Layer(s) | Planned Test | RED Evidence | GREEN Evidence | Code Files | Doc Files | Doc Evidence | Status | Notes |

Initial Status for all rows: PLANNED.

Doc Files must list:
- Crate-level README changes (in affected crate).
- Rustdoc module updates (`//!` or `///`).
- Product docs updates under `ettlex/docs/`.

Doc Evidence must include confirmation that `make doc` succeeded without
warnings.

This table MUST be updated after each scenario.

---

## STEP 3 — Makefile Update (BEFORE ANY TESTS OR CODE)

Before writing any tests, update the `makefile` to add:

```makefile
# Slice test targets — driven by handoff/slice_registry.toml
test-slice:
	cargo nextest run --workspace $(SLICE_TEST_FILTER)

test-full:
	cargo nextest run --workspace
```

Where `SLICE_TEST_FILTER` is constructed from the test names registered
in `handoff/slice_registry.toml` for all slices implemented so far,
plus this slice's planned tests. The filter uses nextest's
`--test-threads` and name-based filtering to run only registered slice
tests.

The existing `test` target MUST remain unchanged and continues to run
the full suite. Its output is expected to include pre-authorised
failures; this is not a failure condition for the slice.

Coverage targets (`coverage-check`, `coverage-html`) MUST be updated to
operate against `test-slice` scope, not `test` (full suite) scope, for
the duration of slice development. The coverage threshold applies to
slice-scoped tests only.

Update the `help` target to describe the new targets.

Run `make lint` after the Makefile update to confirm no syntax errors.

---

## STEP 4 — Sequential TDD Loop (One Scenario at a Time)

For each scenario in plan order:

### 4A — RED

- Write ONLY the declared test for this scenario.
- The test file MUST be within the declared slice boundary.
- Run `make test-slice` (not `make test`).
- Capture the failing assertion signature.
- Fill RED Evidence column.
- Status → RED.

If the test passes before implementation → STOP → FAIL.

### 4B — GREEN

- Implement minimal production code within the declared slice boundary.
- Re-run `make test-slice`.
- Capture passing confirmation.
- Fill GREEN Evidence column and Code Files column.
- Status → GREEN.

No speculative code allowed. No code outside the slice boundary.

### 4C — Documentation Update (MANDATORY PER SCENARIO)

For any new or changed public surface:

You MUST update:
- Crate-level `README.md` in the affected crate.
- Rustdoc module documentation (`//!` or `///`).
- Product documentation under `ettlex/docs/` for user-visible
  workflows.

Then:
- Run `make doc`.
- Confirm documentation builds without warnings.
- Fill Doc Files and Doc Evidence columns.
- Status → DONE.

You MUST NOT move to the next scenario until documentation is updated.

---

## Controlled Refactor Block (Escape Hatch)

Allowed only after at least one scenario is DONE.

Must:
- Declare intent explicitly.
- Confirm no new externally observable behaviour.
- Confirm refactor stays within the declared slice boundary.
- Update conformance table Notes column.
- Re-run `make test-slice` AND `make doc`.
- Confirm both complete cleanly.

---

## STEP 5 — Global Acceptance Gate

Run in order:

1. `make lint`
2. `make test-slice` — MUST pass with zero failures.
3. `make test` — WILL produce failures. Record the failure list.
   Confirm every failure is in the Pre-Authorised Failure Registry.
   Any failure NOT in the registry → STOP → FAIL.
4. `make coverage-check` (scoped to slice tests).
5. `make coverage-html` (scoped to slice tests).
6. `make doc` — MUST complete without warnings.

Coverage threshold MUST NOT be altered. Documentation build MUST
complete without warnings.

---

## STEP 6 — Slice Registry Update (MANDATORY)

Append the completed slice entry to `handoff/slice_registry.toml`.

The entry format is:

```toml
[[slice]]
id = "ettle-crud"
ettle_id = "ettle:your-ettle-id"
description = "Ettle CRUD — Store, Engine/Action, MCP"
layers = ["store", "engine", "mcp"]
status = "complete"

[[slice.tests]]
crate = "ettlex-engine"
file = "tests/ettle_crud_tests.rs"
test = "test_ettle_create_succeeds"
scenario = "SC-01"

[[slice.tests]]
crate = "ettlex-engine"
file = "tests/ettle_crud_tests.rs"
test = "test_ettle_create_rejects_empty_title"
scenario = "SC-02"

# ... one entry per registered test ...

[[slice.pre_authorised_failures]]
crate = "ettlex-engine"
file = "tests/legacy_test.rs"
test = "test_old_behaviour"
reason = "References old field name superseded by this slice"
```

Every test in the conformance table MUST appear in the registry entry.
Every pre-authorised failure MUST appear in `pre_authorised_failures`.

---

## STEP 7 — Plan vs Actual Review (MANDATORY)

Produce a comparison table:

| Scenario ID | Planned Test | Actual Test | Match? | Planned Modules | Actual Modules | Match? | Planned Docs | Actual Docs | Match? | Notes |

Any unjustified mismatch → FAIL.

---

## STEP 8 — TDD Integrity Audit

For each scenario confirm:

- RED occurred before code.
- GREEN required minimal change.
- No retroactive test writing.
- No behaviour without scenario.
- Documentation updated for any new public surface.
- No code modified outside the declared slice boundary (except Makefile
  and slice registry).
- All replacement targets identified in the plan are cleaned up in the
  implementation (no dead dispatch paths remaining).

Explicitly state:

> No retroactive TDD occurred, all documentation obligations were
> satisfied, no code outside the declared slice boundary was modified,
> and all replacement targets have been superseded.

If this cannot be truthfully stated → FAIL.

---

## STEP 9 — Drift Audit Against Ettle

Confirm:

- No emergent behaviour.
- All invariants asserted.
- All post-slice structural invariants verified (see plan Step 4).
- No scenario lacks a test.
- No test lacks a scenario.
- No public surface lacks documentation.
- Implementation outcomes match each scenario's expected outcomes.

If drift is detected, you MUST either:
- Modify code to match the Ettle, OR
- Output a Proposed Ettle Patch for review.

You MUST NOT legitimise emergent behaviour by writing additional tests
post hoc.

---

## STEP 10 — Completion Report (MANDATORY OUTPUT ARTEFACT)

File: `handoff/completed/<slice-id>_completion_report.md`

MUST include:

1. Slice identifier and Ettle reference.
2. Change classification (A/B/C/D).
3. Slice boundary declaration (in-scope and read-only).
4. Replacement targets with post-slice structural invariant confirmation.
5. Layer coverage confirmation (each declared layer evidenced).
6. Original plan (verbatim).
7. Final conformance table.
8. Plan vs Actual table.
9. RED → GREEN evidence summary (per scenario).
10. Pre-Authorised Failure Registry (full list with reasons).
11. Deferred Items (full list with resolution owners).
12. `make test` output showing pre-authorised failures only.
13. `make test-slice` output showing zero failures.
14. Documentation update summary (explicit paths).
15. `make doc` confirmation output.
16. Slice Registry entry (verbatim as appended).
17. Helper test justification (if any).
18. Acceptance gate results (all commands and outcomes).
19. Explicit integrity confirmation statement.

---

# COEXISTENCE RULE (HARD)

Code outside the declared slice boundary MUST NOT be modified.
This includes: source files, test files, seed files, schema files,
and documentation outside the declared crate scope.

The ONLY exceptions are:
- `makefile` — updated per Step 3.
- `handoff/slice_registry.toml` — updated per Step 6.
- `handoff/<slice-id>_plan.md` — written per Step 1 on plan approval.

Violation of the coexistence rule → FAIL regardless of test results.

**Exception for infrastructure slices:** A slice whose explicit purpose
is the retirement of a type or facility (e.g. `EttleXError`) is
permitted to update files throughout the workspace that reference the
retiring construct, since those files are logically within the boundary
of the retirement. This exception MUST be stated explicitly in the
slice's boundary declaration.

---

# AUTOMATIC FAILURE CONDITIONS

- Production code written before RED evidence.
- Tests written post-implementation to justify behaviour.
- Documentation missing for any new public surface.
- `make doc` fails or produces warnings.
- `make test-slice` produces any failure.
- `make test` produces a failure not in the Pre-Authorised Failure
  Registry.
- Plan vs Actual section missing from completion report.
- Any scenario row has GREEN but no RED evidence.
- Coverage threshold modified.
- Completion report missing any required section.
- Slice Registry entry missing or incomplete.
- Approved plan not written to `handoff/<slice-id>_plan.md` before
  Step 2 begins.
- Code modified outside the declared slice boundary (other than
  Makefile and slice registry, or the infrastructure exception above).
- Layer coverage declaration not evidenced in the test suite.
- Replacement targets identified in the plan but not cleaned up.

---

# SLICE REGISTRY FORMAT REFERENCE

`handoff/slice_registry.toml` accumulates entries across all slices.
It is the source of truth for:
- Which tests belong to which slice.
- Which tests are pre-authorised failures for each slice.
- The cumulative set of tests that `make test-slice` must run.

The Makefile `test-slice` target is derived from this file. Each slice
agent is responsible for updating both the registry and the Makefile
`SLICE_TEST_FILTER` consistently. On reading the registry, confirm that
your planned test names do not collide with any already registered test
name across all prior slices.

---

End of prompt.
