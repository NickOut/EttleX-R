-- Migration 004: Decision schema stubs
-- Canonical persistence for design decisions with portable evidence and EP/Ettle linkage

PRAGMA strict = ON;

-- decisions: canonical decision artefacts
CREATE TABLE IF NOT EXISTS decisions (
    decision_id TEXT PRIMARY KEY NOT NULL,
    title TEXT NOT NULL,
    status TEXT NOT NULL DEFAULT 'proposed',
    decision_text TEXT NOT NULL,
    rationale TEXT NOT NULL,
    alternatives_text TEXT,
    consequences_text TEXT,
    evidence_kind TEXT NOT NULL,
    evidence_excerpt TEXT,
    evidence_capture_id TEXT,
    evidence_file_path TEXT,
    evidence_hash TEXT,
    created_at INTEGER NOT NULL,  -- milliseconds since epoch
    updated_at INTEGER NOT NULL,  -- milliseconds since epoch
    tombstoned_at INTEGER         -- milliseconds since epoch (NULL if not tombstoned)
);

-- decision_evidence_items: portable conversation captures
CREATE TABLE IF NOT EXISTS decision_evidence_items (
    evidence_capture_id TEXT PRIMARY KEY NOT NULL,
    source TEXT NOT NULL,
    content TEXT NOT NULL,
    content_hash TEXT NOT NULL,
    created_at INTEGER NOT NULL  -- milliseconds since epoch
);

-- decision_links: many-to-many links between decisions and targets
CREATE TABLE IF NOT EXISTS decision_links (
    decision_id TEXT NOT NULL,
    target_kind TEXT NOT NULL,
    target_id TEXT NOT NULL,
    relation_kind TEXT NOT NULL,
    ordinal INTEGER NOT NULL DEFAULT 0,
    created_at INTEGER NOT NULL,  -- milliseconds since epoch
    tombstoned_at INTEGER,        -- milliseconds since epoch (NULL if not tombstoned)
    PRIMARY KEY (decision_id, target_kind, target_id, relation_kind),
    FOREIGN KEY (decision_id) REFERENCES decisions(decision_id) ON DELETE CASCADE
);

-- Indices for efficient queries
CREATE INDEX IF NOT EXISTS idx_decision_links_target ON decision_links(target_kind, target_id);
CREATE INDEX IF NOT EXISTS idx_decision_links_decision ON decision_links(decision_id);
CREATE INDEX IF NOT EXISTS idx_decisions_created_at ON decisions(created_at, decision_id);
CREATE INDEX IF NOT EXISTS idx_decision_links_created_at ON decision_links(created_at, decision_id);
