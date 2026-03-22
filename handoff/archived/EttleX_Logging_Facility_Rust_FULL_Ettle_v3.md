# Ettle (Design Markdown) — EttleX Logging Facility (Rust)

Product: EttleX  
Purpose: Implement and enforce a single EttleX logging facility (structured events + spans) used uniformly across all Rust crates and across all implementation stages (authoring/CI/runtime/TES), with canonical schema, redaction, deterministic test capture, and correlation propagation.  
Tier: CORE (Cross-cutting facility)  
Status: Generator-ready (leaf implementable)  
Created: 2026-02-21  
Ettle-ID: ettle/facility-logging-ettlex  
EPs: EP0 only (single-leaf implementable)  

## Reading guide (for linear code generators)

This Ettle is intentionally split into two parts:

A) Generator instructions (normative): what MUST be implemented.  
B) TES acceptance criteria (verification): how conformance is later proven. The generator should implement what is required, but should not treat this section as runtime behaviour.

---

## Architectural position and dependency rules

This facility MUST depend on the Rust Common Error Handling Facility (`error_facility`) and on `ettlex_core_types`.

Dependency direction (normative):

- `ettlex_core_types` MUST exist (non-optional) and contains:
  - schema constants (field keys; op/event identifiers),
  - correlation ID types and `RequestContext`,
  - `Sensitive<T>` marker and redaction helpers that do not depend on logging runtime.
- `logging_facility` MUST depend on `error_facility` (to extract `err.kind` and `err.code` from `ExError`) and on `ettlex_core_types`.
- `error_facility` MUST NOT depend on `logging_facility`.

Non-goals (explicit):
- This facility MUST NOT redefine error taxonomy.
- This facility MUST NOT attempt to “fix” error semantics; it only records them.
- This facility MUST NOT allow per-crate alternative initialisation.

---

# EP0 — Structured Logging Spine + Canonical Schema + Boundary Ownership + Correlation Propagation + Enforcement

## WHY (purpose / rationale)

EttleX relies on deterministic semantics and on being able to inspect/dogfood its behaviour. Logging is a core visibility channel. Without a single enforced facility:
- implementations drift per crate/module,
- correlation across multi-crate operations becomes unreliable,
- sensitive data leakage risk increases,
- and facility-level verification becomes impossible.

The logging approach must be a facility: centralised, unavoidable, and test-verifiable.

## WHAT (normative conditions / features)

### 1. Single logging mechanism and single initialisation point

- The Rust codebase MUST use exactly one structured logging ecosystem and MUST initialise it in exactly one place: the “logging spine”.
- No crate/module may initialise logging independently.

The logging spine MUST provide:
- init/config API (profile-based),
- canonical field keys and event names (from `ettlex_core_types`),
- redaction helpers (from `ettlex_core_types::Sensitive`),
- correlation context helpers (RequestContext + span attachment),
- test harness helpers for capturing emitted events deterministically.

Rules:
- A crate MUST NOT call subscriber/logger initialisation directly (e.g. `tracing_subscriber::init()`); it MUST call the facility’s `init()`.
- The facility MUST support both development and production profiles; production MUST be structured/machine-readable.

### 2. Canonical EttleX event schema (stable fields)

All emitted events MUST be structured and MUST use canonical keys.

Required fields where applicable:
- `component`
- `op`
- `event`
- `duration_ms`
- `request_id` / `trace_id` / `span_id`
- `ettle_id`, `ep_id`, `ep_ordinal`
- `rt_len`, `ept_len`
- `err.kind`, `err.code` (on errors; sourced from `ExError`)

Rules:
- Field names snake_case.
- Sensitive values MUST NOT be logged. Only identifiers/counts/classifications.
- When logging errors, `err.kind` MUST always be present; `err.code` MUST be present if the error has a mapped code.
- Start/end pairs MUST share correlation identifiers.

### 3. Boundary ownership for start/end logging (multi-crate operations)

A key source of drift is duplicate lifecycle logging across layers. This facility defines ownership rules:

Normative ownership rule:
- The top-level boundary function (the “entrypoint” into an externally meaningful operation) MUST own lifecycle start/end (or end_error) events.
- Inner layers (engine/store/core) MUST NOT emit additional lifecycle start/end pairs for the same op.

Allowed inner-layer logging:
- Inner layers MAY create child spans and DEBUG/TRACE events for internal steps.
- Inner layers MAY emit WARN/ERROR events for significant internal conditions, but MUST NOT duplicate the boundary start/end pair for the same `op`.

Generator guidance:
- The generator SHOULD wrap boundary entrypoints in helpers that:
  - create a root span for the operation,
  - emit `start` event once,
  - execute the body under that span,
  - emit `end` or `end_error` once with duration and (if error) err.kind/err.code.

### 4. Correlation propagation across async boundaries

Correlation types and RequestContext location (normative):
- `RequestId`, `TraceId`, and `RequestContext` MUST live in `ettlex_core_types` (non-optional).

Propagation rules:
- On entry to an externally meaningful operation, a `RequestContext` MUST be created or retrieved and attached to the root span.
- Async boundaries MUST preserve correlation using span instrumentation (preferred) or explicit parameter threading (fallback).

