# Code Generation Handoff Policy v1

This document defines the obligations that govern AI code-generation handoff sessions
for the EttleX project. Each obligation is wrapped in `<!-- HANDOFF: START -->` and
`<!-- HANDOFF: END -->` markers so they can be extracted by the `policy_export`
operation with `export_kind = "codegen_handoff"`.

---

<!-- HANDOFF: START -->

## B1.1 — Behaviour Authority

The Ettle seed document (YAML) is the authoritative specification for all behaviour.
When a seed scenario conflicts with any other instruction, the seed wins.
All implementation decisions must be traceable to a seed scenario or an explicitly
documented deviation.

<!-- HANDOFF: END -->

<!-- HANDOFF: START -->

## B1.2 — Strict TDD Protocol

All code changes follow RED→GREEN→REFACTOR order per TDD cycle:

1. Write tests referencing types/functions that do not yet exist (compile failure = RED gate).
2. Confirm the RED gate (do not skip).
3. Write minimal production code to pass the tests (GREEN).
4. Optionally clean up (REFACTOR).

Implementing production code before tests is a protocol violation.

<!-- HANDOFF: END -->

<!-- HANDOFF: START -->

## B1.3 — No Speculative Implementation

Only implement what is directly required by the current seed scenario set.
Do not add features, refactor unrelated code, or introduce helpers for hypothetical
future requirements. Scope creep invalidates coverage counts and masks test gaps.

<!-- HANDOFF: END -->

<!-- HANDOFF: START -->

## B1.4 — Error Kind Taxonomy

All public errors must be expressed via `ExErrorKind` variants with stable codes
(`ERR_*` prefix). New variants require: (a) enum arm, (b) `code()` match arm,
(c) inline test asserting the code string. Error kinds must not be reused across
semantically distinct failure modes.

<!-- HANDOFF: END -->

<!-- HANDOFF: START -->

## B1.5 — Coverage Gate

Coverage must not fall below the `COVERAGE_MIN` threshold (currently 80%).
Each TDD cycle GREEN phase must include coverage validation (`make coverage-check`).
Tests that are `#[ignore]`d must carry a documented reason (e.g. unreachable in Phase 1).

<!-- HANDOFF: END -->

<!-- HANDOFF: START -->

## B1.6 — Documentation Triad

Each TDD cycle GREEN phase produces three documentation artefacts alongside code and tests:

1. **Crate README** — updated with the public surface added.
2. **Rustdoc** (`//!` / `///`) — all public items in touched modules.
3. **Product docs** under `docs/` — cross-cutting behaviour (architecture, error contract).

Documentation is a hard gate: a cycle is not complete until all three locations are updated.

<!-- HANDOFF: END -->
