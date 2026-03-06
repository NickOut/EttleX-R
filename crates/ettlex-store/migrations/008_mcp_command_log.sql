-- Migration 008: MCP command log for optimistic concurrency
-- Row count = state_version for MCP apply operations.

PRAGMA strict = ON;

CREATE TABLE IF NOT EXISTS mcp_command_log (
    id         INTEGER PRIMARY KEY AUTOINCREMENT,
    applied_at INTEGER NOT NULL  -- milliseconds since epoch
);
