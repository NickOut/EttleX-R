# ettlex-core-types

**EttleX Core Types - Shared Foundational Types**

Shared types used across EttleX facilities for correlation, sensitive data handling, and schema constants.

## Overview

EttleX Core Types provides foundational types that are shared between error handling and logging facilities:

- **Correlation types**: Request/trace/span tracking for distributed operations
- **Sensitive data**: Type-safe marker for automatic redaction in logs
- **Schema constants**: Canonical field keys and event names

This crate is dependency-free and can be used by all layers of the EttleX stack.

## Features

### ✅ Correlation Types

Support for distributed tracing and request correlation.

```rust
use ettlex_core_types::{RequestId, TraceId, SpanId, RequestContext};

// Generate new correlation IDs
let request_id = RequestId::new();
let trace_id = TraceId::new();
let span_id = SpanId::new();

// Create request context
let ctx = RequestContext {
    request_id,
    trace_id,
    span_id,
    parent_span_id: None,
};
```

**Types**:

- `RequestId` - Unique identifier for a single API request
- `TraceId` - Identifier for a distributed trace spanning multiple requests
- `SpanId` - Identifier for a single operation within a trace
- `RequestContext` - Container for all correlation IDs

**Format**: UUIDv7 (time-ordered, monotonic within millisecond)

**Use cases**:

- Distributed tracing across CLI/MCP/Tauri surfaces
- Log correlation for debugging multi-step operations
- Error context propagation through call chains

### ✅ Sensitive Data Marker

Type-safe wrapper for sensitive data with automatic redaction.

```rust
use ettlex_core_types::Sensitive;

// Wrap sensitive data
let password = Sensitive::new("secret123");
let api_key = Sensitive::new("sk-1234567890");

// Debug output is redacted
println!("{:?}", password);  // Output: Sensitive([REDACTED])

// Explicit access when needed
let actual_value = password.expose();
```

**Properties**:

- Automatic redaction in `Debug` and `Display` implementations
- Explicit `expose()` method for intentional access
- No accidental leakage in logs or error messages
- Zero-cost abstraction (no runtime overhead)

**Use cases**:

- Database credentials
- API tokens
- User passwords
- Encryption keys
- Any PII (personally identifiable information)

### ✅ Schema Constants

Canonical field keys and event names for consistent logging and error metadata.

```rust
use ettlex_core_types::schema;

// Field keys
const OPERATION: &str = schema::OPERATION;        // "operation"
const DURATION_MS: &str = schema::DURATION_MS;    // "duration_ms"
const ERROR_KIND: &str = schema::ERROR_KIND;      // "err.kind"
const ERROR_MESSAGE: &str = schema::ERROR_MESSAGE; // "err.message"

// Event names
const START: &str = schema::START;                // "start"
const END: &str = schema::END;                    // "end"
const END_ERROR: &str = schema::END_ERROR;        // "end_error"
```

**Purpose**:

- Consistent field naming across logging and error facilities
- Enables log parsing and querying
- Supports structured logging standards (e.g., OpenTelemetry)

## Module Documentation

### `correlation` - Request Correlation

Provides types for tracking requests across distributed systems.

**Public types**:

- `RequestId` - Single request identifier
- `TraceId` - Distributed trace identifier
- `SpanId` - Operation span identifier
- `RequestContext` - Full correlation context

**ID generation**:

- Uses UUIDv7 for time-ordered, globally unique IDs
- Thread-safe generation
- Monotonic within millisecond precision

### `sensitive` - Sensitive Data Handling

Type-safe wrapper for sensitive data with automatic redaction.

**Public types**:

- `Sensitive<T>` - Wrapper for sensitive data of type T

**Traits**:

- `Debug` - Always returns `Sensitive([REDACTED])`
- `Display` - Always returns `[REDACTED]`
- `Clone` - Clones wrapped value
- `PartialEq` - Compares wrapped values

**Methods**:

- `new(value: T) -> Sensitive<T>` - Wrap sensitive value
- `expose(&self) -> &T` - Explicitly access wrapped value

### `schema` - Schema Constants

Canonical field keys and event names.

**Constant categories**:

- **Operation fields**: `OPERATION`, `DURATION_MS`, `TIMESTAMP`
- **Error fields**: `ERROR_KIND`, `ERROR_MESSAGE`, `ERROR_SOURCE`
- **Correlation fields**: `REQUEST_ID`, `TRACE_ID`, `SPAN_ID`
- **Event names**: `START`, `END`, `END_ERROR`

## Usage Examples

### Correlation in Multi-Step Operation

```rust
use ettlex_core_types::RequestContext;

fn multi_step_operation(ctx: &RequestContext) -> Result<()> {
    // Log with correlation
    tracing::info!(
        request_id = %ctx.request_id,
        trace_id = %ctx.trace_id,
        span_id = %ctx.span_id,
        "Starting operation"
    );

    // Pass context to child operations
    step_1(ctx)?;
    step_2(ctx)?;

    Ok(())
}
```

### Sensitive Data in Config

```rust
use ettlex_core_types::Sensitive;
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
struct DatabaseConfig {
    pub host: String,
    pub port: u16,
    pub password: Sensitive<String>,
}

let config = DatabaseConfig {
    host: "localhost".into(),
    port: 5432,
    password: Sensitive::new("supersecret".into()),
};

// Safe to log - password is redacted
tracing::debug!(?config, "Loaded database config");
```

### Schema Constants in Logging

```rust
use ettlex_core_types::schema;

tracing::info!(
    { schema::OPERATION } = "snapshot_commit",
    { schema::DURATION_MS } = 250,
    { schema::END } = true,
    "Operation completed"
);
```

## Testing

Run tests:

```bash
cargo test -p ettlex-core-types
```

Test coverage:

- Correlation ID generation and uniqueness
- Sensitive data redaction in Debug/Display
- Schema constant values and consistency

## Dependencies

Zero external dependencies beyond:

- `uuid` (v7 feature) - Correlation ID generation

## Design Principles

1. **Dependency-free**: Minimal dependencies to avoid conflicts
2. **Type-safe**: Use types to prevent mistakes (e.g., leaking sensitive data)
3. **Zero-cost**: No runtime overhead for abstractions
4. **Composable**: Types can be used independently or together

## Future Work

Planned enhancements:

- [ ] OpenTelemetry span context integration
- [ ] Custom sensitive data serializers for JSON/YAML
- [ ] Schema validation utilities
- [ ] Additional correlation ID formats (e.g., W3C Trace Context)
