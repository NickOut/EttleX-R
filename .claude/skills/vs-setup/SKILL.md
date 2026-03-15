---
name: vs-setup
description: Vertical slice setup — Steps 2 and 3. Initialise the conformance table and update the Makefile. Invoke after the plan in handoff/slice_plan.md has been approved.
user-invocable: true
allowed-tools: Read, Write, Edit, Bash
---

You are executing Steps 2 and 3 of the EttleX Vertical Slice Protocol.

You MUST read `handoff/slice_plan.md` first. If it does not exist, stop and tell the user to run /vs-plan first.
You MUST NOT write any tests or production code.
You MUST NOT proceed to vs-implement without user confirmation.

---

## Read Plan

Read `handoff/slice_plan.md` and extract:
- Slice identifier (kebab-case id)
- All scenario IDs and test names from the scenario inventory
- All planned test names for the Makefile update

---

## STEP 2 — Initialise Live Conformance Table

Write `handoff/slice_wip.md` with the following structure:

```markdown
# Slice WIP — <slice-id>

**Ettle ID:** <ettle-id>
**Status:** IN PROGRESS

## Conformance Table

| SC | Layer(s) | Planned Test | RED Evidence | GREEN Evidence | Code Files | Doc Files | Doc Evidence | Status |
|----|----------|-------------|--------------|----------------|------------|-----------|--------------|--------|
| SC-NN | ... | test_name | | | | | | PLANNED |
...one row per scenario, all Status = PLANNED...
```

One row per scenario from the plan. All rows must have Status = PLANNED.
Doc Files column must list what rustdoc/README/product-doc updates are expected for that scenario's public surface.

---

## STEP 3 — Makefile Update

Update the `SLICE_TEST_FILTER` in `makefile` to append all new test names from the plan to the existing filter regex.

The existing filter must be preserved exactly. New test names are appended inside the existing regex alternation.

After updating, run:
```
make lint
```

Capture and output the full lint result.

If lint fails, fix the Makefile and re-run before proceeding.

---

## STOP

Output:
```
STEP 2 COMPLETE — handoff/slice_wip.md written with N rows at PLANNED
STEP 3 COMPLETE — Makefile updated, make lint: PASS
Awaiting your confirmation before proceeding.
Invoke /vs-implement to begin TDD implementation.
```

Do NOT write any tests or production code.
