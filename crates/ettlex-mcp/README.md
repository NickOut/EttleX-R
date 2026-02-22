# ettlex-mcp

**EttleX MCP Server - Model Context Protocol Integration**

MCP server for integrating EttleX with AI assistants and development tools.

## Status

🚧 **Stub Crate** - Not yet implemented

This crate is a placeholder for future MCP server functionality.

## Overview

EttleX MCP will provide a Model Context Protocol (MCP) server that allows AI assistants and other MCP clients to interact with EttleX repositories.

## Planned Features

### 🔧 Tools (MCP)

Expose EttleX operations as MCP tools:

- **`seed_import`** - Import seed YAML files
- **`render_ettle`** - Render single Ettle to Markdown
- **`render_bundle`** - Render leaf bundle (full EPT path)
- **`validate_tree`** - Validate tree invariants
- **`snapshot_commit`** - Create snapshot commit
- **`snapshot_diff`** - Diff two snapshots
- **`query_ept`** - Compute EPT for an Ettle
- **`search_ettles`** - Search Ettles by title/content

### 📚 Resources (MCP)

Expose EttleX entities as MCP resources:

- **`ettle://{id}`** - Single Ettle with all EPs
- **`ep://{id}`** - Single EP with content
- **`bundle://{leaf_id}`** - Full bundle (EPT path)
- **`snapshot://{id}`** - Snapshot manifest
- **`tree://`** - Full tree structure

### 📝 Prompts (MCP)

Pre-configured prompts for common workflows:

- **`review_ettle`** - Review Ettle for completeness
- **`suggest_refinement`** - Suggest child Ettles
- **`validate_seed`** - Validate seed YAML syntax
- **`generate_scenarios`** - Generate Gherkin scenarios from WHAT

## Architecture

```
MCP Client (Claude Desktop, VS Code, etc.)
    ↓ (JSON-RPC over stdio)
MCP Server (ettlex-mcp)
    ↓
Engine Commands (ettlex-engine)
    ↓
Store/Core Layers
```

## Usage (Planned)

### Running the Server

```bash
ettlex-mcp
```

The server communicates over stdio using JSON-RPC as defined by the MCP protocol.

### Configuration (claude_desktop_config.json)

```json
{
  "mcpServers": {
    "ettlex": {
      "command": "/path/to/ettlex-mcp",
      "args": [],
      "cwd": "/path/to/ettlex/repo"
    }
  }
}
```

### Example Tool Call

```json
{
  "jsonrpc": "2.0",
  "id": 1,
  "method": "tools/call",
  "params": {
    "name": "render_ettle",
    "arguments": {
      "ettle_id": "ettle:snapshot_commit"
    }
  }
}
```

### Example Resource Access

```json
{
  "jsonrpc": "2.0",
  "id": 2,
  "method": "resources/read",
  "params": {
    "uri": "ettle://snapshot_commit"
  }
}
```

## Tool Definitions (Planned)

### `seed_import`

Import a seed YAML file into the repository.

**Input**:

```json
{
  "path": "handoff/seed_example.yaml"
}
```

**Output**:

```json
{
  "seed_digest": "ce110808...",
  "imported_ettles": ["ettle:root", "ettle:store"],
  "imported_links": 2
}
```

### `render_bundle`

Render full EPT path for a leaf Ettle.

**Input**:

```json
{
  "leaf_id": "ettle:snapshot_diff",
  "ep_ordinal": 0
}
```

**Output**:

```json
{
  "markdown": "# Leaf Bundle: ...\n\n## WHY...",
  "ept": ["ep:root:1", "ep:store:0", "ep:snapshot_commit:0"],
  "path": ["ettle:root", "ettle:store", "ettle:snapshot_commit", "ettle:snapshot_diff"]
}
```

### `validate_tree`

Validate tree invariants for an Ettle.

**Input**:

```json
{
  "root_ettle_id": "ettle:root"
}
```

**Output**:

```json
{
  "valid": true,
  "violations": [],
  "statistics": {
    "total_ettles": 5,
    "total_eps": 12,
    "total_links": 4
  }
}
```

## Dependencies (Planned)

- `ettlex-engine` - Command orchestration
- `ettlex-core` - Domain models
- `serde` / `serde_json` - JSON-RPC serialization
- `tokio` - Async runtime
- MCP SDK (when available)

## Testing

MCP server will include:

- Unit tests for tool handlers
- Integration tests with mock MCP client
- Resource serialization tests
- Error handling tests

## Future Work

- [ ] Implement MCP server framework
- [ ] Add tool handlers for all EttleX operations
- [ ] Implement resource providers
- [ ] Add pre-configured prompts
- [ ] Support sampling (AI-initiated actions)
- [ ] Add progress reporting for long operations
- [ ] Support cancellation
- [ ] Add telemetry/logging integration
