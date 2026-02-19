# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Build Commands

```bash
cargo build                        # Build entire workspace
cargo build -p ettlex-core         # Build a single crate
cargo test                         # Run all tests
cargo test -p ettlex-core          # Test a single crate
cargo test -p ettlex-core test_name # Run a specific test
cargo clippy --workspace           # Lint all crates
cargo fmt --all -- --check         # Check formatting
cargo fmt --all                    # Auto-format
```

Binaries: `ettlex-cli`, `ettlex-mcp`, `ettlex-tauri` (run with `cargo run -p <name>`).

## Architecture

EttleX is a Rust monorepo workspace for architectural transformation/testing with event sourcing, content-addressable storage, and multi-format projections.

### Crate Dependency Layers (top → bottom)

```
ettlex-cli / ettlex-mcp / ettlex-tauri   ← entry points (binaries)
        ↓
   ettlex-engine                          ← orchestration, previews, snapshot commits, git mirror
        ↓
ettlex-projection    ettlex-store         ← exporters (JSON/MD/ArchiMate) │ persistence (ledger/CAS/repo)
        ↓                ↓
            ettlex-core                   ← pure domain: models, ops, rules, TES, diff, coverage
```

### Key Patterns

- **Workspace dependencies**: Common deps (serde, uuid v7, thiserror, tracing) are declared in root `Cargo.toml` under `[workspace.dependencies]` and inherited by crates.
- **Error handling**: Uses `thiserror` with per-crate error types (see `errors.rs` in core).
- **ettlex-store**: Implements repository pattern, event ledger, content-addressable storage (CAS), schema management, and transactions (`txn.rs`).
- **ettlex-engine**: Command abstractions in `commands/`, preview generation, snapshot commits, and git synchronization via `git_mirror/`.
- **ettlex-projection**: Format-specific exporters in `json/`, `markdown/`, `archimate/` subdirectories.

### Desktop App

`apps/ettlex-desktop/` is a planned Tauri + SvelteKit frontend (not yet initialized). `ettlex-tauri` provides the Rust backend command bridge with state management (`state.rs`) and command handlers.
