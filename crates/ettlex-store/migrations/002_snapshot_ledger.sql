-- Migration 002: Snapshot Ledger Schema
-- Replaces stub facet_snapshots with proper snapshot ledger
--
-- This migration creates the snapshots table that stores immutable semantic
-- anchors for the EttleX system. Each snapshot represents a versioned state
-- of the refinement tree with computed digests for idempotency and verification.

DROP TABLE IF EXISTS facet_snapshots;

CREATE TABLE snapshots (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    snapshot_id TEXT NOT NULL UNIQUE,        -- UUIDv7 for temporal ordering
    root_ettle_id TEXT NOT NULL,             -- Root ettle for this snapshot
    manifest_digest TEXT NOT NULL,           -- Full manifest digest (includes created_at)
    semantic_manifest_digest TEXT NOT NULL,  -- Digest excluding created_at (for idempotency)
    created_at INTEGER NOT NULL,             -- Unix timestamp in milliseconds
    parent_snapshot_id TEXT,                 -- For linear history tracking
    policy_ref TEXT NOT NULL,                -- Policy reference (e.g., "policy/default@0")
    profile_ref TEXT NOT NULL,               -- Profile reference (e.g., "profile/default@0")
    status TEXT NOT NULL DEFAULT 'committed',-- Status: committed, draft (future)
    FOREIGN KEY (parent_snapshot_id) REFERENCES snapshots(snapshot_id)
);

-- Index for idempotency checks (find existing snapshot with same semantic digest)
CREATE INDEX idx_snapshots_semantic ON snapshots(semantic_manifest_digest);

-- Index for querying by root ettle
CREATE INDEX idx_snapshots_root ON snapshots(root_ettle_id);

-- Index for history traversal
CREATE INDEX idx_snapshots_parent ON snapshots(parent_snapshot_id);

-- Index for temporal queries
CREATE INDEX idx_snapshots_created ON snapshots(created_at);
