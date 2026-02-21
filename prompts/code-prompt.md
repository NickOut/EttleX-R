### Always-required supporting files
- Read all attached files before proceeding

## Authority rules (NON-NEGOTIABLE)

1. **Strict TDD for behaviour work:** show RED before GREEN for each scenario/delta.


## Strict TDD (RED → GREEN → REFACTOR)

For each scenario or scenario delta:

- **RED:** write/modify tests; run tests; capture a meaningful assertion failure.
- **GREEN:** minimal code to pass; re-run; capture pass evidence.
- **REFACTOR (optional):** after GREEN; within scope; re-run and record.

Targeted test runs are allowed for iteration speed. Full acceptance gates are mandatory before completion.

## Acceptance requirement (MANDATORY)

## Triad completeness + placement (MANDATORY)

All work produced from an Ettle leaf MUST satisfy the Tests/Code/Docs triad. This is not optional and is part of acceptance.

### 1) Triad completeness (scenario-level)

For **every** Scenario / Scenario Outline / scenario-delta in the leaf Ettle's HOW (Gherkin):
- At least one corresponding automated test MUST exist.
- Production code MUST be created/changed **only** to satisfy those tests (strict TDD).
- Documentation MUST be created/updated to reflect the behavioural contract and the public surface that results.

Constraints:
- No scenario may exist without a test.
- No behavioural production code may exist without a driving scenario/test.
- All new/changed public functions, structs, traits, commands, and error types MUST be documented.

### 2) Traceability (scenario → artefacts)

You MUST produce an explicit mapping table (in the output) that links each scenario/delta to:
- Test file(s) + test name(s)
- Production module/file(s) touched
- Documentation file/section(s) updated

This mapping is part of the completion gate.

### 3) Repo structure + artefact locations (Rust workspace)

Outputs MUST conform to the Rust workspace structure below and MUST NOT invent alternative roots without explicit instruction:

- Workspace root: `ettlex/` (contains root `Cargo.toml`)
- Domain core (pure, no I/O): `ettlex/crates/ettlex-core/`
- Store/persistence boundary: `ettlex/crates/ettlex-store/`
- Projections/exporters: `ettlex/crates/ettlex-projection/`
- Application orchestration: `ettlex/crates/ettlex-engine/`
- Tool surfaces: `ettlex/crates/ettlex-mcp/`, `ettlex/crates/ettlex-cli/`, `ettlex/crates/ettlex-tauri/`
- User-facing docs root: `ettlex/docs/`

Placement rules:
- **Core domain code** goes under `ettlex/crates/ettlex-core/src/` (e.g. `model/`, `ops/`, `rules/`, `errors.rs`).
- **Unit tests** for core domain code go in `ettlex/crates/ettlex-core/src/**` as `#[cfg(test)]` modules when tight coupling is required.
- **Integration tests** go under `ettlex/crates/ettlex-core/tests/` (create this folder if absent).
- **CLI/MCP/Tauri command tests** live with the crate that owns the surface (`ettlex-cli`, `ettlex-mcp`, `ettlex-tauri`).
- **Documentation** MUST be updated in one of:
  - crate-level docs (`ettlex/crates/<crate>/README.md`),
  - rustdoc module docs (`//!` or `///`) in the touched modules,
  - product docs under `ettlex/docs/` for cross-cutting behaviour (preferred for user-facing workflows).

### 4) Triad Expectation Set (TES) 

If the leaf Ettle output includes TES/Triad obligations (even as 'basic JSON' or 'stub'), you MUST still:
- generate tests that represent the TES obligations (even if some are marked TODO only when explicitly permitted),
- generate code to satisfy the non-TODO obligations via strict TDD,
- document the TES output format and how it is derived/validated.

Do not treat TES as a placeholder excuse to skip tests or documentation.
- Respect the crate boundary constraints specified in the entry document. 
- Ensure dependencies align with the intended architectural layer.

## Acceptance gates (MANDATORY)

1. All tests run
2. Build passes without any errors or warnings
3. Documentation produced (crate-level / rustdocs / product docs)

Run the canonical Makefile targets in order (details in policy files):

1. `make lint`
2. `make test`
3. `make coverage-check` (threshold enforced by `COVERAGE_MIN` in Makefile)
4. `make coverage-html`

