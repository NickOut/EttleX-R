# Ettle: Slice 00 — Infrastructure: logging_facility + error_facility crate extraction

**Ettle ID:** ettle:019cf0f6-5797-73a3-895a-c42ef4f22e38
**Status:** Generator-ready
**Layers:** ettlex-errors (new), ettlex-logging (new), ettlex-core (refactor), ettlex-store (migration), ettlex-engine (migration), ettlex-mcp (migration)
**Prerequisite for:** All subsequent vertical slices

---

## WHY

The EttleX codebase currently has two architectural violations that must be resolved before the vertical slice programme can proceed cleanly.

First, `logging_facility` is implemented as a module inside `ettlex-core` (`ettlex-core/src/logging_facility/`). The logging constraint requires it to be a standalone crate that all other crates depend on. Having it inside `ettlex-core` inverts the dependency: crates that should sit below `ettlex-core` (like `ettlex-store`) cannot currently use the logging spine without taking a dependency on the full core domain model. This must be resolved before any slice introduces new logging calls.

Second, `ExError` and `ExErrorKind` are implemented inside `ettlex-core/src/errors.rs` alongside `EttleXError`, the legacy error enum. The error handling constraint requires `error_facility` to be a standalone crate depending only on `ettlex-core-types`. Currently any crate that wants `ExError` must depend on all of `ettlex-core`. Additionally, `EttleXError` is a legacy enum that should be retired in favour of `ExError` across the codebase; having both creates confusion and drift.

Third, the `ExErrorKind` taxonomy is missing four variants required by Slice 01: `AlreadyTombstoned`, `SelfReferentialLink`, `HasActiveDependants`, and `MissingLinkType`. These must be added here, not in Slice 01, so the error taxonomy is complete before feature work begins.

`ettlex-core-types` already exists as a standalone crate in the correct position. No changes are needed to its structure, though its content should be verified against the constraint definition.

This infrastructure slice is a prerequisite for all subsequent vertical slices. It produces no new user-visible behaviour — it is a structural refactor that moves existing implementations to their correct positions in the dependency graph.

---

## WHAT

### Scope

This slice performs the following structural changes:

1. Extract `logging_facility` from `ettlex-core` into a new standalone `ettlex-logging` crate
2. Extract `ExError`, `ExErrorKind`, and related error infrastructure from `ettlex-core` into a new standalone `ettlex-errors` crate
3. Retire `EttleXError` by migrating all usage to `ExError` across the workspace
4. Add four missing `ExErrorKind` variants: `AlreadyTombstoned`, `SelfReferentialLink`, `HasActiveDependants`, `MissingLinkType`
5. Update all `Cargo.toml` workspace dependencies to reflect the new crate structure
6. Verify `ettlex-core-types` content matches constraint definition

### New crate: `ettlex-errors`

Contains:
- `ExErrorKind` enum with all variants (including the four new ones)
- `ExError` struct with all builder methods
- `assert_err_kind!` and `assert_err_field!` test macros
- `From<T>` conversions for `rusqlite::Error`, `serde_json::Error`, `std::io::Error`
- NO dependency on `ettlex-core` or `ettlex-logging`
- Depends only on `ettlex-core-types`

New `ExErrorKind` variants:
- `AlreadyTombstoned` — operation attempted on a record whose `tombstoned_at` is set; distinct from `Deleted` (legacy boolean column)
- `SelfReferentialLink` — `reasoning_link_id` set to the Ettle's own ID
- `HasActiveDependants` — tombstone blocked because active records reference this entity
- `MissingLinkType` — `reasoning_link_id` supplied without `reasoning_link_type`

Stable error codes for new variants:
- `AlreadyTombstoned` → `ERR_ALREADY_TOMBSTONED`
- `SelfReferentialLink` → `ERR_SELF_REFERENTIAL_LINK`
- `HasActiveDependants` → `ERR_HAS_ACTIVE_DEPENDANTS`
- `MissingLinkType` → `ERR_MISSING_LINK_TYPE`

