# Slice WIP — agent-api-ettle

**Ettle ID:** ettle:019cf3a7-0b26-78f0-95b5-fab62af26b71
**Status:** COMPLETE

## Conformance Table

| SC | Layer(s) | Planned Test | RED Evidence | GREEN Evidence | Code Files | Doc Files | Doc Evidence | Status |
|----|----------|-------------|--------------|----------------|------------|-----------|--------------|--------|
| SC-01 | Agent API | test_agent_ettle_get_returns_full_record | compile fail (no impl) | 53/53 pass | `src/operations/ettle.rs` | `README.md`, `//!` on `agent_ettle_get` | make doc clean | DONE |
| SC-02 | Agent API | test_agent_ettle_get_returns_tombstoned | compile fail (no impl) | 53/53 pass | `src/operations/ettle.rs` | `//!` on `agent_ettle_get` | make doc clean | DONE |
| SC-03 | Agent API | test_agent_ettle_get_not_found | compile fail (no impl) | 53/53 pass | `src/operations/ettle.rs` | `//!` on `agent_ettle_get` | make doc clean | DONE |
| SC-04 | Agent API | test_agent_ettle_get_byte_identical | compile fail (no impl) | 53/53 pass | `src/operations/ettle.rs` | `//!` on `agent_ettle_get` | make doc clean | DONE |
| SC-05 | Agent API (logging) | test_agent_ettle_get_lifecycle_events | compile fail (no impl) | 53/53 pass | `src/operations/ettle.rs` | `//!` on `agent_ettle_get` (lifecycle note) | make doc clean | DONE |
| SC-06 | Agent API | test_agent_ettle_context_assembled | compile fail (no impl) | 53/53 pass | `src/operations/ettle.rs` | `README.md`, `//!` on `agent_ettle_context` | make doc clean | DONE |
| SC-07 | Agent API | test_agent_ettle_context_not_found | compile fail (no impl) | 53/53 pass | `src/operations/ettle.rs` | `//!` on `agent_ettle_context` | make doc clean | DONE |
| SC-08 | Agent API | test_agent_ettle_list_active_deterministic | compile fail (no impl) | 53/53 pass | `src/operations/ettle.rs` | `README.md`, `//!` on `agent_ettle_list` | make doc clean | DONE |
| SC-09 | Agent API | test_agent_ettle_list_pagination | compile fail (no impl) | 53/53 pass | `src/operations/ettle.rs` | `//!` on `agent_ettle_list` (cursor encoding) | make doc clean | DONE |
| SC-10 | Agent API | test_agent_ettle_list_limit_zero_rejected | compile fail (no impl) | 53/53 pass | `src/operations/ettle.rs` | `//!` on `agent_ettle_list` | make doc clean | DONE |
| SC-11 | Agent API | test_agent_ettle_create_title_only | compile fail (no impl) | 53/53 pass | `src/operations/ettle.rs` | `README.md`, `//!` on `agent_ettle_create` | make doc clean | DONE |
| SC-12 | Agent API | test_agent_ettle_create_empty_title | compile fail (no impl) | 53/53 pass | `src/operations/ettle.rs` | `//!` on `agent_ettle_create` | make doc clean | DONE |
| SC-13 | Agent API | test_agent_ettle_create_rejects_caller_id | compile fail (no impl) | 53/53 pass | `src/operations/ettle.rs` | `//!` on `agent_ettle_create` | make doc clean | DONE |
| SC-14 | Agent API | test_agent_ettle_create_link_without_type | compile fail (no impl) | 53/53 pass | `src/operations/ettle.rs` | `//!` on `agent_ettle_create` | make doc clean | DONE |
| SC-15 | Agent API | test_agent_ettle_update_fields | compile fail (no impl) | 53/53 pass | `src/operations/ettle.rs` | `README.md`, `//!` on `agent_ettle_update` | make doc clean | DONE |
| SC-16 | Agent API | test_agent_ettle_update_clears_reasoning_link | compile fail (no impl) | 53/53 pass | `src/operations/ettle.rs` | `//!` on `agent_ettle_update` (double-Option) | make doc clean | DONE |
| SC-17 | Agent API | test_agent_ettle_update_preserves_unspecified | compile fail (no impl) | 53/53 pass | `src/operations/ettle.rs` | `//!` on `agent_ettle_update` | make doc clean | DONE |
| SC-18 | Agent API | test_agent_ettle_update_rejects_tombstoned | compile fail (no impl) | 53/53 pass | `src/operations/ettle.rs` | `//!` on `agent_ettle_update` | make doc clean | DONE |
| SC-19 | Agent API | test_agent_ettle_tombstone_marks_inactive | compile fail (no impl) | 53/53 pass | `src/operations/ettle.rs` | `README.md`, `//!` on `agent_ettle_tombstone` | make doc clean | DONE |
| SC-20 | Agent API | test_agent_ettle_tombstone_rejects_active_dependants | compile fail (no impl) | 53/53 pass | `src/operations/ettle.rs` | `//!` on `agent_ettle_tombstone` | make doc clean | DONE |
| SC-21 | Agent API (OCC) | test_agent_occ_correct_version | compile fail (no impl) | 53/53 pass | `src/operations/ettle.rs` | `//!` on OCC param in write fns | make doc clean | DONE |
| SC-22 | Agent API (OCC) | test_agent_occ_wrong_version | compile fail (no impl) | 53/53 pass | `src/operations/ettle.rs` | `//!` on OCC param in write fns | make doc clean | DONE |
| SC-23 | Agent API | test_agent_relation_create_succeeds | compile fail (no impl) | 53/53 pass | `src/operations/relation.rs` | `README.md`, `//!` on `agent_relation_create` | make doc clean | DONE |
| SC-24 | Agent API | test_agent_relation_create_rejects_caller_id | compile fail (no impl) | 53/53 pass | `src/operations/relation.rs` | `//!` on `agent_relation_create` | make doc clean | DONE |
| SC-25 | Agent API | test_agent_relation_create_unknown_type | compile fail (no impl) | 53/53 pass | `src/operations/relation.rs` | `//!` on `agent_relation_create` | make doc clean | DONE |
| SC-26 | Agent API | test_agent_relation_create_self_referential | compile fail (no impl) | 53/53 pass | `src/operations/relation.rs` | `//!` on `agent_relation_create` | make doc clean | DONE |
| SC-27 | Agent API | test_agent_relation_create_missing_source | compile fail (no impl) | 53/53 pass | `src/operations/relation.rs` | `//!` on `agent_relation_create` | make doc clean | DONE |
| SC-28 | Agent API | test_agent_relation_create_tombstoned_source | compile fail (no impl) | 53/53 pass | `src/operations/relation.rs` | `//!` on `agent_relation_create` | make doc clean | DONE |
| SC-29 | Agent API | test_agent_relation_get_returns_full_record | compile fail (no impl) | 53/53 pass | `src/operations/relation.rs` | `README.md`, `//!` on `agent_relation_get` | make doc clean | DONE |
| SC-30 | Agent API | test_agent_relation_get_not_found | compile fail (no impl) | 53/53 pass | `src/operations/relation.rs` | `//!` on `agent_relation_get` | make doc clean | DONE |
| SC-31 | Agent API | test_agent_relation_list_by_source | compile fail (no impl) | 53/53 pass | `src/operations/relation.rs` | `README.md`, `//!` on `agent_relation_list` | make doc clean | DONE |
| SC-32 | Agent API | test_agent_relation_list_no_filter_fails | compile fail (no impl) | 53/53 pass | `src/operations/relation.rs` | `//!` on `agent_relation_list` | make doc clean | DONE |
| SC-33 | Agent API | test_agent_relation_list_ordering_deterministic | compile fail (no impl) | 53/53 pass | `src/operations/relation.rs` | `//!` on `agent_relation_list` | make doc clean | DONE |
| SC-34 | Agent API | test_agent_relation_tombstone_marks_inactive | compile fail (no impl) | 53/53 pass | `src/operations/relation.rs` | `README.md`, `//!` on `agent_relation_tombstone` | make doc clean | DONE |
| SC-35 | Agent API | test_agent_relation_tombstone_not_found | compile fail (no impl) | 53/53 pass | `src/operations/relation.rs` | `//!` on `agent_relation_tombstone` | make doc clean | DONE |
| SC-36 | Agent API | test_agent_relation_tombstone_already_tombstoned | compile fail (no impl) | 53/53 pass | `src/operations/relation.rs` | `//!` on `agent_relation_tombstone` | make doc clean | DONE |
| SC-37 | Agent API | test_agent_group_create_succeeds | compile fail (no impl) | 53/53 pass | `src/operations/group.rs` | `README.md`, `//!` on `agent_group_create` | make doc clean | DONE |
| SC-38 | Agent API | test_agent_group_create_empty_name_fails | compile fail (no impl) | 53/53 pass | `src/operations/group.rs` | `//!` on `agent_group_create` | make doc clean | DONE |
| SC-39 | Agent API | test_agent_group_get_returns_full_record | compile fail (no impl) | 53/53 pass | `src/operations/group.rs` | `README.md`, `//!` on `agent_group_get` | make doc clean | DONE |
| SC-40 | Agent API | test_agent_group_get_not_found | compile fail (no impl) | 53/53 pass | `src/operations/group.rs` | `//!` on `agent_group_get` | make doc clean | DONE |
| SC-41 | Agent API | test_agent_group_list_active_deterministic | compile fail (no impl) | 53/53 pass | `src/operations/group.rs` | `README.md`, `//!` on `agent_group_list` | make doc clean | DONE |
| SC-42 | Agent API | test_agent_group_member_add_succeeds | compile fail (no impl) | 53/53 pass | `src/operations/group.rs` | `README.md`, `//!` on `agent_group_member_add` | make doc clean | DONE |
| SC-43 | Agent API | test_agent_group_member_add_duplicate_fails | compile fail (no impl) | 53/53 pass | `src/operations/group.rs` | `//!` on `agent_group_member_add` | make doc clean | DONE |
| SC-44 | Agent API | test_agent_group_member_add_tombstoned_group_fails | compile fail (no impl) | 53/53 pass | `src/operations/group.rs` | `//!` on `agent_group_member_add` | make doc clean | DONE |
| SC-45 | Agent API | test_agent_group_member_remove_marks_tombstoned | compile fail (no impl) | 53/53 pass | `src/operations/group.rs` | `README.md`, `//!` on `agent_group_member_remove` | make doc clean | DONE |
| SC-46 | Agent API | test_agent_group_member_remove_not_found_fails | compile fail (no impl) | 53/53 pass | `src/operations/group.rs` | `//!` on `agent_group_member_remove` | make doc clean | DONE |
| SC-47 | Agent API | test_agent_group_member_list_by_group_id | compile fail (no impl) | 53/53 pass | `src/operations/group.rs` | `README.md`, `//!` on `agent_group_member_list` | make doc clean | DONE |
| SC-48 | Agent API | test_agent_group_member_list_no_filter_fails | compile fail (no impl) | 53/53 pass | `src/operations/group.rs` | `//!` on `agent_group_member_list` | make doc clean | DONE |
| SC-49 | Conformance | test_agent_api_only_memory_dep | compile fail (no impl) | 53/53 pass | `Cargo.toml` structure | `README.md` (dependency note) | make doc clean | DONE |
| SC-50 | Conformance | test_agent_api_writes_route_through_memory | compile fail (no impl) | 53/53 pass | `src/operations/*.rs` | `README.md` (routing invariant) | make doc clean | DONE |
| SC-51 | Conformance | test_agent_api_single_boundary_module | compile fail (no impl) | 53/53 pass | `src/boundary/mapping.rs` | `README.md` (boundary module note) | make doc clean | DONE |
| SC-52 | Conformance | test_agent_api_no_why_what_how_in_logs | compile fail (no impl) | 53/53 pass | `src/operations/*.rs` | — | make lint clean | DONE |
| SC-53 | Conformance | test_agent_api_no_apply_mcp_command | compile fail (no impl) | 53/53 pass | workspace-wide | — | make lint clean | DONE |
