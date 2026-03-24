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

## Test execution rules (NON-NEGOTIABLE)

ALL test runs MUST use `make test-slice`. Direct `cargo nextest` invocations are PROHIBITED.

- CORRECT: `make test-slice`
- PROHIBITED: `cargo nextest run ...` (with any flags, including --test-threads, --workspace, or -p)

The `make test-slice` target is scoped to registered tests only and runs with parallel execution. Direct nextest invocations bypass the scope filter, serialise execution, and cause unnecessary full-workspace compilation. Any deviation from `make test-slice` is a protocol violation.

## Timeout and hang rules (NON-NEGOTIABLE)

If `make test-slice` has not returned any output after 3 minutes:
1. Interrupt the command immediately (do not wait longer).
2. Report to the user: "make test-slice has not returned output after 3 minutes — possible hang or very slow compile. Stopping."
3. Wait for explicit user instruction before retrying.
4. NEVER attempt to work around a slow or hanging test run by switching to direct `cargo nextest` invocations, background tasks, polling temp files, or any other method.

Silently waiting and polling background task output files is a protocol violation equivalent to using direct cargo nextest. Fail loudly; do not degrade silently.

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
3. Run `make test-slice` (ONLY — never `cargo nextest` directly).
4. Confirm the test fails (compile error or runtime failure). Capture the exact failure message.
5. If the test passes before any implementation — STOP. This is a protocol failure. Report it to the user.
6. Update `handoff/slice_wip.md`: fill RED Evidence column, set Status = RED.

### 4B — GREEN

1. Implement the minimal production code within the declared slice boundary to make this scenario pass.
2. No speculative code. No code outside the slice boundary. No behaviour without a scenario.
3. Run `make test-slice` (ONLY — never `cargo nextest` directly).
4. Confirm the test passes. Capture the passing confirmation.
5. Update `handoff/slice_wip.md`: fill GREEN Evidence and Code Files columns, set Status = GREEN.

### 4C — Documentation Update (MANDATORY — do not skip)

For any new or changed public surface introduced by this scenario:

1. Update the crate-level `README.md` in every affected crate.
2. Update rustdoc (`//!` module doc or `///` item doc) for every new public function, struct, or module.
3. Update product documentation under `docs/` for any user-visible workflow change.
4. **MCP tool surface** — if this scenario introduces or removes any write command (dispatched through `ettlex_apply`) or read tool (a `tool_def` entry in `handle_tools_list()`):
   - Open `crates/ettlex-mcp/src/main.rs` and locate `handle_tools_list()`.
   - For each **new write command tag**: add it to the `ettlex_apply` description string AND to the `command.description` schema field inside the JSON object.
   - For each **removed write command tag**: remove it from both.
   - For each **new read tool**: add a `tool_def(...)` block with correct name, description, and input schema.
   - For each **removed read tool** (e.g. backing table dropped): remove its `tool_def(...)` block entirely.
   - Run `make lint` and confirm no regressions before proceeding.
5. Run `make doc`.
6. Confirm documentation builds without new warnings. Pre-existing warnings in unrelated crates are acceptable.
7. **For destructive scenarios (any deletion)** — stale documentation is actively misleading. Before marking 4C complete, run for EACH deleted entity:
   ```
   grep -rn "<deleted_entity_name>" crates/*/README.md docs/
   ```
   Every hit MUST be removed or updated. This step is NON-NEGOTIABLE — "I deleted code, there is nothing to document" is incorrect. Removals require more documentation work, not less.
8. Update `handoff/slice_wip.md`: fill Doc Files and Doc Evidence columns, set Status = DONE.

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
