-- Migration 011: Add title column to eps table
--
-- The eps table gains an optional title TEXT field.
-- Nullable (no NOT NULL constraint) so that existing rows
-- continue to work without migration data-backfill.
ALTER TABLE eps ADD COLUMN title TEXT;
