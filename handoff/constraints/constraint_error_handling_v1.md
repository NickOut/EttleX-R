# Constraint: Rust Common Error Handling

**Constraint ID:** constraint/error-handling/v1
**Family:** cross-cutting
**Kind:** architectural
**Scope:** workspace-wide — applies to all EttleX Rust crates
**Status:** Active
**Supersedes:** EttleX_Rust_Common_Error_Handling_Facility_FULL_Ettle_v3.md (generator Ettle)

This document is the normative constraint form of the EttleX error handling standard.
The original generator Ettle remains in `handoff/` for implementation reference.
This constraint form governs conformance checking for all vertical slices.

---

## What this constraint governs

Every piece of code introduced or modified by a vertical slice MUST conform to the
rules in this document. The code generator reads this constraint at Step 0 and flags
any scenario whose implementation would violate it before planning begins.

---

## C1 — Canonical error type

All fallible public operations MUST return `Result<T, ExError>`.

`ExError` MUST carry:
- `kind: ExErrorKind` — required, stable, matchable in tests
- Optional structured fields: `op`, `entity_id`, `source_kind`, `request_id`, `trace_id`
- Optional source error chain for diagnostics (not normative for matching)

Rules:
- Tests MUST assert `kind`, never string messages.
- Errors MUST be non-panicking for all expected invalid inputs and state transitions.
- `panic!` is reserved for provably impossible invariant violations and is treated as
  a defect if triggered by user input or expected state.
- Error `Display` rendering MUST be deterministic for the same logical inputs.

---

## C2 — Stable error taxonomy

`ExErrorKind` MUST contain the following kinds at minimum. New kinds MAY be added;
existing kinds MUST NOT change meaning once introduced.

**Structural / validation:**
- `InvalidInput` — missing required fields, invalid parameter values
- `InvalidTitle` — title is empty or whitespace-only
- `InvalidOrdinal`
- `NotFound` — entity does not exist
- `Deleted` — entity exists but was soft-deleted (legacy boolean `deleted` field)
- `AlreadyTombstoned` — entity has a `tombstoned_at` timestamp set; distinct from `Deleted`
- `ConstraintViolation`
- `IllegalReparent`
- `CycleDetected`
- `SelfReferentialLink` — reasoning_link_id set to own ettle_id
- `HasActiveDependants` — tombstone blocked by active dependants
- `MissingLinkType` — reasoning_link_id supplied without reasoning_link_type
- `MultipleParents`
- `DuplicateMapping`
- `MissingMapping`
- `AmbiguousSelection`
- `EmptyUpdate` — update command supplied with no optional fields

**Authorisation / permission:**
- `Unauthorised` — authentication missing or invalid
- `Forbidden` — authenticated but not permitted

**Concurrency:**
- `HeadMismatch` — expected_state_version does not match current state_version

**Traversal / export:**
- `TraversalBroken`
- `DeletedNodeInTraversal`
- `AmbiguousLeafSelection`
- `RefinementIntegrityViolation`
- `DeterminismViolation` — internal defensive guard only; not for normal control flow

**Integration / IO:**
- `Io`
- `Serialization`
- `Persistence`
- `ExternalService`
- `Timeout`
- `Concurrency`
- `Internal` — unexpected internal state; treated as a defect

**Note on `Deleted` vs `AlreadyTombstoned`:** `Deleted` is the legacy kind used with
the boolean `deleted` column (pre-Slice 01 schema). `AlreadyTombstoned` is the kind
used with the `tombstoned_at` nullable timestamp column (Slice 01 onwards). Both MUST
remain in the taxonomy during the transition period. Once schema cleanup removes all
`deleted` boolean columns, `Deleted` may be deprecated.

---

## C3 — Constructors and test helpers

The error facility MUST provide:
- Constructors: `ExError::new(kind).with_op("...").with_entity_id("...").with_message("...")`
- Test macros: `assert_err_kind!(result, ExErrorKind::NotFound)`
- Optional: `assert_err_field!(result, "op", "ettle_create")`

Macros MUST be deterministic and MUST NOT depend on the logging facility.

---

## C4 — Conversion rules

Internal modules MAY define local error types but MUST convert to `ExError` at the
crate public API boundary. `From<T>` implementations MUST:
- Map `kind` deterministically
- Preserve key classification fields (`op`, `entity_id`) where present
- Preserve the source error chain where meaningful
- NOT collapse semantically distinct failures (e.g. `NotFound` vs `AlreadyTombstoned`;
  `Unauthorised` vs `Forbidden`)

IO, serde, and persistence errors MUST map to `Io`, `Serialization`, and `Persistence`
respectively unless a more specific semantic kind is required by the API contract.

---

## C5 — Boundary mapping placement

`ettlex-core` and `ettlex-store` MUST return `ExError` only. They MUST NOT contain
CLI or MCP response mapping code.

`ettlex-engine` MUST return `ExError` to its callers. It MUST NOT introduce a separate
engine-level outcome type.

`ettlex-mcp` and `ettlex-cli` MUST each contain exactly one boundary mapping module
that converts `ExError` to their respective external response format. Mapping MUST NOT
be scattered across business logic.

Boundary responses MUST include:
- A stable error code derived from `kind` (e.g. `ERR_NOT_FOUND`)
- A human-readable message (non-normative)
- Correlation ID if present
- MUST NOT leak secrets, raw payloads, or internal stack traces

---

## C6 — Determinism

Any traversal, ordering, rendering, export, hashing, or stable-output code path MUST
use deterministic iteration order.

- Use `BTreeMap`/`BTreeSet` or sort before output wherever insertion order is not
  guaranteed by construction.
- Never rely on `HashMap` iteration order in any path that affects output.
- `DeterminismViolation` is a defensive guard for detecting runtime violations of this
  rule; prevention via deterministic collections is the primary requirement.

---

## C7 — Panic and unsafe convenience policy

Disallowed in non-test code without explicit annotation and justification:
- `unwrap()` / `expect()`
- `panic!()` for validation, IO, or state-transition errors
- Implicit panics via unchecked indexing when the index is input-driven

Allowed:
- Explicit invariant assertions for provably impossible states (documented as defects)
- Internally proven unreachable branches (treated as defects if encountered)

---

## C8 — Lint enforcement

The workspace root `Cargo.toml` MUST define shared lint policy under
`[workspace.lints.rust]` and `[workspace.lints.clippy]`. This is the single source
of truth. Per-crate `#![deny(...)]` blocks are permitted only for crate-specific
additions that do not weaken the workspace baseline.

Minimum required:
- `unused_must_use` — denied (prevents silent dropping of `Result`)
- clippy `unwrap_used`, `expect_used`, `panic` in non-test code — denied or forbidden

---

## Conformance check (for code generators)

Before producing any scenario implementation, verify:

- [ ] All new public functions return `Result<T, ExError>`
- [ ] All new error kinds are in `ExErrorKind` or added to it
- [ ] Tests assert `kind`, not string messages
- [ ] No `unwrap()`/`expect()` in non-test code
- [ ] No `panic!()` for input-driven errors
- [ ] Boundary mapping lives only in `ettlex-mcp` and `ettlex-cli`
- [ ] All ordered outputs use `BTreeMap`/`BTreeSet` or sort before output
- [ ] `Deleted` and `AlreadyTombstoned` are used correctly per the note in C2