### New crate: `ettlex-logging`

Contains:
- `logging_facility::init(profile)` — single initialisation point
- `Profile` enum (Development, Production)
- `log_op_start!`, `log_op_end!`, `log_op_error!` macros
- `TestCapture`, `CapturedEvent`, `init_test_capture` — test capture mode
- Root span wrapper for boundary entrypoints
- Depends on `ettlex-errors` and `ettlex-core-types`
- Does NOT depend on `ettlex-core`

### EttleXError retirement

`EttleXError` is the legacy error enum in `ettlex-core/src/errors.rs`. It coexists with `ExError` via a `From<EttleXError> for ExError` bridge.

This slice retires `EttleXError` by:
- Migrating all internal `ettlex-core` code that produces `EttleXError` to produce `ExError` directly
- Migrating all `ettlex-engine`, `ettlex-store`, `ettlex-mcp` code that references `EttleXError` to use `ExError`
- Removing the `EttleXError` enum and its `From` bridge once all usages are migrated
- The `Result<T>` alias in `ettlex-errors` is `std::result::Result<T, ExError>`

### Dependency graph after this slice

```
ettlex-cli  ettlex-mcp  ettlex-tauri       (entry points)
        ↓         ↓           ↓
          ettlex-engine
               ↓
          ettlex-store
               ↓
          ettlex-core
               ↓
        ettlex-logging
               ↓
        ettlex-errors
               ↓
       ettlex-core-types        (no workspace deps)

ettlex-projection → ettlex-core only
```

### Workspace Cargo.toml changes

- Add `ettlex-errors` and `ettlex-logging` to `[workspace]` members
- Add `ettlex-errors` and `ettlex-logging` to `[workspace.dependencies]`
- All crates that previously depended on `ettlex-core` for `ExError`/`ExErrorKind` must now depend on `ettlex-errors`
- All crates that previously depended on `ettlex-core` for logging must now depend on `ettlex-logging`
- `ettlex-core` depends on `ettlex-logging` and `ettlex-errors`
- `ettlex-core/src/errors.rs` retains only re-exports from `ettlex-errors`; `EttleXError` is removed
- `ettlex-core/src/logging_facility/` is removed; `ettlex-core` imports from `ettlex-logging`

### ettlex-core-types verification

Verify the following are present and correct in `ettlex-core-types`:
- `RequestId`, `TraceId`, `SpanId` types (correlation IDs)
- `RequestContext` struct
- Canonical schema field key constants (`component`, `op`, `event`, `duration_ms`, `request_id`, `trace_id`, `ettle_id`, `err.kind`, `err.code`)
- `Sensitive<T>` marker and redaction helpers
- No subscriber initialisation, no IO, no logging dependency

If any of these are absent, add them as part of this slice.

### Layer responsibilities (unchanged by this slice)

No business logic changes. No new user-visible behaviour. No schema changes. No MCP tool changes. This slice moves code to correct positions only.

### Out of scope

- Any feature work
- Schema migrations
- New MCP tools or commands
- Snapshot, decision, constraint, profile changes
- CLI wiring

---

## HOW

### Architectural conformance invariants

The following MUST hold after this slice. These are tested by the scenarios below.

- **IC-1:** `ettlex-errors` MUST NOT depend on `ettlex-core`, `ettlex-logging`, `ettlex-store`, or `ettlex-engine`.
- **IC-2:** `ettlex-logging` MUST NOT depend on `ettlex-core`, `ettlex-store`, or `ettlex-engine`.
- **IC-3:** `ettlex-core` MUST NOT define `EttleXError`. No `EttleXError` type MUST exist anywhere in the workspace after this slice.
- **IC-4:** All fallible public functions in `ettlex-core`, `ettlex-store`, and `ettlex-engine` MUST return `Result<T, ExError>` (from `ettlex-errors`).
- **IC-5:** No crate MUST call `tracing_subscriber::init()` directly. All logging initialisation MUST go through `ettlex_logging::init()`.
- **IC-6:** `println!` and `eprintln!` MUST NOT appear in non-test code anywhere in the workspace.
- **IC-7:** `AlreadyTombstoned`, `SelfReferentialLink`, `HasActiveDependants`, and `MissingLinkType` MUST exist in `ExErrorKind` with their specified stable codes.
- **IC-8:** The `From<EttleXError> for ExError` bridge MUST NOT exist after this slice.

