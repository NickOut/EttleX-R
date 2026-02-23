-- Migration 003: Constraints Schema
-- Adds support for family-agnostic constraints attached to EPs

-- Constraints table: stores all constraint instances
CREATE TABLE IF NOT EXISTS constraints (
    constraint_id TEXT PRIMARY KEY NOT NULL,
    family TEXT NOT NULL,
    kind TEXT NOT NULL,
    scope TEXT NOT NULL,
    payload_json TEXT NOT NULL,
    payload_digest TEXT NOT NULL,
    created_at INTEGER NOT NULL,
    updated_at INTEGER NOT NULL,
    deleted_at INTEGER
) STRICT;

-- Index for filtering by family
CREATE INDEX IF NOT EXISTS idx_constraints_family ON constraints(family);

-- Index for filtering out tombstoned constraints
CREATE INDEX IF NOT EXISTS idx_constraints_deleted_at ON constraints(deleted_at);

-- EP-to-Constraint attachment table: many-to-many relationship
CREATE TABLE IF NOT EXISTS ep_constraint_refs (
    ep_id TEXT NOT NULL,
    constraint_id TEXT NOT NULL,
    ordinal INTEGER NOT NULL,
    created_at INTEGER NOT NULL,
    PRIMARY KEY (ep_id, constraint_id),
    FOREIGN KEY (ep_id) REFERENCES eps(id) ON DELETE CASCADE,
    FOREIGN KEY (constraint_id) REFERENCES constraints(constraint_id) ON DELETE CASCADE
) STRICT;

-- Index for querying constraints by EP (and ordering by ordinal)
CREATE INDEX IF NOT EXISTS idx_ep_constraint_refs_ep_id ON ep_constraint_refs(ep_id, ordinal);

-- Index for reverse lookup: which EPs reference a constraint
CREATE INDEX IF NOT EXISTS idx_ep_constraint_refs_constraint_id ON ep_constraint_refs(constraint_id);
