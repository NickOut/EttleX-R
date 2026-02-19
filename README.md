# EttleX

Rust workspace scaffolding for:

- ettlex-core (pure domain)
- ettlex-store (persistence + ledger + CAS)
- ettlex-engine (orchestration + previews + snapshot commits)
- ettlex-projection (exporters/views)
- ettlex-mcp (MCP tool server)
- ettlex-cli (CLI)
- ettlex-tauri (Tauri backend command bridge)

Front-end lives in apps/ettlex-desktop (Tauri + SvelteKit).
