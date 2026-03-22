# Constraint: Architectural Layering

> **SUPERSEDED**
> This document has been converted to a first-class EttleX Ettle and updated to reflect the current architecture.
> Authoritative constraint: `ettle:019d15d6-9cf1-73a3-b366-9e31b2a7bdd8` — EttleX — Architectural Layering Constraint
> This file is retained as a historical artefact only. Code generators and reviewers MUST use the Ettle, not this file.

**Constraint ID:** constraint/architectural-layering/v1
**Family:** cross-cutting
**Kind:** architectural
**Scope:** workspace-wide — applies to all EttleX Rust crates
**Status:** Active

This document defines the mandatory architectural layering rules for the EttleX
Rust workspace. All vertical slices MUST conform to these rules. Violations are
structural defects, not style preferences.

---

## What this constraint governs

Every piece of code introduced or modified by a vertical slice MUST conform to the
layer responsibilities and dependency rules in this document. The code generator
reads this constraint at Step 0 and flags any scenario whose implementation would
violate it before planning begins.

---

## C1 — Layer definitions and responsibilities

The workspace is organised into strict dependency layers. Each layer has defined
responsibilities. Code that belongs in one layer MUST NOT appear in another.

### Layer 1 — Foundation: `ettlex-core-types`

Shared primitives only. No business logic, no IO, no persistence, no logging
initialisation. Contains:
- Correlation ID types: `RequestId`, `TraceId`, `RequestContext`
- Canonical schema field key constants
- `Sensitive<T>` marker and redaction helpers

MUST NOT depend on any other workspace crate.

### Layer 2 — Domain model: `ettlex-core`

Pure domain model. Record types, domain rules, command/query types, diff logic,
coverage model, traversal algorithms.

MUST NOT contain: persistence, network, logging initialisation, MCP/CLI wiring.
MUST return `ExError` from all fallible public APIs.

### Layer 3 — Persistence: `ettlex-store`

SQLite persistence via `rusqlite` (bundled). Append-only ledger, CAS, schema
migrations, seed import, snapshot persistence, profile and policy storage.

MUST NOT contain: business logic, invariant enforcement, MCP/CLI wiring.
MUST NOT call out to engine or higher layers.
MUST return `ExError` from all fallible public APIs.
MUST NOT contain boundary mapping code (CLI/MCP response formatting).