### Scenarios (all MUST be implemented as tests; Gherkin is normative)

#### Feature: ettlex-errors crate — happy path

```gherkin
Scenario: ExError can be constructed with all builder fields
  When I construct ExError::new(ExErrorKind::NotFound)
    .with_op("test_op")
    .with_entity_id("ettle:1")
    .with_message("not found")
  Then ex.kind() == ExErrorKind::NotFound
  And ex.op() == Some("test_op")
  And ex.entity_id() == Some("ettle:1")
  And ex.message() == "not found"
  And ex.code() == "ERR_NOT_FOUND"

Scenario: assert_err_kind! macro passes on correct kind
  Given a Result::Err(ExError::new(ExErrorKind::NotFound))
  When I call assert_err_kind!(result, ExErrorKind::NotFound)
  Then the assertion passes

Scenario: assert_err_kind! macro fails on wrong kind
  Given a Result::Err(ExError::new(ExErrorKind::NotFound))
  When I call assert_err_kind!(result, ExErrorKind::InvalidInput)
  Then the test panics with a descriptive message
```

#### Feature: New ExErrorKind variants

```gherkin
Scenario: AlreadyTombstoned has correct stable code
  When I call ExErrorKind::AlreadyTombstoned.code()
  Then the result is "ERR_ALREADY_TOMBSTONED"

Scenario: SelfReferentialLink has correct stable code
  When I call ExErrorKind::SelfReferentialLink.code()
  Then the result is "ERR_SELF_REFERENTIAL_LINK"

Scenario: HasActiveDependants has correct stable code
  When I call ExErrorKind::HasActiveDependants.code()
  Then the result is "ERR_HAS_ACTIVE_DEPENDANTS"

Scenario: MissingLinkType has correct stable code
  When I call ExErrorKind::MissingLinkType.code()
  Then the result is "ERR_MISSING_LINK_TYPE"

Scenario: All four new variants are distinct from each other and from Deleted
  Then ExErrorKind::AlreadyTombstoned != ExErrorKind::Deleted
  And ExErrorKind::AlreadyTombstoned != ExErrorKind::SelfReferentialLink
  And ExErrorKind::AlreadyTombstoned != ExErrorKind::HasActiveDependants
  And ExErrorKind::AlreadyTombstoned != ExErrorKind::MissingLinkType
```

#### Feature: ettlex-logging crate — happy path

```gherkin
Scenario: init(Profile::Development) succeeds without panic
  When I call ettlex_logging::init(Profile::Development)
  Then no panic occurs
  And subsequent log calls do not panic

Scenario: init_test_capture returns a TestCapture handle
  When I call ettlex_logging::init_test_capture()
  Then a TestCapture handle is returned
  And captured events can be retrieved in emission order

Scenario: log_op_start! emits a structured start event
  Given test capture is initialised
  When I call log_op_start!(op = "test_op", ettle_id = "ettle:1")
  Then a captured event with event="start" and op="test_op" is recorded

Scenario: log_op_end! emits a structured end event with duration
  Given test capture is initialised
  When I call log_op_end!(op = "test_op", duration_ms = 42)
  Then a captured event with event="end" and op="test_op" and duration_ms=42 is recorded

Scenario: log_op_error! emits a structured error event with err.kind
  Given test capture is initialised
  And ExError err with kind NotFound
  When I call log_op_error!(op = "test_op", err = err, duration_ms = 5)
  Then a captured event with event="end_error" and err.kind="ERR_NOT_FOUND" is recorded
```

#### Feature: ettlex-logging — enforcement

