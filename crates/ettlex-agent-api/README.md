# ettlex-agent-api

**EttleX Agent API — Agent Surface Definitions**

Defines the public interface surface for EttleX AI agents. This crate provides shared
type definitions that agent implementations depend on, without exposing engine or store
internals.

## Purpose

`ettlex-agent-api` enforces the dependency boundary between agents and the engine:

```
agent implementations
    ↓
ettlex-agent-api    ← this crate (types only)
    ↓
ettlex-memory       ← runtime dependency for actual agents
```

By depending only on `ettlex-agent-api`, agent code never directly references
`ettlex-engine` or `ettlex-store`.

## Status

**Phase 1 stub.** The public API surface is currently empty. Types will be added in
future slices as the agent execution model is defined.

## Dependencies

This crate has no workspace dependencies other than `ettlex-errors`. It MUST NOT
depend on `ettlex-engine`, `ettlex-store`, or `ettlex-memory`.
