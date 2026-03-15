# CLAUDE.md

This file provides guidance to Claude Code when working with code in this repository.

---

## Make Targets

Always use the Makefile targets rather than raw cargo commands.

```bash
make build            # Build entire workspace
make lint             # Run banned-pattern check + clippy + fmt check
make fmt              # Auto-format all code
make test             # Run full test suite (cargo nextest) — may include pre-authorised failures
make test-slice       # Run only slice-registered tests (see handoff/slice_registry.toml)
make coverage-check   # Run tests with coverage and enforce minimum threshold (COVERAGE_MIN)
make coverage-html    # Generate HTML coverage report in coverage/html/
make doc              # Generate rustdocs (target: aarch64-apple-darwin)
make clean            # Clean build artifacts
```

The `COVERAGE_MIN` threshold in the Makefile MUST NOT be modified to pass a failing gate.
Coverage is assessed against `make test-slice` scope during the slice programme, not the full suite.

Binaries are run with `cargo run -p <crate-name>`:
- `ettlex-cli` — command line interface
- `ettlex-mcp` — MCP transport server
- `ettlex-tauri` — desktop app backend (Tauri)

---

## Architecture

EttleX is a Rust workspace implementing a semantic reasoning and governance system. The codebase
is structured in strict dependency layers. No upward dependencies are permitted.

### Crate Dependency Layers (top → bottom)

```
ettlex-cli  ettlex-mcp  ettlex-tauri       ← binary entry points
                ↓
          ettlex-engine                    ← orchestration layer: action dispatch,
                ↓                            snapshot commit pipeline, command handlers
          ettlex-store                     ← persistence layer: SQLite via rusqlite,
                ↓                            append-only ledger, CAS, seed import,
          ettlex-core                        schema migrations
                ↓
        ettlex-logging                     ← logging facility: single init point,
                ↓                            canonical event schema, test capture
        ettlex-errors                      ← error facility: ExError, ExErrorKind,
                ↓                            canonical taxonomy, test macros
       ettlex-core-types                   ← shared primitives: correlation IDs,
                                             schema constants, Sensitive<T> marker

ettlex-projection → ettlex-core only
```

`ettlex-core-types` is the foundational crate. It MUST NOT depend on any other workspace crate.
`ettlex-errors` MUST NOT depend on `ettlex-logging` or any feature crate.
`ettlex-logging` MUST NOT depend on `ettlex-core` or higher.

**Note:** `ettlex-errors` and `ettlex-logging` are introduced by Slice 00. Before Slice 00
completes, `ExError`/`ExErrorKind` live in `ettlex-core/src/errors.rs` and `logging_facility`
lives in `ettlex-core/src/logging_facility/`. These are temporary positions being corrected
by the slice programme.

### Crate Responsibilities

**ettlex-core-types**
Shared primitives only. Contains:
- Correlation ID types: `RequestId`, `TraceId`, `RequestContext`
- Schema field key constants
- `Sensitive<T>` marker and redaction helpers
- No runtime initialisation, no IO, no logging dependency

**ettlex-errors** *(introduced by Slice 00)*
Canonical error facility. Contains:
- `ExErrorKind` enum — stable, matchable error taxonomy
- `ExError` struct with builder methods
- `assert_err_kind!` and `assert_err_field!` test macros
- `From<T>` conversions for common error sources
- No dependency on `ettlex-core` or higher

**ettlex-logging** *(introduced by Slice 00)*
Canonical logging facility. Contains:
- `init(profile)` — single initialisation point (Development / Production)
- `log_op_start!`, `log_op_end!`, `log_op_error!` macros
- `TestCapture` / `init_test_capture` — deterministic test capture mode
- No dependency on `ettlex-core` or higher

**ettlex-core**
Pure domain model. Contains record types, domain rules, command/query types, diff logic,
coverage model, traversal algorithms.

MUST NOT contain: persistence, network, logging initialisation, MCP/CLI wiring.
MUST return `ExError` from all fallible public APIs.

**ettlex-store**
Persistence layer. SQLite via `rusqlite` (bundled). Append-only ledger, CAS, schema
migrations (numbered SQL files in `crates/ettlex-store/src/migrations/`), seed import,
snapshot persistence, profile and policy storage. No engine logic here.

MUST NOT contain: business logic, invariant enforcement, MCP/CLI wiring, boundary mapping.
MUST return `ExError` from all fallible public APIs.

**ettlex-engine**
Orchestration layer. Action dispatch (`commands/`), snapshot commit pipeline (`snapshot/`),
command handlers for all write operations. Calls into `ettlex-store`. Contains ALL invariant
enforcement (existence checks, tombstone checks, dependant checks, OCC verification).

MUST NOT contain: MCP/CLI transport concerns, JSON serialisation for external APIs.
MUST return `ExError` from all fallible public APIs.
MUST NOT invent a separate engine-level outcome type.

The engine owns: state_version increment (via `mcp_command_log`), provenance event append,
and all business rule validation.

**ettlex-mcp**
MCP transport layer. Thin wrappers over engine action layer. Schema validation only —
no business logic, no invariant enforcement. Boundary mapping (ExError → MCP error) in
one module only.