Preferred mechanism (normative if using tracing):
- Use a root span that carries `request_id` and (if available) `trace_id`.
- Any spawned async tasks that are part of the same request MUST be instrumented with the current span so correlation is preserved.
- Logging helpers MUST retrieve correlation from the current span context (not from ad-hoc globals).

Fallback rule:
- If a code path does not run under spans (exceptional), the `RequestContext` MUST be threaded explicitly as a parameter to the functions that need to log.

### 5. Redaction and Sensitive markers

To prevent ad-hoc redaction, the facility MUST provide a canonical mechanism:

- `ettlex_core_types` MUST define `Sensitive<T>` newtype (or equivalent) and redaction helpers.
- Logging macros MUST:
  - refuse to log `Sensitive<T>` raw values,
  - log either a redacted placeholder or a hashed/tokenised representation if explicitly requested by policy.

Rules:
- Generators MUST wrap any potentially sensitive values in `Sensitive<T>` (or an equivalent marker) at the earliest boundary where the data enters the system.
- Logs MUST never include raw secrets, credentials, or raw intent content fields.

### 6. Severity and eventing rules

- INFO: lifecycle + boundary milestones (start/end) for externally meaningful operations.
- WARN: degraded behaviour, retries, policy avoidance, suspicious input, fallback paths.
- ERROR: operation failed (paired with err.kind).
- DEBUG/TRACE: internal details; disabled by default in production.

Start/end pairing rule (normative):
- For any externally meaningful operation, there MUST be a `start` and an `end` event (or `end_error`) with consistent correlation identifiers.

### 7. Determinism and testability

- Log output MUST be capturable in tests and asserted on by event name + canonical fields.
- Tests MUST NOT assert on non-deterministic fields (timestamps, thread ids) except through deterministic normalisation provided by the facility.
- The logging spine MUST provide a test capture mode that:
  - collects events in-memory in emission order,
  - exposes a stable representation for assertions,
  - omits timestamps from assertions by default; normalisation to a fixed value is available but non-default.

### 8. Enforcement across all implementation stages

Conformance MUST be enforceable by:
- authoring-time checks (banned patterns: `println!/eprintln!`, ad-hoc log init outside spine),
- CI checks (static + tests verifying boundary ownership and schema),
- runtime config (profile-based sinks and minimum level),
- TES obligations for realised leaves.

## HOW (method / process, including Gherkin scenarios)

### Generator action list (do these in order)

1. Ensure `ettlex_core_types` exists and includes schema constants, correlation IDs, `RequestContext`, `Sensitive<T>`.
2. Create `logging_facility` crate depending on `ettlex_core_types` and `error_facility`.
3. Implement `init(profile)` as the only logging initialisation path.
4. Implement canonical macros/wrappers:
   - `log_op_start!`
   - `log_op_end!`
   - `log_op_error!` (extracts from `ExError`)
   - optional `with_request_context!(ctx, || ...)` helper
5. Implement root-span wrapper for boundary entrypoints enforcing single start/end ownership.
6. Implement test capture mode (in-memory collector + normalisation).
7. Add CI checks denying ad-hoc init and `println!/eprintln!` outside tests.
8. Add integration tests asserting:
   - one start/end pair per boundary op,
   - err.kind propagation on failure,
   - correlation propagation across async spawn.

### Implementation approach (narrative)

1. Create `logging_facility` crate/module:
   - `init(profile)`,
   - canonical field/event constants,
   - redaction utilities,
   - test capture subscriber/export.
2. Replace all direct logging initialisation with calls to the spine.
3. Provide boundary wrappers enforcing ownership and correlation propagation.
4. Add CI checks and integration tests.

---

## TES acceptance criteria (verification obligations)

### Gherkin scenarios (normative acceptance tests)

Feature: Single initialisation and structured schema

Scenario: Ad-hoc initialisation is rejected  
Given a crate adds independent logger initialisation  
When CI conformance checks run  
Then the build fails indicating only the logging spine may initialise logging  

Scenario: println! is rejected outside tests  
Given a module uses println! in non-test code  
When CI conformance checks run  
Then the build fails with a banned pattern diagnostic  

Feature: Boundary ownership across crates

Scenario: Only boundary emits lifecycle start/end for a multi-crate op  
Given an operation enters via ettlex-cli and calls engine then store  
When the operation runs successfully  
Then logs contain exactly one start and one end event for op="..." at INFO  
And inner layers do not emit duplicate start/end pairs for that op  

Feature: Error integration

Scenario: render_leaf_bundle emits error event with err.kind  
Given a leaf bundle render fails due to ambiguous selection  
When render_leaf_bundle is invoked without selecting a leaf EP  
Then logs contain event end_error with op="render_leaf_bundle"  
And logs contain err.kind="AmbiguousSelection"  
And the log does not contain raw intent content  

Feature: Correlation propagation

Scenario: request_id flows across async spawn  
Given a boundary operation creates RequestContext with request_id  
When it spawns an async task within the operation  
Then logs from the spawned task include the same request_id  

### Evidence required at snapshot/TES (minimum)

A realised leaf MUST link evidence showing:
1. `logging_facility` is the only init path.
2. CI rejects ad-hoc init and `println!`/`eprintln!` in non-test code.
3. Integration tests assert boundary ownership (single start/end per op).
4. Integration tests assert `err.kind` appears and matches the `ExErrorKind` produced by `error_facility`.
5. Integration tests assert correlation propagation across at least one async spawn.

---
