```
ettlex/
    Cargo.toml                 # workspace root
    crates/
        ettlex-core/             # pure domain + operations (no IO)
            Cargo.toml
            src/
                lib.rs
                model/               # Ettle, EP, Constraint, Snapshot types
                ops/                 # split/merge/reparent/partition/apply-constraint
                rules/               # monotonicity, reachability, selector eval
                coverage/            # coverage computation
                diff/                # manifest diffs, CIA primitives
                tes/                 # Triad Expectation Set derivation
                errors.rs

        ettlex-store/            # persistence boundary (SQLite/Postgres later)
          Cargo.toml
          src/
            lib.rs
            schema/              # migrations (sqlx) + types
            repo/                # repositories: ettles, eps, constraints, etc.
            txn.rs               # transactional wrapper
            cas/                 # content-addressed blob store
            ledger/              # append-only facet_snapshots + provenance_events

        ettlex-projection/       # exporters: markdown/json/graph views
          Cargo.toml
          src/
            lib.rs
            markdown/
            json/
            archimate/           # optional: view generation scaffolding

        ettlex-engine/           # application service layer (orchestrates core+store)
          Cargo.toml
          src/
            lib.rs
            commands/            # command handlers producing preview bundles
            preview/             # impact preview computation
            snapshot/            # snapshot commit flow
            git_mirror/          # optional: export + commit + record git hash

        ettlex-mcp/              # MCP server (tool surface)
          Cargo.toml
          src/
            main.rs
            tools/               # tool handlers mapping to engine commands

        ettlex-cli/              # CLI (calls engine directly or via MCP)
          Cargo.toml
          src/
            main.rs
            commands/

        ettlex-tauri/            # Tauri backend (IPC bridge to engine)
          Cargo.toml
          src/
            main.rs
            commands/            # #[tauri::command] wrappers
            state.rs             # app state (db path, handles, config)

apps/
ettlex-desktop/          # Tauri + SvelteKit frontend
src-tauri/             # points to crates/ettlex-tauri (or contains it)
src/                   # SvelteKit UI

tools/
migrations/              # optional central migrations folder
scripts/
docs/       # product docs
target/docs         # rustdocs output
README.md
```
