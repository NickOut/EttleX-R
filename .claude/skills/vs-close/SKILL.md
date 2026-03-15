---
name: vs-close
description: Vertical slice closure — Steps 5 through 10. Run acceptance gates, update the slice registry, produce the Plan vs Actual table, perform TDD and drift audits, and write the completion report. Invoke after /vs-implement has confirmed all scenarios are at DONE.
user-invocable: true
allowed-tools: Read, Write, Edit, Glob, Grep, Bash
---

You are executing Steps 5 through 10 of the EttleX Vertical Slice Protocol.

You MUST read `handoff/slice_plan.md` and `handoff/slice_wip.md` first.
If either does not exist, stop and tell the user to run /vs-plan, /vs-setup, and /vs-implement first.
If any row in `handoff/slice_wip.md` has Status != DONE, stop and tell the user to complete all scenarios via /vs-implement first.

Extract the following from `handoff/slice_plan.md`:
- Slice identifier (kebab-case) — used to name the completion report file
- Ettle ID
- All plan sections verbatim (for Step 10 section 6)
- Pre-Authorised Failure Registry (full list)
- Slice Registry entry (verbatim TOML as declared in the plan)

---

## STEP 5 — Global Acceptance Gate

Run each command in order. Capture full output for Step 10.

1. `make lint`
   - MUST pass with zero errors. If it fails, fix the issue and re-run before proceeding.

2. `make test-slice`
   - MUST pass with zero failures.
   - Capture the exact output (passed count, failed count).
   - If any failure occurs — STOP. Do not proceed. Report the failure to the user.

3. `make test`
   - WILL produce failures. Record the failure list.
   - Compare every failure against the Pre-Authorised Failure Registry from `handoff/slice_plan.md`.
   - If any failure is NOT in the registry — STOP. Report the unregistered failure to the user.
   - If all failures are registered — record: "N failures, all pre-authorised."

4. `make coverage-check`
   - Coverage threshold MUST NOT be modified. If the check fails, report it to the user — do not alter the threshold.

5. `make coverage-html`
   - Capture confirmation of HTML report generation.

6. `make doc`
   - MUST complete without warnings. Pre-existing warnings in crates outside the slice boundary are acceptable.
   - If new warnings appear in slice boundary crates — fix them before proceeding.

If any gate fails (other than pre-authorised `make test` failures), stop and report to the user. Do not proceed to Step 6 until all gates pass.

---

## STEP 6 — Slice Registry Update

Read `handoff/slice_registry.toml`.

Append the slice entry exactly as declared in the plan (Section 9 of the plan). The entry MUST include:
- `[[slice]]` header with `id`, `ettle_id`, `description`, `layers`, `status = "complete"`
- One `[[slice.tests]]` entry per test in the conformance table
- One `[[slice.pre_authorised_failures]]` entry per PAFR item

Verify that no test name in this entry collides with any test name already registered in the file.

Write the updated `handoff/slice_registry.toml`.

---

## STEP 7 — Plan vs Actual Review

Produce the following table covering every scenario:

| SC | Planned Test | Actual Test | Match? | Planned Modules | Actual Modules | Match? | Planned Docs | Actual Docs | Match? | Notes |

Source:
- Planned Test and Planned Modules: from `handoff/slice_plan.md` scenario inventory
- Actual Test and Actual Modules: from `handoff/slice_wip.md` Code Files column
- Planned Docs and Actual Docs: from `handoff/slice_plan.md` and `handoff/slice_wip.md` Doc Files column

For any mismatch, the Notes column MUST contain a justification. An unjustified mismatch is a failure condition — stop and report it to the user.

---

## STEP 8 — TDD Integrity Audit

For each scenario row in `handoff/slice_wip.md`, verify:
- RED Evidence column is non-empty (RED occurred before GREEN)
- GREEN Evidence column is non-empty
- Doc Evidence column is non-empty (documentation confirmed)
- Status = DONE

Then output this statement verbatim if it is true:

> No retroactive TDD occurred, all documentation obligations were satisfied, no code outside the declared slice boundary was modified, and all replacement targets have been superseded.

If it is NOT true for any scenario, state which scenarios failed the audit and what the specific violation was. Do not proceed to Step 9 until the audit passes.

---

## STEP 9 — Drift Audit Against Ettle

Read the original Ettle specification file (identified in `handoff/slice_plan.md` Step 0 file list).

Confirm for each scenario:
- Implementation outcome matches the scenario's expected outcome as described in the Ettle
- No emergent behaviour was introduced (no code without a corresponding scenario)
- All post-slice structural invariants declared in the plan's Replacement Targets section hold
- Every scenario has exactly one test; no test exists without a scenario

If drift is detected:
- Describe it explicitly
- If the drift is a deviation from the Ettle: modify code to match the Ettle, or output a Proposed Ettle Patch for user review
- Do NOT write additional tests post-hoc to legitimise emergent behaviour

---

## STEP 10 — Completion Report

Write `handoff/completed/<slice-id>_completion_report.md` where `<slice-id>` is the kebab-case identifier from `handoff/slice_plan.md`.

The report MUST contain all 18 of the following sections. Missing any section is a protocol failure.

1. **Slice identifier and Ettle reference** — slice id, ettle id, date completed
2. **Change classification** — A/B/C/D with brief justification
3. **Slice boundary declaration** — in-scope crates/modules and read-only crates/modules (verbatim from plan)
4. **Replacement targets with post-slice structural invariant confirmation** — for each replacement target: was it superseded? Does the invariant hold?
5. **Layer coverage confirmation** — for each declared layer (Store / Engine / MCP / CLI), provide test evidence (test name or test file that exercises it)
6. **Original plan (verbatim)** — the full contents of `handoff/slice_plan.md`
7. **Final conformance table** — full table from `handoff/slice_wip.md` at time of closure
8. **Plan vs Actual table** — the table from Step 7
9. **RED → GREEN evidence summary** — one row per scenario: SC ID, RED Evidence, GREEN Evidence
10. **Pre-Authorised Failure Registry** — full list with reasons (verbatim from plan)
11. **`make test` output** — verbatim or summarised, confirming only pre-authorised failures
12. **`make test-slice` output** — verbatim, showing zero failures
13. **Documentation update summary** — explicit file paths updated per scenario
14. **`make doc` confirmation** — output confirming clean build
15. **Slice Registry entry** — verbatim TOML as appended to `handoff/slice_registry.toml`
16. **Helper test justification** — if any test helper functions were written, justify them here; if none, state "None"
17. **Acceptance gate results** — all six Step 5 commands with their outcomes
18. **Integrity confirmation** — output verbatim:

> All 18 completion report sections are present.
> make test-slice: N passed, 0 failed.
> make test: M failures, all pre-authorised.
> make coverage-check: PASS (N%).
> make doc: PASS, no warnings in slice boundary crates.
> Slice registry updated.
> Plan vs Actual: N matches, 0 unjustified mismatches.
> TDD integrity: confirmed.
> Drift audit: confirmed.

---

## Cleanup

After the completion report is written:
- Rename `handoff/slice_wip.md` to `handoff/completed/<slice-id>_slice_wip.md` (archive it)
- Rename `handoff/slice_plan.md` to `handoff/completed/<slice-id>_slice_plan.md` (archive it)

This clears the working files so the next /vs-plan invocation starts clean.

---

## Final Output

```
STEP 5 COMPLETE — lint: PASS, test-slice: N passed, test: M pre-authorised failures, coverage: PASS, doc: PASS
STEP 6 COMPLETE — slice_registry.toml updated with N tests and M pre-authorised failures
STEP 7 COMPLETE — Plan vs Actual: N rows, 0 unjustified mismatches
STEP 8 COMPLETE — TDD integrity confirmed
STEP 9 COMPLETE — drift audit confirmed
STEP 10 COMPLETE — handoff/completed/<slice-id>_completion_report.md written
Working files archived to handoff/completed/
Slice <slice-id> is CLOSED.
```
