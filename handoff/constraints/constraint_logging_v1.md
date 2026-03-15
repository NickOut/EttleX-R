# Constraint: Rust Logging Facility

**Constraint ID:** constraint/logging/v1
**Family:** cross-cutting
**Kind:** architectural
**Scope:** workspace-wide — applies to all EttleX Rust crates
**Status:** Active
**Supersedes:** EttleX_Logging_Facility_Rust_FULL_Ettle_v3.md (generator Ettle)

This document is the normative constraint form of the EttleX logging standard.
The original generator Ettle remains in `handoff/` for implementation reference.
This constraint form governs conformance checking for all vertical slices.

---

## What this constraint governs

Every piece of code introduced or modified by a vertical slice MUST conform to the
rules in this document. The code generator reads this constraint at Step 0 and flags
any scenario whose implementation would violate it before planning begins.

---

## C1 — Single initialisation point

The workspace MUST use exactly one structured logging ecosystem, initialised in
exactly one place: the logging spine (`logging_facility`).

- No crate or module MAY call subscriber/logger initialisation directly
  (e.g. `tracing_subscriber::init()`). All callers MUST use `logging_facility::init(profile)`.
- The facility MUST support development (human-readable) and production
  (structured/machine-readable) profiles.

Banned patterns (enforced by `scripts/check_banned_patterns.sh` and CI):
- `println!` / `eprintln!` in non-test code
- Any direct subscriber initialisation outside `logging_facility`

---

## C2 — Canonical event schema

All emitted log events MUST be structured and MUST use canonical field keys from
`ettlex_core_types::schema`.

Required fields where applicable:

| Field | When required |
|---|---|
| `component` | Always |
| `op` | All externally meaningful operations |
| `event` | All events (`start`, `end`, `end_error`) |
| `duration_ms` | On `end` and `end_error` events |
| `request_id` | When RequestContext is available |
| `trace_id` | When propagated from caller |
| `span_id` | When using tracing spans |
| `ettle_id` | When operating on an Ettle |
| `err.kind` | On all error events (sourced from ExErrorKind) |
| `err.code` | On error events where a stable code is mapped |

**Note on `ep_id` / `ep_ordinal`:** These fields exist in the schema constants for
legacy compatibility. New code introduced by vertical slices MUST NOT use `ep_id` or
`ep_ordinal` — they reference the EP model being retired. New code MUST use `ettle_id`
only. Existing logging that references `ep_id` is pre-authorised legacy output;
do not propagate it into new code paths.

Field naming rules:
- All field names MUST be `snake_case`
- Sensitive values MUST NOT be logged — use `Sensitive<T>` markers (see C4)
- `err.kind` MUST always be present when logging an error event

---

## C3 — Boundary ownership for lifecycle events

The entry-point boundary function (the topmost function handling an externally
meaningful operation) MUST own the lifecycle `start` / `end` / `end_error` events
for that operation.

Inner layers (engine, store, core) MUST NOT emit additional `start`/`end` pairs
for the same `op`. They MAY:
- Create child spans for internal steps
- Emit `DEBUG`/`TRACE` events for internal detail
- Emit `WARN`/`ERROR` for significant internal conditions

The boundary wrapper pattern (normative):
```
log_op_start!(op="ettle_update", ettle_id=...)
let result = execute_body();
match result {
    Ok(_) => log_op_end!(op="ettle_update", duration_ms=...),
    Err(e) => log_op_error!(op="ettle_update", err=e, duration_ms=...),
}
```

For vertical slices: the MCP and CLI entry points are the boundary. Engine and store
handlers MUST NOT emit lifecycle events for operations already owned by MCP/CLI.

---

## C4 — Sensitive data and redaction

`ettlex_core_types` MUST define `Sensitive<T>` and redaction helpers.

- Any potentially sensitive value MUST be wrapped in `Sensitive<T>` at the earliest
  boundary where it enters the system.
- Logging macros MUST refuse to log `Sensitive<T>` raw values.
- Logs MUST NEVER include: raw secrets, credentials, raw WHY/WHAT/HOW content fields,
  or any user-authored reasoning content.

---

## C5 — Correlation propagation

`RequestId`, `TraceId`, and `RequestContext` MUST live in `ettlex_core_types`.

Rules:
- A `RequestContext` MUST be created or retrieved on entry to each externally
  meaningful operation and attached to the root tracing span.
- Async task spawns within an operation MUST propagate the current span so
  `request_id` is preserved in all child log output.
- If a code path does not run under spans (exceptional), `RequestContext` MUST
  be threaded explicitly as a parameter.

---

## C6 — Severity rules

| Level | When to use |
|---|---|
| `INFO` | Lifecycle events (`start`, `end`, `end_error`) for externally meaningful operations |
| `WARN` | Degraded behaviour, retries, policy avoidance, suspicious input, fallback paths |
| `ERROR` | Operation failed — always paired with `err.kind` |
| `DEBUG` / `TRACE` | Internal steps — disabled by default in production |

Start/end pairing rule: every externally meaningful operation MUST have a `start`
event and either an `end` or `end_error` event, sharing the same correlation
identifiers.

---

## C7 — Testability

The logging spine MUST provide a test capture mode that:
- Collects events in-memory in emission order
- Exposes a stable representation for assertions
- Omits non-deterministic fields (timestamps, thread IDs) from assertions by default

Tests MUST assert on event name and canonical field values.
Tests MUST NOT assert on raw timestamp values or thread identifiers.

---

## C8 — Dependency direction

The dependency order is fixed and MUST NOT be violated:

```
ettlex_core_types  (no workspace dependencies)
       ↓
  error_facility   (depends on ettlex_core_types only)
       ↓
 logging_facility  (depends on error_facility + ettlex_core_types)
       ↓
  all other crates
```

`error_facility` MUST NOT depend on `logging_facility`.
`ettlex_core_types` MUST NOT depend on any other workspace crate.

---

## Conformance check (for code generators)

Before producing any scenario implementation, verify:

- [ ] No `println!`/`eprintln!` in non-test code
- [ ] No direct subscriber initialisation outside `logging_facility`
- [ ] All boundary entry points own exactly one `start`/`end` pair
- [ ] Inner layers (engine, store) emit no lifecycle events for ops owned by MCP/CLI
- [ ] All error log events include `err.kind`
- [ ] No sensitive values logged without `Sensitive<T>` wrapper
- [ ] No new code uses `ep_id` or `ep_ordinal` log fields
- [ ] Async task spawns propagate the current tracing span
- [ ] `RequestContext` is created at each operation entry point
