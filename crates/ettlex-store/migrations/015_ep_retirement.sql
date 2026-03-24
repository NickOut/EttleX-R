-- Migration 015: EP Retirement and Schema Cleanup
--
-- Removes all EP-era schema artefacts:
--   1. Drop the `eps` table (EP model, retired by Slice 03)
--   2. Drop `facet_snapshots` (EP-era Phase 2 stub, never populated)
--   3. Drop `cas_blobs` (EP-era non-load-bearing index, superseded)
--   4. Rebuild `ettles` to remove dead columns:
--      parent_id, deleted, parent_ep_id, metadata, idx_ettles_parent_id
--
-- After this migration, the ettles table contains exactly:
--   id, title, why, what, how, reasoning_link_id, reasoning_link_type,
--   tombstoned_at, created_at, updated_at
--
-- FK enforcement is not active by default in SQLite. The rebuild uses the
-- standard SQLite table-rebuild pattern (create-copy-drop-rename).

-- 1. Drop eps table (indexes dropped automatically with the table)
DROP TABLE IF EXISTS eps;

-- 2. Drop facet_snapshots
DROP TABLE IF EXISTS facet_snapshots;

-- 3. Drop cas_blobs
DROP TABLE IF EXISTS cas_blobs;

-- 4. Rebuild ettles without dead columns
--    (parent_id, deleted, parent_ep_id, metadata are all removed)
CREATE TABLE ettles_new (
    id                  TEXT PRIMARY KEY,
    title               TEXT NOT NULL,
    why                 TEXT NOT NULL DEFAULT '',
    what                TEXT NOT NULL DEFAULT '',
    how                 TEXT NOT NULL DEFAULT '',
    reasoning_link_id   TEXT,
    reasoning_link_type TEXT,
    tombstoned_at       TEXT,
    created_at          TEXT NOT NULL,
    updated_at          TEXT NOT NULL,
    FOREIGN KEY (reasoning_link_id) REFERENCES ettles_new(id)
);

INSERT INTO ettles_new (id, title, why, what, how, reasoning_link_id, reasoning_link_type, tombstoned_at, created_at, updated_at)
SELECT id, title, why, what, how, reasoning_link_id, reasoning_link_type, tombstoned_at, created_at, updated_at
FROM ettles;

DROP TABLE ettles;

ALTER TABLE ettles_new RENAME TO ettles;
