# Schema Cleanup Notes

This file tracks columns, tables, and indexes that are known dead weight from the
original schema. These should be removed in a single cleanup migration once the
vertical slice programme has stabilised and all slice tests pass cleanly.

**Trigger for cleanup:** `make test-slice` passes across all registered slices with
zero failures, and the pre-authorised failure list in `slice_registry.toml` is empty
or near-empty. At that point, nothing should still reference these structures and a
cleanup migration can be written with confidence.

---

## ettles table

### Columns to remove

| Column | Introduced | Reason dead |
|---|---|---|
| `parent_id` | 001_initial_schema | Superseded by `reasoning_link_id` (Slice 01). Encoded Ettle-level parent as a direct FK; the new model uses `reasoning_link_id` + `reasoning_link_type` to carry typed parent relationships. |
| `deleted` | 001_initial_schema | Superseded by `tombstoned_at` (Slice 01). Boolean deletion flag replaced by nullable timestamp for consistency and auditability. |
| `parent_ep_id` | 009_parent_ep_id | Introduced to bridge the old EP-to-Ettle link model. Conflated two independent link trees. Superseded by the Reasoning Link model. |
| `metadata` | 001_initial_schema | JSON blob with no defined schema or current usage. No slice has introduced a replacement; confirm dead before removing. |

### Indexes to remove

| Index | Reason dead |
|---|---|
| `idx_ettles_parent_id` | References `parent_id` column being removed. |

### Columns being added by Slice 01 (not dead weight — record for reference)

- `why`, `what`, `how` (TEXT NOT NULL DEFAULT '')
- `reasoning_link_id` (TEXT NULL, FK → ettles.id)
- `reasoning_link_type` (TEXT NULL)
- `tombstoned_at` (TEXT NULL, ISO-8601)

---

## eps table

The eps table is a legacy construct being phased out entirely. EPs are not part of
the new conceptual model; WHY/WHAT/HOW content is stored directly on the Ettle record.

### Full table removal (deferred)

The eps table and all associated structures should be removed once all slices that
previously depended on EP content have been replaced. Do not remove until confirmed
that no slice, query, or test still references eps.

### Columns known dead within the eps table

| Column | Introduced | Reason dead |
|---|---|---|
| `ettle_id` | 001_initial_schema | To be renamed `containing_ettle` per Schema Migration 012 (on hold). Superseded when eps table is removed. |
| `child_ettle_id` | 001_initial_schema | Encoded EP-to-child-Ettle link. Superseded by Reasoning Link on Ettle record. |
| `content_digest` | 001_initial_schema | Dual-storage pattern (CAS + inline). Superseded when eps table is removed. |
| `content_inline` | 001_initial_schema | Dual-storage pattern. Superseded when eps table is removed. |
| `deleted` | 001_initial_schema | Boolean deletion flag; superseded by tombstoned_at pattern. Superseded when eps table is removed. |
| `parent_ep_id` | 009_parent_ep_id | Cross-Ettle EP refinement link from the old model. Superseded when eps table is removed. |
| `title` | 011_eps_title | Optional EP title. Superseded when eps table is removed. |

### Associated tables to remove with eps

| Table | Reason |
|---|---|
| `ep_constraint_refs` | Links constraints to EPs. Superseded once Constraint Association model targets Ettles directly. |

### Indexes to remove with eps

| Index | Reason |
|---|---|
| `idx_eps_ettle_id` | References eps table. |
| `idx_eps_ordinal` | References eps table. |

---

## Migrations to consolidate (long-term)

Once the schema is stable, consider consolidating the migration history into a single
baseline migration for fresh installs, keeping the incremental migration files for
upgrade paths from existing databases. This is a separate concern from the column
cleanup above and should be considered only after the cleanup migration is confirmed
clean.

Migrations that introduced now-dead structures:
- `009_parent_ep_id.sql` — adds `ettles.parent_ep_id` (dead)
- `010_backfill_parent_ep_id.sql` — backfills `ettles.parent_ep_id` from `eps.child_ettle_id` (dead)
- `011_eps_title.sql` — adds `eps.title` (dead when eps removed)

---

## Schema Migration 012 (on hold)

Schema Migration 012 (`ettle:019ccf15-e2b1-7e33-9794-bf2cf0704178`) is on hold pending
model redesign completion. It partially addressed the structural model (renaming columns,
restructuring EP parent links). Its intent is superseded by the vertical slice programme.
The pre-authorised failure tests it introduced (EFR-01 through EFR-14) overlap with the
Slice 01 pre-authorised failure registry. Do not apply Migration 012 independently;
its changes are subsumed by the slice migrations.

---

## Cleanup migration checklist (to be completed when triggered)

- [ ] Confirm no code references `ettles.parent_id`
- [ ] Confirm no code references `ettles.deleted`
- [ ] Confirm no code references `ettles.parent_ep_id`
- [ ] Confirm no code references `ettles.metadata` (or document new usage)
- [ ] Confirm no code references `eps` table at all
- [ ] Confirm no code references `ep_constraint_refs` table
- [ ] Write cleanup migration (single file, e.g. `NNN_schema_cleanup.sql`)
- [ ] Run `make test-slice` — must pass with zero failures
- [ ] Run `make test` — confirm no new failures beyond pre-authorised list
- [ ] Update this file to mark all items resolved
