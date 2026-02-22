-- Migration 001: Initial Schema
-- Creates the 6 core tables for EttleX Phase 1 Store Spine
-- (schema_version table is created by the migration runner)

-- Ettles
CREATE TABLE ettles (
    id TEXT PRIMARY KEY,
    title TEXT NOT NULL,
    parent_id TEXT,
    deleted INTEGER NOT NULL DEFAULT 0,
    created_at INTEGER NOT NULL,
    updated_at INTEGER NOT NULL,
    metadata TEXT,  -- JSON
    FOREIGN KEY (parent_id) REFERENCES ettles(id)
);

-- EPs
CREATE TABLE eps (
    id TEXT PRIMARY KEY,
    ettle_id TEXT NOT NULL,
    ordinal INTEGER NOT NULL,
    normative INTEGER NOT NULL,
    child_ettle_id TEXT,
    content_digest TEXT,  -- SHA256 if CAS-backed
    content_inline TEXT,  -- If not CAS-backed
    deleted INTEGER NOT NULL DEFAULT 0,
    created_at INTEGER NOT NULL,
    updated_at INTEGER NOT NULL,
    UNIQUE (ettle_id, ordinal),
    FOREIGN KEY (ettle_id) REFERENCES ettles(id),
    FOREIGN KEY (child_ettle_id) REFERENCES ettles(id)
);

-- Facet snapshots (schema stub for Phase 2)
CREATE TABLE facet_snapshots (
    id INTEGER PRIMARY KEY,
    snapshot_id TEXT NOT NULL,
    facet_kind TEXT NOT NULL,
    entity_id TEXT NOT NULL,
    digest TEXT NOT NULL,
    created_at INTEGER NOT NULL,
    UNIQUE (snapshot_id, facet_kind, entity_id)
);

-- Provenance events
CREATE TABLE provenance_events (
    id INTEGER PRIMARY KEY,
    kind TEXT NOT NULL,
    correlation_id TEXT NOT NULL,
    timestamp INTEGER NOT NULL,
    metadata TEXT  -- JSON
);

-- CAS blobs index (non-load-bearing)
CREATE TABLE cas_blobs (
    digest TEXT PRIMARY KEY,
    relpath TEXT NOT NULL,
    size_bytes INTEGER NOT NULL,
    kind TEXT NOT NULL,
    verified_at INTEGER,
    status TEXT
);

-- Indexes
CREATE INDEX idx_eps_ettle_id ON eps(ettle_id);
CREATE INDEX idx_eps_ordinal ON eps(ettle_id, ordinal);
CREATE INDEX idx_ettles_parent_id ON ettles(parent_id);
CREATE INDEX idx_provenance_correlation ON provenance_events(correlation_id);
CREATE INDEX idx_facet_snapshot ON facet_snapshots(snapshot_id);