Store functions are structural: they enforce FK existence and uniqueness constraints
only. They do not enforce domain rules (e.g. "an Ettle must not be tombstoned before
updating"). Domain rule enforcement belongs in the engine.

### Layer 4 — Orchestration: `ettlex-engine`

Action dispatch, command handlers, invariant enforcement, snapshot commit pipeline.

MUST NOT contain: MCP/CLI transport concerns, JSON serialisation for external APIs,
direct user-facing response construction.
MUST call into `ettlex-store` for all persistence operations.
MUST return `ExError` from all fallible public APIs.
MUST NOT invent a separate engine-level outcome type — callers receive `ExError`.

All invariant enforcement (existence checks, tombstone checks, self-referential link
checks, dependant checks, OCC verification) MUST live here, not in the store or MCP
layers.

The engine owns: state_version increment (via `mcp_command_log`), provenance event
append, and all business rule validation.

### Layer 5 — Transport: `ettlex-mcp`, `ettlex-cli`

Thin wrappers over the engine action layer. Transport wiring only.

`ettlex-mcp` responsibilities:
- Deserialise JSON input to command types
- Schema validation (types, required fields) only — no semantic validation
- Call `apply_mcp_command` / engine query handlers
- Serialise results to JSON response
- Boundary mapping: convert `ExError` to MCP error responses (in one module)

`ettlex-cli` responsibilities:
- Parse CLI arguments to command types
- Call engine action layer directly (not via MCP)
- Boundary mapping: convert `ExError` to CLI output (in one module)

MUST NOT contain: business logic, invariant enforcement, persistence calls,
domain rule validation.

### Layer 6 — Entry points: binaries

`ettlex-mcp` (binary), `ettlex-cli` (binary), `ettlex-tauri` (binary).
Initialise the application (logging, config, DB connection) and delegate to their
respective library layer. No business logic.

### Layer 7 — Exporters: `ettlex-projection`

Read-only exporters (JSON, Markdown). Depends on `ettlex-core` only.
MUST NOT depend on `ettlex-store`, `ettlex-engine`, `ettlex-mcp`, or `ettlex-cli`.

---

## C2 — Dependency direction (mandatory)

Dependencies flow strictly downward. No upward dependencies are permitted.

```
ettlex-cli (binary)   ettlex-mcp (binary)   ettlex-tauri (binary)
        ↓                     ↓                      ↓
   ettlex-cli (lib)      ettlex-mcp (lib)       ettlex-tauri (lib)
              ↘                ↓               ↙
                      ettlex-engine
                            ↓
                      ettlex-store
                            ↓
                       ettlex-core
                            ↓
                    ettlex-core-types

ettlex-projection → ettlex-core only
```

Any `Cargo.toml` dependency that creates an upward reference (e.g. `ettlex-store`
depending on `ettlex-engine`) is a hard violation.

---

## C3 — Business logic placement

Business logic MUST live in `ettlex-engine`. The following are business logic and
MUST NOT appear in `ettlex-mcp`, `ettlex-cli`, or `ettlex-store`:

- Existence checks (does this Ettle exist?)
- Tombstone checks (is this Ettle already tombstoned?)
- Dependant checks (does this Ettle have active dependants?)
- Self-referential link checks
- Cycle detection
- OCC verification (does expected_state_version match?)
- Any validation beyond "is this field the right type?"

The following are NOT business logic and belong in `ettlex-store`:
- FK existence enforcement (SQLite FOREIGN KEY constraints)
- Uniqueness enforcement (SQLite UNIQUE constraints)
- Column type enforcement

---

## C4 — Mutation routing

All write operations MUST flow through `action:commands::apply` in `ettlex-engine`.
No crate MAY call store mutation functions directly except `ettlex-engine`.

This means:
- `ettlex-mcp` MUST call `apply_mcp_command` → engine dispatch → store
- `ettlex-cli` MUST call the engine action layer → store
- Neither MCP nor CLI MAY call `SqliteRepo` or any store mutation function directly

---

## C5 — State version and provenance

State version increment (insert into `mcp_command_log`) MUST be owned by the engine
action layer. It MUST NOT be done in MCP, CLI, or store layers.

Provenance event append (insert into `provenance_events`) MUST be owned by the engine
action layer. It MUST NOT be done in MCP, CLI, or store layers.

Every successful mutation MUST append exactly one provenance event.
Failed mutations MUST NOT append any provenance event.

---

## C6 — Replacement vs extension during vertical slices

When a vertical slice replaces existing behaviour, the replacement target MUST be
explicitly identified. Code in the slice boundary is replaced, not extended alongside
the old implementation.

After a slice is complete, the following structural invariants MUST hold for
the operations covered by that slice:

- The MCP tool handler MUST contain no invariant enforcement logic
- The engine command handler MUST contain all invariant enforcement logic
- The store function MUST contain no domain rule validation
- Boundary mapping MUST exist in exactly one module per transport layer
- No dead dispatch paths (old command handling code) MUST remain in the covered
  operations' modules

---

## C7 — Naming conventions

- Engine command handlers: `handle_{operation}` or as a match arm in
  `dispatch_mcp_command`
- Store functions: `insert_{entity}`, `get_{entity}`, `list_{entities}`,
  `update_{entity}`, `tombstone_{entity}`
- MCP tool handlers: `handle_{tool_name}` in `tools/{group}.rs`
- All public API function names MUST be consistent across layers for the same
  operation (e.g. the store function for creating an Ettle is `insert_ettle`; the
  engine command is `EttleCreate`; the MCP tool is `ettlex.apply` with tag
  `EttleCreate`)

---

## C8 — Coexistence rule during slice programme

Code outside the declared boundary of a vertical slice MUST NOT be modified, with
two exceptions: `makefile` and `handoff/slice_registry.toml`.

This rule exists because old code and new code will coexist during the slice
programme. The old code is not wrong — it will be addressed by future slices. The
coexistence rule prevents accidental scope creep and keeps pre-authorised failure
lists accurate.

---

## Conformance check (for code generators)

Before producing any scenario implementation, verify:

- [ ] No business logic in `ettlex-mcp` or `ettlex-cli` implementations
- [ ] No domain rule validation in `ettlex-store` implementations
- [ ] All invariant enforcement in `ettlex-engine`
- [ ] All mutations routed through engine action layer
- [ ] State version increment owned by engine action layer
- [ ] Provenance event append owned by engine action layer
- [ ] No upward dependencies introduced in `Cargo.toml` files
- [ ] Boundary mapping in exactly one module per transport (`ettlex-mcp`, `ettlex-cli`)
- [ ] Replacement targets explicitly identified; no dead dispatch paths left behind
- [ ] `ettlex-projection` depends only on `ettlex-core`