```gherkin
Scenario: println! in non-test code is rejected by CI
  Given a non-test source file contains println!("debug")
  When scripts/check_banned_patterns.sh runs
  Then the check fails with a diagnostic identifying the file and line

Scenario: Direct tracing_subscriber initialisation is rejected by CI
  Given a non-test source file calls tracing_subscriber::fmt().init()
  When scripts/check_banned_patterns.sh runs
  Then the check fails with a diagnostic identifying the file and line
```

#### Feature: Architectural conformance — dependency graph

```gherkin
Scenario: ettlex-errors has no dependency on ettlex-core (IC-1)
  When I inspect ettlex-errors/Cargo.toml
  Then ettlex-core is not listed as a dependency
  And ettlex-store is not listed as a dependency
  And ettlex-engine is not listed as a dependency
  And ettlex-logging is not listed as a dependency

Scenario: ettlex-logging has no dependency on ettlex-core (IC-2)
  When I inspect ettlex-logging/Cargo.toml
  Then ettlex-core is not listed as a dependency
  And ettlex-store is not listed as a dependency
  And ettlex-engine is not listed as a dependency

Scenario: EttleXError does not exist in the workspace (IC-3)
  When I search the workspace source for the identifier EttleXError
  Then no occurrences are found outside of test files and migration comments

Scenario: All public store functions return Result<T, ExError> (IC-4)
  When I inspect the public API of ettlex-store
  Then every fallible public function returns Result<T, ExError>
  And no function returns Result<T, EttleXError>

Scenario: All public engine functions return Result<T, ExError> (IC-4)
  When I inspect the public API of ettlex-engine
  Then every fallible public function returns Result<T, ExError>
  And no function returns Result<T, EttleXError>

Scenario: No direct tracing_subscriber::init calls exist (IC-5)
  When I search the workspace source for tracing_subscriber::fmt and tracing_subscriber::init
  Then no occurrences are found outside of ettlex-logging/src/

Scenario: No println!/eprintln! in non-test code (IC-6)
  When scripts/check_banned_patterns.sh runs against the full workspace
  Then no violations are reported
```

#### Feature: EttleXError retirement (IC-3, IC-8)

```gherkin
Scenario: From<EttleXError> for ExError bridge does not exist
  When I search the workspace for impl From<EttleXError>
  Then no occurrences are found

Scenario: EttleXError enum is not defined anywhere in the workspace
  When I search the workspace for enum EttleXError
  Then no occurrences are found
```

#### Feature: ettlex-core-types verification

```gherkin
Scenario: RequestId, TraceId, RequestContext are defined in ettlex-core-types
  When I inspect ettlex-core-types/src/
  Then RequestId is defined
  And TraceId is defined
  And RequestContext is defined

Scenario: Sensitive<T> is defined in ettlex-core-types
  When I inspect ettlex-core-types/src/
  Then Sensitive<T> is defined
  And Sensitive<T> does not implement Display or Debug in a way that exposes the inner value

Scenario: ettlex-core-types has no dependency on any other workspace crate
  When I inspect ettlex-core-types/Cargo.toml
  Then no workspace crate appears as a dependency
```

#### Feature: No behavioural regression

```gherkin
Scenario: All pre-existing passing tests continue to pass after extraction
  Given the workspace test suite passes before this slice
  When this slice is implemented
  Then make test-slice passes with zero failures
  And make test produces no new failures beyond the pre-authorised failure list
```

### Expected Failure Registry (pre-authorised)

**EFR-01:** Any test in `ettlex-core/tests/` that imports `EttleXError` directly — these will fail once `EttleXError` is removed. The code generator MUST migrate these tests to use `ExError` as part of this slice, since the retirement of `EttleXError` is within the slice boundary.

**Note:** Unlike feature slices, this infrastructure slice is permitted to update test files that reference `EttleXError` because the retirement of `EttleXError` is the explicit intent of the slice. The coexistence rule applies to code *outside* the slice boundary; `EttleXError` references anywhere in the workspace are within scope for this slice.
