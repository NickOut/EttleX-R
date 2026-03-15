---
name: vs-implement
description: Vertical slice implementation — Step 4. Execute the full RED→GREEN→Doc TDD loop for all scenarios. Invoke after vs-setup has completed and you have confirmed the Makefile and conformance table are correct.
user-invocable: true
allowed-tools: Read, Write, Edit, Glob, Grep, Bash
---

You are executing Step 4 of the EttleX Vertical Slice Protocol.

You MUST read `handoff/slice_plan.md` and `handoff/slice_wip.md` first.
If either does not exist, stop and tell the user to run /vs-plan and /vs-setup first.

You MUST execute scenarios in SC-NN order.
You MUST NOT skip the RED gate for any scenario.
You MUST NOT write production code before RED evidence exists.
You MUST NOT write tests that fit existing code — tests are written from the plan only.
You MUST update `handoff/slice_wip.md` after each scenario.

---

## Read Plan and WIP

Read `handoff/slice_plan.md` — extract scenario inventory, boundary declaration, replacement targets.
Read `handoff/slice_wip.md` — this is your conformance table. You will update it after each scenario.

---

## STEP 4 — Sequential TDD Loop

For EACH scenario in SC-NN order:

### 4A — RED

1. Write ONLY the declared test for this scenario. Test file must be within the declared slice boundary.
2. Tests are written from the plan specification ONLY. Do not look at existing production code first.
3. Run `make test-slice`.
4. Confirm the test fails (compile error or runtime failure). Capture the exact failure message.
5. If the test passes before any implementation — STOP. This is a protocol failure. Report it to the user.
6. Update `handoff/slice_wip.md`: fill RED Evidence column, set Status = RED.

### 4B — GREEN

1. Implement the minimal production code within the declared slice boundary to make this scenario pass.
2. No speculative code. No code outside the slice boundary. No behaviour without a scenario.
3. Run `make test-slice`.
4. Confirm the test passes. Capture the passing confirmation.
5. Update `handoff/slice_wip.md`: fill GREEN Evidence and Code Files columns, set Status = GREEN.

### 4C — Documentation Update (MANDATORY — do not skip)

For any new or changed public surface introduced by this scenario:

1. Update the crate-level `README.md` in every affected crate.
2. Update rustdoc (`//!` module doc or `///` item doc) for every new public function, struct, or module.
3. Update product documentation under `docs/` for any user-visible workflow change.
4. Run `make doc`.
5. Confirm documentation builds without new warnings. Pre-existing warnings in unrelated crates are acceptable.
6. Update `handoff/slice_wip.md`: fill Doc Files and Doc Evidence columns, set Status = DONE.

Do NOT move to the next scenario until Status = DONE.

---

## After All Scenarios

Run `make test-slice` one final time. All scenarios must be DONE with zero failures.

Output the final `make test-slice` result.

Output:
```
STEP 4 COMPLETE — all N scenarios at DONE
handoff/slice_wip.md updated
make test-slice: N passed, 0 failed
Invoke /vs-close to run acceptance gates and produce the completion report.
```
