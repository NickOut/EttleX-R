# Ettle (Design Markdown) — Rust Common Error Handling Facility

Product: EttleX  
Purpose: Establish a canonical, enforceable Rust error-handling standard used across all crates/modules, with deterministic typed errors, stable classification, and consistent conversion/mapping rules for boundaries.  
Tier: CORE (Cross-cutting facility)  
Status: Generator-ready (leaf implementable)  
Created: 2026-02-21  
Ettle-ID: ettle/facility-rust-common-error-handling  
EPs: EP0 only (single-leaf implementable)  

## Reading guide (for linear code generators)

This Ettle is intentionally split into two parts:

A) Generator instructions (normative): what MUST be implemented.  
B) TES acceptance criteria (verification): how conformance is later proven. The generator should implement what is required, but should not treat this section as runtime behaviour.

---

## Architectural position and dependency rules

This facility is foundational and MUST NOT depend on the logging facility.

Dependency direction (normative):

- `ettlex_core_types` MUST exist (non-optional) and contains only minimal shared types/constants (see below).
- `error_facility` MUST depend only on `ettlex_core_types` and other “core” crates as needed, but MUST NOT depend on `logging_facility`.
- `logging_facility` MUST depend on `error_facility` and `ettlex_core_types`.

Required crate: `ettlex_core_types` (non-optional)
- Contains only:
  - canonical schema field keys and operation/event identifiers as `'static` constants (no runtime init),
  - correlation ID types: `RequestId`, `TraceId` (and optionally `SpanId` as a string wrapper if needed),
  - request context type: `RequestContext` (see correlation rules),
  - sensitivity marker types: `Sensitive<T>` (and helpers that do not depend on logging runtime).
- MUST NOT contain:
  - subscriber initialisation
  - formatting/sinks
  - IO or network dependencies

Non-goals (explicit):
- This facility MUST NOT emit logs.
- This facility MUST NOT implement HTTP/tool response formatting itself.
- This facility MUST NOT contain runtime subscriber initialisation.

---

# EP0 — Canonical Error Model + Conversions + Boundary Mapping + Enforcement

## WHY (purpose / rationale)

EttleX depends on deterministic semantics and stable verification. If error behaviour is ad-hoc, then tests become brittle, logs become noisy, boundary behaviour drifts, and traversal/export/tooling can no longer rely on stable error kinds.

A shared error facility ensures:
- stable typed failure modes across the codebase,
- consistent boundary mapping (tool surface / IO / persistence),
- predictable logging/error coupling (without introducing hard dependency cycles),
- deterministic outputs (including human-facing render functions),
- enforceable quality gates across authoring, CI, runtime, and snapshot/TES.

## WHAT (normative conditions / features)

### 1. Canonical error representation

All fallible operations in EttleX Rust MUST return a single canonical `Result<T, ExError>` (or an equivalent wrapper that is trivially and losslessly convertible to `ExError`).

`ExError` MUST be a typed error enum or structured type with:
- `kind: ExErrorKind` (required; stable and matchable in tests),
- optional structured fields for classification and correlation (examples: `code`, `op`, `entity_id`, `ep_id`, `ordinal`, `source_kind`, `request_id`, `trace_id`),
- optional source error chain preserved for diagnostics (non-normative for matching).

Rules:
- Tests MUST assert `kind` (and selected structured fields where relevant), not string messages.
- Errors MUST be non-panicking for expected invalid inputs/state transitions.
- Panic is reserved for “impossible” invariants and MUST be treated as a defect if triggered by user input or expected state.
- Error rendering (`Display` and any “public message” rendering) MUST be deterministic for the same logical inputs.

### 2. Stable taxonomy and naming

`ExErrorKind` MUST contain a stable set of kinds covering, at minimum:

A) Structural / validation:
- `InvalidInput`
- `InvalidTitle`
- `InvalidOrdinal`
- `NotFound`
- `Deleted`
- `ConstraintViolation`
- `IllegalReparent`
- `CycleDetected`
- `MultipleParents`
- `DuplicateMapping`
- `MissingMapping`
- `AmbiguousSelection`

B) Authorisation / permission:
- `Unauthorised` (authentication missing/invalid)
- `Forbidden` (authenticated but not permitted)

C) Traversal/export:
- `TraversalBroken`
- `DeletedNodeInTraversal`
- `AmbiguousLeafSelection`
- `DeterminismViolation` (internal-only guard; see §2.4)

D) Integration/IO:
- `Io`
- `Serialization`
- `Persistence`
- `ExternalService`
- `Timeout`
- `Concurrency` (locks/poisoning/cancellation)

Rules:
- Kinds MUST be named for semantic meaning, not for crate/function names.
- Kinds MUST remain stable once introduced; new kinds may be added, but existing kinds MUST NOT change meaning.
- Each kind MUST have a stable external code mapping (see “Boundary mapping rules”).

### 2.4 Determinism rules and DeterminismViolation semantics

