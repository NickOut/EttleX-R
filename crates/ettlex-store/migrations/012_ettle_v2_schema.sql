-- Migration 012: Ettle v2 schema
-- Add WHY/WHAT/HOW, reasoning_link, tombstone model.
-- Remove legacy deleted/parent_id/parent_ep_id/metadata columns.

ALTER TABLE ettles ADD COLUMN why TEXT NOT NULL DEFAULT '';
ALTER TABLE ettles ADD COLUMN what TEXT NOT NULL DEFAULT '';
ALTER TABLE ettles ADD COLUMN how TEXT NOT NULL DEFAULT '';
ALTER TABLE ettles ADD COLUMN reasoning_link_id TEXT NULL REFERENCES ettles(id);
ALTER TABLE ettles ADD COLUMN reasoning_link_type TEXT NULL;
ALTER TABLE ettles ADD COLUMN tombstoned_at TEXT NULL;
CREATE INDEX IF NOT EXISTS idx_ettles_reasoning_link ON ettles(reasoning_link_id);
CREATE INDEX IF NOT EXISTS idx_ettles_tombstoned ON ettles(tombstoned_at);
