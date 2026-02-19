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

## Acceptance gates (MANDATORY)

Run the canonical Makefile targets in order (details in policy files):

1. `make lint`
2. `make test`
3. `make coverage-check` (threshold enforced by `COVERAGE_MIN` in Makefile)
4. 'make coverage-html' (if 3.: make coverage-check passes) 