**ettlex-cli**
Command line interface. Thin wrappers over action layer. No business logic. Boundary
mapping (ExError → CLI output) in one module only.

**ettlex-projection**
Read-only exporters. Depends on `ettlex-core` only. JSON, Markdown output formats.

**ettlex-tauri**
Desktop app backend. Tauri command bridge. State management in `state.rs`.

---

## Key Conventions

### Error Handling

All public APIs return `Result<T, ExError>`. See:
`handoff/constraints/constraint_error_handling_v1.md`
`handoff/EttleX_Rust_Common_Error_Handling_Facility_FULL_Ettle_v3.md`

- `ExErrorKind` is the stable, matchable error taxonomy.
- Tests MUST assert `kind`, not string messages.
- `unwrap()` and `expect()` are banned in non-test code (enforced by lint).
- Boundary mapping (ExError → external response) lives only in `ettlex-cli` and `ettlex-mcp`.
- `Deleted` (legacy boolean `deleted` column) and `AlreadyTombstoned` (`tombstoned_at` column)
  are distinct error kinds. Use the correct one for the schema in scope.

### Logging

Single initialisation point via the logging facility. See:
`handoff/constraints/constraint_logging_v1.md`
`handoff/EttleX_Logging_Facility_Rust_FULL_Ettle_v3.md`

- No crate may call `tracing_subscriber::init()` directly.
- `println!` and `eprintln!` are banned in non-test code (enforced by `scripts/check_banned_patterns.sh`).
- Canonical event schema: `component`, `op`, `event`, `duration_ms`, `request_id`, `err.kind`.
- Boundary entrypoints (MCP/CLI) own lifecycle `start`/`end` events. Inner layers MUST NOT duplicate them.
- New code MUST NOT use `ep_id` or `ep_ordinal` log fields — these are legacy EP model fields.

### Persistence

- SQLite via `rusqlite` with the `bundled` feature (SQLite 3.43.x).
- Schema managed via numbered migration files in `crates/ettlex-store/src/migrations/`.
- All mutations go through `action:commands::apply`. No direct store writes from CLI or MCP.
- State version is a global monotonic OCC counter incremented on every successful mutation.
- Provenance events are appended for every successful mutation and never deleted.

### Determinism

All traversal, ordering, rendering, and export code paths MUST produce deterministic output.
Use `BTreeMap`/`BTreeSet` or sort before output wherever insertion order is not guaranteed.
Non-deterministic iteration from `HashMap` in ordered output paths is a defect.

### Workspace Dependencies

Common dependencies are declared in the root `Cargo.toml` under `[workspace.dependencies]`
and inherited by crates with `dep.workspace = true`. Do not re-declare versions in crate
`Cargo.toml` files.

### Lint Policy

Workspace lint policy is defined in root `Cargo.toml` under `[workspace.lints.*]`.
Per-crate `#![deny(...)]` blocks are allowed only for crate-specific additions that do
not weaken the workspace baseline.

---

## Vertical Slice Implementation

The codebase is being restructured via vertical slices. Each slice covers all layers
(MCP → Engine → Store) for a defined set of behaviour. Existing code outside a slice
boundary is left in place until a future slice addresses it.

### Slice order

- **Slice 00** — Infrastructure: extract `ettlex-errors` and `ettlex-logging` crates,
  retire `EttleXError`. MUST be completed before any feature slice.
- **Slice 01** — Ettle CRUD (Store → Engine/Action → MCP). Requires Slice 00.

### Slice tooling

Slice scope is tracked in `handoff/slice_registry.toml`. The `make test-slice` target
runs only tests registered in that file. The `make test` target runs the full suite
and may include pre-authorised failures from superseded code.

The code generator prompt for vertical slices is at:
`prompts/code_generator_prompt_vertical_slice_v1.1.md`

Constraints that inform Ettle authoring (not read by code generators) are at:
`handoff/constraints/constraint_error_handling_v1.md`
`handoff/constraints/constraint_logging_v1.md`
`handoff/constraints/constraint_architectural_layering_v1.md`

---

## Repository Layout

```
crates/                        Rust workspace crates
handoff/                       Ettle specs, seed files, completion reports
  completed/                   Completed slice completion reports
  constraints/                 Constraint documents (govern Ettle authoring)
    constraint_error_handling_v1.md
    constraint_logging_v1.md
    constraint_architectural_layering_v1.md
  slice_registry.toml          Cumulative slice test registry (source of truth for test-slice)
  schema_cleanup_notes.md      Tracking of dead schema columns pending cleanup migration
  EttleX_Logging_Facility_Rust_FULL_Ettle_v3.md
  EttleX_Rust_Common_Error_Handling_Facility_FULL_Ettle_v3.md
policies/                      Policy documents (referenced by policy_ref in snapshots)
prompts/                       Code generator and authoring prompts
rendered/                      CLI render bundle outputs
scripts/                       CI helper scripts (check_banned_patterns.sh etc.)
docs/                          Product documentation
coverage/                      Coverage reports (gitignored outputs)
makefile                       Canonical build targets — always use these
```
