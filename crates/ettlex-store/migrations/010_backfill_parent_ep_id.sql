-- Migration 010: Backfill parent_ep_id from eps.child_ettle_id
--
-- Migration 009 added the parent_ep_id column (NULL for all existing rows).
-- This migration populates it for ettles that already had a parent-child
-- relationship recorded via the old ep.child_ettle_id field.
--
-- For each child ettle (parent_id IS NOT NULL, parent_ep_id IS NULL), find
-- the non-deleted EP whose child_ettle_id points to it.  In the pre-fan-out
-- model each child was pointed to by exactly one EP, so the subquery returns
-- at most one row.

UPDATE ettles
SET parent_ep_id = (
    SELECT eps.id
    FROM eps
    WHERE eps.child_ettle_id = ettles.id
      AND eps.deleted = 0
    LIMIT 1
)
WHERE parent_id IS NOT NULL
  AND parent_ep_id IS NULL;