Determinism is a semantic requirement for EttleX traversal/render/export outputs.

Normative determinism rule:
- Any traversal, ordering, rendering, export, hashing, or “stable output” code path MUST use deterministic iteration order.
- Generators MUST NOT rely on non-deterministic iteration order from standard hash maps/sets or unordered collections when producing ordered outputs.

Implementation rules (normative):
- If a map/set is used in a path that affects ordering, the generator MUST choose a deterministic collection (e.g. BTreeMap/BTreeSet) OR MUST sort keys/values before iteration.
- If a Vec is used to accumulate items, the generator MUST sort it by a stable key before output if insertion order is not already guaranteed stable by construction.

`DeterminismViolation` meaning:
- This kind is reserved for defensive guards that detect a breach of the determinism rule at runtime (e.g. inconsistent ordering across repeated computations within the same run).
- Generators SHOULD NOT emit `DeterminismViolation` in normal control flow.
- Generators MAY include defensive detection and return `DeterminismViolation` if detection triggers, but prevention (deterministic collections/sorting) is the primary requirement.

Concrete example:
- If EPT rendering collects children into a `HashMap` and then iterates it to output lines, ordering may vary per run. Generator MUST instead use `BTreeMap` or sort the child keys before output. The defensive guard, if included, returns `DeterminismViolation` only if it detects inconsistent results despite these measures (e.g. due to a bug).

### 3. Canonical constructors and macros

The facility MUST provide:
- canonical constructors (e.g. `ExError::new(kind).with_field(...)`),
- convenience macros for:
  - producing typed errors (e.g. `ex_err!(Kind::NotFound, op="read_ep", id=...)`),
  - asserting in tests (e.g. `assert_err_kind!(res, Kind::MissingMapping)`),
  - optionally asserting structured fields (e.g. `assert_err_field!(res, "op", "create_ettle")`).

Rules:
- Macros MUST produce deterministic results and MUST NOT depend on the logging facility.
- Constructors MUST not eagerly format strings that would become non-deterministic or environment-dependent.

### 4. Conversion rules (From/Into)

- Internal modules MAY define local error enums/structs, but MUST convert to `ExError` at module boundary (crate public API boundary at minimum).
- `From<T>` conversions into `ExError` MUST preserve:
  - `kind` (mapped deterministically),
  - key classification fields (e.g. `op`, `entity_id`, `ep_id`, `ordinal` if present),
  - the original `source` where meaningful.

Rules:
- A conversion MUST NOT collapse semantically distinct failures into a single kind if that would prevent stable tests (e.g. `NotFound` vs `Deleted`; `Unauthorised` vs `Forbidden`).
- IO/serde/persistence failures MUST map to kinds `Io`/`Serialization`/`Persistence` respectively, unless a more specific semantic kind is mandated by the calling API contract.

### 5. Boundary mapping rules (with crate placement)

At external boundaries (tool surface, CLI, persistence boundary, network boundary), the system MUST define deterministic mappings from `ExError` to boundary responses.

Boundary response MUST include:
- a stable error code derived from `kind` (e.g. `ERR_NOT_FOUND`, `ERR_FORBIDDEN`),
- a human-readable message (non-normative),
- correlation id if present (`request_id` / `trace_id`),
- MUST NOT leak secrets, raw payloads, or internal stack traces by default.

Placement rule (normative, aligned to spine layout):
- `ettlex-core` and `ettlex-store` MUST return `ExError` and MUST NOT contain CLI/tool mapping code.
- `ettlex-engine` MUST return `ExError` to its callers; it MUST NOT invent an “engine-level outcome” type that diverges from `ExError` (avoid ambiguity and drift).
- `ettlex-cli` (and any future tool/MCP surface crate) MUST contain the boundary mapping module(s) that convert `ExError` into user/tool-visible responses.

Mapping MUST be implemented in one place per boundary adapter, not scattered across business logic.

### 6. Panic and “unsafe convenience” policy

Disallowed in non-test code (unless explicitly annotated and justified):
- `unwrap()` / `expect()`
- `panic!()` for validation, IO, or state-transition errors
- implicit panics via indexing without bounds checks when input-driven

Allowed:
- explicit invariant checks for “impossible” states, clearly marked as defects
- internally proven unreachable branches (still treated as defects if encountered)

Enforcement MUST exist (see “Enforcement at all stages”).

### 7. Enforcement at all implementation stages (including lint placement)

Conformance MUST be enforceable at:
- authoring: lint/static checks for banned patterns (untyped public errors; `panic!` in non-test code except explicitly annotated; `unwrap()/expect()` outside tests),
- CI: unit tests verifying conversions and boundary mapping (including “NotFound vs Deleted” separation),
- runtime: error-to-response mapping stable and redact-safe,
- snapshot/TES: realised leaves include evidence that operations return typed errors and tests cover expected failure modes.

