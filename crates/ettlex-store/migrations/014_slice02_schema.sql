-- Migration 014: Slice 02 — Relations, Groups, Relation Type Registry
--
-- 1. Rename mcp_command_log → command_log
-- 2. Migrate provenance_events.timestamp (INTEGER epoch-ms) → occurred_at TEXT ISO-8601
-- 3. Migrate command_log.applied_at (INTEGER epoch-ms) → TEXT ISO-8601
-- 4. Add relation_type_registry table and seed 4 entries
-- 5. Add relations table
-- 6. Add groups table
-- 7. Add group_members table
-- 8. Drop legacy constraint tables

-- 1. Rename mcp_command_log to command_log
ALTER TABLE mcp_command_log RENAME TO command_log;

-- 2. Migrate provenance_events: rename column timestamp → occurred_at, convert INTEGER to ISO-8601
ALTER TABLE provenance_events RENAME COLUMN timestamp TO occurred_at;
UPDATE provenance_events
SET occurred_at = REPLACE(datetime(CAST(occurred_at AS INTEGER) / 1000, 'unixepoch'), ' ', 'T') || 'Z'
WHERE typeof(occurred_at) = 'integer'
   OR (typeof(occurred_at) = 'text' AND length(occurred_at) <= 15);

-- 3. Migrate command_log.applied_at INTEGER → TEXT ISO-8601
UPDATE command_log
SET applied_at = REPLACE(datetime(CAST(applied_at AS INTEGER) / 1000, 'unixepoch'), ' ', 'T') || 'Z'
WHERE typeof(applied_at) = 'integer'
   OR (typeof(applied_at) = 'text' AND length(applied_at) <= 15);

-- 4. Relation Type Registry
CREATE TABLE relation_type_registry (
    relation_type  TEXT PRIMARY KEY,
    properties_json TEXT NOT NULL DEFAULT '{}',
    created_at     TEXT NOT NULL,
    tombstoned_at  TEXT
);

INSERT INTO relation_type_registry (relation_type, properties_json, created_at) VALUES
('refinement',    '{"traversal_eligible":true,"cycle_check":false,"cascade_tombstone_default":false,"cardinality":"many","expected_fields":[]}',
                  strftime('%Y-%m-%dT%H:%M:%SZ', 'now')),
('option',        '{"traversal_eligible":false,"cycle_check":false,"cascade_tombstone_default":false,"cardinality":"many","expected_fields":[]}',
                  strftime('%Y-%m-%dT%H:%M:%SZ', 'now')),
('semantic_peer', '{"traversal_eligible":false,"cycle_check":false,"cascade_tombstone_default":false,"cardinality":"many","expected_fields":[]}',
                  strftime('%Y-%m-%dT%H:%M:%SZ', 'now')),
('constraint',    '{"traversal_eligible":false,"cycle_check":true,"cascade_tombstone_default":false,"cardinality":"many","expected_fields":["family","kind","scope","traversal_leaf_id","context_vector_id"]}',
                  strftime('%Y-%m-%dT%H:%M:%SZ', 'now'));

-- 5. Relations
CREATE TABLE relations (
    id              TEXT PRIMARY KEY,
    source_ettle_id TEXT NOT NULL REFERENCES ettles(id),
    target_ettle_id TEXT NOT NULL REFERENCES ettles(id),
    relation_type   TEXT NOT NULL REFERENCES relation_type_registry(relation_type),
    properties_json TEXT NOT NULL DEFAULT '{}',
    created_at      TEXT NOT NULL,
    tombstoned_at   TEXT
);
CREATE INDEX idx_relations_source ON relations(source_ettle_id) WHERE tombstoned_at IS NULL;
CREATE INDEX idx_relations_target ON relations(target_ettle_id) WHERE tombstoned_at IS NULL;
CREATE INDEX idx_relations_type   ON relations(relation_type)   WHERE tombstoned_at IS NULL;

-- 6. Groups
CREATE TABLE groups (
    id            TEXT PRIMARY KEY,
    name          TEXT NOT NULL,
    created_at    TEXT NOT NULL,
    tombstoned_at TEXT
);

-- 7. Group Members
CREATE TABLE group_members (
    id            TEXT PRIMARY KEY,
    group_id      TEXT NOT NULL REFERENCES groups(id),
    ettle_id      TEXT NOT NULL REFERENCES ettles(id),
    created_at    TEXT NOT NULL,
    tombstoned_at TEXT
);
CREATE INDEX idx_group_members_group ON group_members(group_id) WHERE tombstoned_at IS NULL;
CREATE INDEX idx_group_members_ettle ON group_members(ettle_id) WHERE tombstoned_at IS NULL;

-- 8. Drop legacy constraint tables
DROP TABLE IF EXISTS ep_constraint_refs;
DROP TABLE IF EXISTS constraint_associations;
DROP TABLE IF EXISTS constraint_set_members;
DROP TABLE IF EXISTS constraint_sets;
DROP TABLE IF EXISTS constraints;
