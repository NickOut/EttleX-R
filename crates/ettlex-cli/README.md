# ettlex-cli

**EttleX Command-Line Interface**

Command-line tool for managing EttleX semantic architectures, seed import, and rendering.

## Overview

EttleX CLI provides a user-friendly command-line interface for working with EttleX repositories. It orchestrates operations across the core domain, storage, and projection layers.

## Installation

Build from source:

```bash
cargo build --release -p ettlex-cli
```

The binary will be at `target/release/ettlex-cli`.

## Commands

### `seed` - Seed Operations

Import seed YAML files into the repository.

```bash
ettlex seed import <path>
```

**Arguments**:

- `<path>` - Path to seed YAML file

**Example**:

```bash
ettlex seed import handoff/seed_snapshot_commit_v4.yaml
```

**Output**:

```
Importing handoff/seed_snapshot_commit_v4.yaml...
✓ Imported (digest: ce110808dd873059470af8e81d94b3ba9ac0e5f5d6acaafc83b2db26b4d1fe2d)
```

**Features**:

- Cross-seed reference support (references entities from previously imported seeds)
- Transaction-based atomic import with automatic rollback on failure
- Provenance event tracking (started/applied/completed)
- Duplicate child mapping detection (enforces EP uniqueness invariant)

### `render` - Render to Markdown

Render Ettles or bundles to human-readable Markdown.

#### Render Single Ettle

```bash
ettlex render ettle <ettle_id> [-o <output>]
```

**Arguments**:

- `<ettle_id>` - Ettle ID to render (e.g., `ettle:root`)
- `-o, --output <path>` - Output file path (optional, defaults to stdout)

**Example**:

```bash
ettlex render ettle ettle:snapshot_diff > rendered/snapshot_diff.md
```

**Output format**:

```markdown
# {Ettle Title}

## EP {ordinal}

**Normative**: {Yes|No}

**WHY**: {why content}

**WHAT**: {what content}

**HOW**: {how content}
```

#### Render Leaf Bundle

Render full EPT path for a leaf Ettle (includes all ancestors).

```bash
ettlex render bundle <leaf_id> [-e <ordinal>] [-o <output>]
```

**Arguments**:

- `<leaf_id>` - Leaf Ettle ID to render
- `-e, --ep-ordinal <n>` - EP ordinal for leaf (optional)
- `-o, --output <path>` - Output file path (optional, defaults to stdout)

**Example**:

```bash
ettlex render bundle ettle:snapshot_diff > rendered/snapshot_diff_bundle.md
```

**Output format**:

```markdown
# Leaf Bundle: {Root Title} > {Parent Title} > ... > {Leaf Title}

## WHY (Rationale)

{Combined WHY from all EPs in path}

## WHAT (Description)

{Combined WHAT from all EPs in path}

## HOW (Implementation)

{Combined HOW from all EPs in path}
```

## Repository Structure

EttleX CLI expects the following repository structure:

```
.ettlex/
├── store.db      # SQLite database
└── cas/          # Content-addressable storage
    └── {hex}/    # First 2 hex chars of digest
        └── {digest}.{ext}
```

This structure is automatically created by the first `seed import` operation.

## Error Handling

CLI uses exit codes to indicate success/failure:

- **0**: Success
- **1**: Error (details printed to stderr)

Error messages include:

- Operation context
- Root cause
- Suggested remediation (when applicable)

## Commands Module Structure

The CLI is organized into command modules:

```
ettlex-cli/
└── src/
    ├── commands/
    │   ├── mod.rs       # Command module registry
    │   ├── seed.rs      # Seed import commands
    │   └── render.rs    # Render commands
    └── main.rs          # CLI argument parsing and dispatch
```

## Development

### Adding a New Command

1. Create command module in `src/commands/{name}.rs`
2. Define args struct with `clap` derives
3. Implement `execute()` function
4. Register in `src/commands/mod.rs`
5. Add variant to `Commands` enum in `src/main.rs`

Example:

```rust
// src/commands/status.rs
use clap::Args;

#[derive(Debug, Args)]
pub struct StatusArgs {
    // args here
}

pub fn execute(args: StatusArgs) -> Result<(), Box<dyn std::error::Error>> {
    // implementation
    Ok(())
}
```

### Testing

Integration tests live in the command modules:

```bash
cargo test -p ettlex-cli
```

## Dependencies

Key dependencies:

- `clap` - Command-line argument parsing
- `ettlex-engine` - High-level command orchestration
- `ettlex-store` - Persistence layer
- `ettlex-core` - Domain models

## Future Commands

Planned commands:

- `ettlex snapshot commit` - Create snapshot commit
- `ettlex snapshot list` - List committed snapshots
- `ettlex snapshot diff` - Diff two snapshots
- `ettlex validate` - Run tree validation
- `ettlex gc` - Garbage collect unreferenced content
- `ettlex export` - Export to JSON/ArchiMate/other formats
