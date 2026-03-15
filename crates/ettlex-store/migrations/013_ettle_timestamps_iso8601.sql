-- Migration 013: Convert ettle timestamps from Unix epoch integers to ISO-8601 text.
-- Rows written by the original persist_ettle used chrono .timestamp() (i64).
-- New rows written by insert_ettle use chrono .to_rfc3339() (TEXT).
-- This migration normalises all existing integer timestamps to ISO-8601 UTC strings
-- matching the format produced by chrono::Utc::now().to_rfc3339().
--
-- Detection: a Unix epoch for any plausible EttleX timestamp is a 10-digit integer.
-- ISO-8601 strings are always longer than 15 characters, so the length guard is safe.

UPDATE ettles
SET
    created_at = REPLACE(datetime(CAST(created_at AS INTEGER), 'unixepoch'), ' ', 'T') || 'Z',
    updated_at = REPLACE(datetime(CAST(updated_at AS INTEGER), 'unixepoch'), ' ', 'T') || 'Z'
WHERE
    typeof(created_at) = 'integer'
    OR (typeof(created_at) = 'text' AND length(created_at) <= 12);
