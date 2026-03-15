---
name: vs-plan
description: Vertical slice planning — Steps 0 and 1. Read mandatory files and produce the binding execution plan. Use when starting a new vertical slice implementation.
argument-hint: "<ettle-spec-file>"
user-invocable: true
allowed-tools: Read, Glob, Grep, Bash, Write
---

You are executing Steps 0 and 1 of the EttleX Vertical Slice Protocol.

The Ettle specification to plan is: $ARGUMENTS

You MUST stop after producing the plan and wait for explicit user approval.
You MUST NOT write any tests or production code.
You MUST NOT proceed to vs-setup until the user confirms the plan.

---

## STEP 0 — Read Supporting Files (MANDATORY FIRST STEP)

Before producing any plan, read ALL of the following. Do not skip any.
Output a confirmation line for each file as you read it.

1. The Ettle specification: $ARGUMENTS
2. `handoff/slice_registry.toml` — prior slice scope and registered tests. You MUST NOT use any test name already registered here.
3. `makefile` — existing targets before proposing modifications.
4. `CLAUDE.md` — project conventions.
5. `handoff/EttleX_Logging_Facility_Rust_FULL_Ettle_v3.md` — logging conventions. All new code must conform.
6. `handoff/EttleX_Rust_Common_Error_Handling_Facility_FULL_Ettle_v3.md` — error handling conventions. All new code must use `ExError`/`ExErrorKind`.

Output after Step 0:
```
STEP 0 COMPLETE — Files read:
✅ [spec file]
✅ handoff/slice_registry.toml
✅ makefile
✅ CLAUDE.md
✅ handoff/EttleX_Logging_Facility_Rust_FULL_Ettle_v3.md
✅ handoff/EttleX_Rust_Common_Error_Handling_Facility_FULL_Ettle_v3.md
```

---

## STEP 1 — Produce Binding Execution Plan (NO TESTS, NO CODE)

Produce a structured plan containing ALL of the following sections.
Missing any section is a protocol failure.

### 1. Slice Identifier
A short kebab-case name (e.g. `ettle-crud`).

### 2. Change Classification
- A: New behaviour — no existing coverage.
- B: Behavioural extension — adds new behaviour alongside existing.
- C: Behavioural modification — changes semantics of existing behaviour.
- D: Refactor-only — no behavioural change intended.

### 3. Slice Boundary Declaration
Explicit list of:
- Crates in scope and modules within each crate.
- Crates and modules that are read-only (outside boundary).
- Any infrastructure exceptions (mechanical changes to out-of-boundary files) — must be stated explicitly and justified.

### 4. Replacement Targets
For any existing function, module, or dispatch logic being replaced (not extended):
- File path and function/module name.
- Statement that it is superseded, not extended.
- The post-slice structural invariant that must hold.

### 5. Layer Coverage Declaration
Explicit statement of which layers this slice covers (Store / Engine / Action / MCP / CLI) and confirmation that all declared layers will be represented in the test suite.

### 6. Pre-Authorised Failure Registry (PAFR)
List of existing tests that will fail as a direct consequence of this slice. For each:
- Full test path (crate + file + test function name).
- Reason for failure.
- Confirmation that the test logic will NOT be modified.

### 7. Scenario Inventory
For each scenario:
- Scenario ID (SC-NN) and title.
- Layer(s) the scenario exercises.
- Expected error kind (if error path).
- Predicted RED failure reason.
- Minimal production module expected to satisfy it.

### 8. Makefile Update Plan
- New test names to be added to SLICE_TEST_FILTER.
- Confirmation that existing `test` and `test-full` targets are unchanged.

### 9. Slice Registry Update Plan
The exact TOML entry that will be appended to `handoff/slice_registry.toml` on completion, including the correct `id` (kebab-case slice identifier) and `ettle_id` (from the Ettle spec).

### 10. Acceptance Strategy
Make targets and coverage scope.

### 11. Plan Integrity Declaration
Output verbatim:
> No production code will be written before RED evidence exists.
> No code outside the declared slice boundary will be modified except the Makefile and handoff/slice_registry.toml (and any declared infrastructure exceptions).
> All replacement targets have been identified and their post-slice structural invariants declared.

---

## Write Plan to File

Write the complete plan to `handoff/slice_plan.md`.
This file is read by vs-setup, vs-implement, and vs-close.

---

## STOP

Output:
```
PLAN WRITTEN to handoff/slice_plan.md
Awaiting your approval before proceeding.
Invoke /vs-setup to continue once you have approved the plan.
```

Do NOT proceed further. Do NOT write any code or tests.