Lint configuration placement (normative):
- The workspace root `Cargo.toml` MUST define shared lint policy using `[workspace.lints.rust]` and `[workspace.lints.clippy]` (single source of truth).
- Crates MUST NOT each define divergent lint policies; per-crate `#![deny(...)]` blocks are allowed only for crate-specific additions that do not weaken the workspace baseline.
- If a `.cargo/config.toml` is required to ensure consistent lint application in CI, it MUST be added at workspace root (not per crate).

Minimum lint requirements:
- `unused_must_use` MUST be denied or treated as CI-failing (prevents silent dropping of `Result`).
- clippy: deny/forbid `unwrap_used`, `expect_used`, and `panic` in non-test code (or equivalent policy).
- Optional but recommended: clippy `result_large_err` and `missing_errors_doc` as warnings.

Note on `#[must_use]`:
- The primary enforcement is `unused_must_use`. Generators do not need to add additional `#[must_use]` attributes to `Result`-returning functions beyond the workspace lint policy.

## HOW (method / process, including Gherkin scenarios)

### Generator action list (do these in order)

1. Create `ettlex_core_types` crate (non-optional) with correlation ID types, `RequestContext`, schema constants, `Sensitive<T>` marker.
2. Create `error_facility` crate, depending only on `ettlex_core_types` (and std).
3. Implement `ExErrorKind` and `ExError` plus constructors/builders.
4. Implement deterministic mappings from common sources (IO/serde/persistence) to kinds.
5. Implement test assertion helpers/macros.
6. Ensure all public APIs return `Result<T, ExError>`.
7. Add workspace root lint policy in root `Cargo.toml` under `[workspace.lints.*]` and CI enforcement.
8. Add representative unit tests (kinds and mappings).
9. Ensure boundary mapping exists only in `ettlex-cli` / tool surface crate(s).

### Implementation approach (narrative)

1. Create a dedicated crate/module: `error_facility`.
2. Implement `ExErrorKind` and `ExError` with structured fields and optional source.
3. Add canonical constructors/macros for creation + test assertions.
4. Define conversion/mapping tables for common sources (IO/serde/persistence) and for common domain failures.
5. Integrate into existing operation layers by replacing ad-hoc errors at public boundaries.
6. Add lints/CI checks and boundary mapping tests.
7. Provide a single “boundary mapping” module per adapter surface (tool/API/CLI), which consumes `ExError` and outputs the external response type.

---

## TES acceptance criteria (verification obligations)

### Gherkin scenarios (normative acceptance tests)

Feature: Typed deterministic error kinds

Scenario: NotFound is test-verifiable without string matching  
Given a new in-memory store  
When I read an unknown Ettle ID  
Then the operation fails with kind NotFound  
And tests assert kind NotFound without matching error message text  

Scenario: Deleted is not silently treated as NotFound  
Given an entity exists and is tombstoned  
When I attempt an update  
Then the operation fails with kind Deleted  
And boundary mapping returns error code ERR_DELETED  

Scenario: Unauthorised and Forbidden are distinct  
Given an operation requires authentication  
When I call it without authentication  
Then the operation fails with kind Unauthorised  
And boundary mapping returns ERR_UNAUTHORISED  
Given I call it with authentication but without permission  
Then the operation fails with kind Forbidden  
And boundary mapping returns ERR_FORBIDDEN  

Scenario: Invalid input produces InvalidInput with structured fields  
Given a function requires a non-empty title  
When I pass an empty string  
Then the operation fails with kind InvalidTitle  
And the error includes field op="create_ettle"  

Feature: Conversion and boundary mapping

Scenario: IO error maps deterministically  
Given an IO read fails with OS error  
When converted into ExError  
Then kind is Io  
And boundary mapping returns error code ERR_IO  
And the source error is retained for diagnostics  
But the response does not expose internal stack traces  

Scenario: Illegal unwrap is rejected by stage enforcement  
Given a non-test module contains unwrap()  
When CI conformance checks run  
Then the build fails with a diagnostic pointing to the banned pattern  

Feature: Deterministic outputs

Scenario: Non-deterministic collection usage is prevented  
Given traversal collects items that affect output ordering  
When the generator implements the traversal  
Then it MUST use deterministic collections or sort before output  
And repeated runs yield identical output for identical input  

### Evidence required at snapshot/TES (minimum)

A realised leaf MUST link evidence showing:
1. `ettlex_core_types` exists and is used (correlation IDs + Sensitive marker).
2. `error_facility` exists and is used by public APIs.
3. Representative unit tests assert `ExErrorKind` for at least:
   - one validation failure,
   - one structural failure (`NotFound` or `Deleted`),
   - one authorisation failure (`Unauthorised` or `Forbidden`),
   - one traversal/export failure (where applicable),
   - one integration failure (IO or Serialization).
4. Boundary mapping tests demonstrate stable external codes for at least `NotFound`, `Deleted`, `InvalidInput`, `Unauthorised`, `Forbidden`, and one integration kind.
5. CI enforcement exists for banned patterns (`unwrap/expect`, `panic` misuse, untyped public errors) and `unused_must_use`.

---
