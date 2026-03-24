# Slice WIP — slice-02b-mcp-read-surface-patch

**Ettle ID:** ettle:019d1d8d-d483-7941-a694-cd6924df612b
**Status:** COMPLETE

## Conformance Table

| SC | Layer(s) | Planned Test | RED Evidence | GREEN Evidence | Code Files | Doc Files | Doc Evidence | Status |
|----|----------|-------------|--------------|----------------|------------|-----------|--------------|--------|
| SC-S02b-01 | MCP | test_s02b_relation_get_returns_full_record | panic: relation_get not registered | 261/261 pass | `tools/relation.rs`, `server.rs`, `main.rs` | `tools/relation.rs` (//!), `crates/ettlex-mcp/README.md`, `docs/relations-groups.md` | make lint PASS, make doc PASS | DONE |
| SC-S02b-02 | MCP | test_s02b_relation_get_returns_tombstoned_record | panic: relation_get not registered | 261/261 pass | `tools/relation.rs` | same | same | DONE |
| SC-S02b-03 | MCP | test_s02b_relation_get_not_found | panic: relation_get not registered | 261/261 pass | `tools/relation.rs` | same | same | DONE |
| SC-S02b-04 | MCP (conformance) | test_s02b_relation_get_does_not_use_apply_path | panic: relation_get not registered | 261/261 pass | `tools/relation.rs` | same | same | DONE |
| SC-S02b-05 | MCP | test_s02b_relation_get_byte_identical_repeated | panic: relation_get not registered | 261/261 pass | `tools/relation.rs` | same | same | DONE |
| SC-S02b-06 | MCP | test_s02b_relation_get_error_logged_with_relation_id | panic: relation_get not registered | 261/261 pass | `tools/relation.rs` | same | same | DONE |
| SC-S02b-07 | MCP | test_s02b_relation_get_does_not_mutate_state | panic: relation_get not registered | 261/261 pass | `tools/relation.rs` | same | same | DONE |
| SC-S02b-08 | MCP | test_s02b_relation_get_fields_match_stored_record | panic: relation_get not registered | 261/261 pass | `tools/relation.rs` | same | same | DONE |
| SC-S02b-09 | MCP | test_s02b_relation_list_by_source_returns_matching | panic: relation_list not registered | 261/261 pass | `tools/relation.rs`, `server.rs`, `main.rs` | same | same | DONE |
| SC-S02b-10 | MCP | test_s02b_relation_list_by_target_returns_matching | panic: relation_list not registered | 261/261 pass | `tools/relation.rs` | same | same | DONE |
| SC-S02b-11 | MCP | test_s02b_relation_list_by_source_and_target_returns_intersection | panic: relation_list not registered | 261/261 pass | `tools/relation.rs` | same | same | DONE |
| SC-S02b-12 | MCP | test_s02b_relation_list_include_tombstoned | panic: relation_list not registered | 261/261 pass | `tools/relation.rs` | same | same | DONE |
| SC-S02b-13 | MCP | test_s02b_relation_list_no_filter_returns_invalid_input | panic: relation_list not registered | 261/261 pass | `tools/relation.rs` | same | same | DONE |
| SC-S02b-14 | MCP | test_s02b_relation_list_empty_when_no_match | panic: relation_list not registered | 261/261 pass | `tools/relation.rs` | same | same | DONE |
| SC-S02b-15 | MCP | test_s02b_relation_list_pagination_complete_non_overlapping | panic: relation_list not registered | 261/261 pass | `tools/relation.rs` | same | same | DONE |
| SC-S02b-16 | MCP (conformance) | test_s02b_relation_list_does_not_use_apply_path | panic: relation_list not registered | 261/261 pass | `tools/relation.rs` | same | same | DONE |
| SC-S02b-17 | MCP | test_s02b_relation_list_ordering_deterministic | panic: relation_list not registered | 261/261 pass | `tools/relation.rs` | same | same | DONE |
| SC-S02b-18 | MCP | test_s02b_relation_list_does_not_mutate_state | panic: relation_list not registered | 261/261 pass | `tools/relation.rs` | same | same | DONE |
| SC-S02b-19 | MCP | test_s02b_group_get_returns_full_record | panic: group_get not registered | 261/261 pass | `tools/group.rs`, `server.rs`, `main.rs` | `tools/group.rs` (//!), `crates/ettlex-mcp/README.md`, `docs/relations-groups.md` | make lint PASS, make doc PASS | DONE |
| SC-S02b-20 | MCP | test_s02b_group_get_returns_tombstoned_group | panic: group_get not registered | 261/261 pass | `tools/group.rs` | same | same | DONE |
| SC-S02b-21 | MCP | test_s02b_group_get_not_found | panic: group_get not registered | 261/261 pass | `tools/group.rs` | same | same | DONE |
| SC-S02b-22 | MCP (conformance) | test_s02b_group_get_does_not_use_apply_path | panic: group_get not registered | 261/261 pass | `tools/group.rs` | same | same | DONE |
| SC-S02b-23 | MCP | test_s02b_group_get_does_not_mutate_state | panic: group_get not registered | 261/261 pass | `tools/group.rs` | same | same | DONE |
| SC-S02b-24 | MCP | test_s02b_group_get_fields_match_stored_record | panic: group_get not registered | 261/261 pass | `tools/group.rs` | same | same | DONE |
| SC-S02b-25 | MCP | test_s02b_group_list_returns_active_groups | panic: group_list not registered | 261/261 pass | `tools/group.rs`, `server.rs`, `main.rs` | same | same | DONE |
| SC-S02b-26 | MCP | test_s02b_group_list_include_tombstoned | panic: group_list not registered | 261/261 pass | `tools/group.rs` | same | same | DONE |
| SC-S02b-27 | MCP | test_s02b_group_list_pagination_complete_non_overlapping | panic: group_list not registered | 261/261 pass | `tools/group.rs` | same | same | DONE |
| SC-S02b-28 | MCP | test_s02b_group_list_empty_when_no_groups | panic: group_list not registered | 261/261 pass | `tools/group.rs` | same | same | DONE |
| SC-S02b-29 | MCP | test_s02b_group_list_ordering_deterministic | panic: group_list not registered | 261/261 pass | `tools/group.rs` | same | same | DONE |
| SC-S02b-30 | MCP (conformance) | test_s02b_group_list_does_not_mutate_state | panic: group_list not registered | 261/261 pass | `tools/group.rs` | same | same | DONE |
| SC-S02b-31 | MCP | test_s02b_group_member_list_by_group_id | panic: group_member_list not registered | 261/261 pass | `tools/group.rs`, `server.rs`, `main.rs` | same | same | DONE |
| SC-S02b-32 | Store + MCP | test_s02b_group_member_list_by_ettle_id | panic: group_member_list not registered | 261/261 pass | `tools/group.rs`, `sqlite_repo.rs` (`list_group_members_by_filter`) | same | same | DONE |
| SC-S02b-33 | MCP | test_s02b_group_member_list_by_group_and_ettle_intersection | panic: group_member_list not registered | 261/261 pass | `tools/group.rs` | same | same | DONE |
| SC-S02b-34 | MCP | test_s02b_group_member_list_include_tombstoned | panic: group_member_list not registered | 261/261 pass | `tools/group.rs` | same | same | DONE |
| SC-S02b-35 | MCP | test_s02b_group_member_list_no_filter_returns_invalid_input | panic: group_member_list not registered | 261/261 pass | `tools/group.rs` | same | same | DONE |
| SC-S02b-36 | MCP | test_s02b_group_member_list_empty_when_no_match | panic: group_member_list not registered | 261/261 pass | `tools/group.rs` | same | same | DONE |
| SC-S02b-37 | MCP | test_s02b_group_member_list_pagination_complete_non_overlapping | panic: group_member_list not registered | 261/261 pass | `tools/group.rs` | same | same | DONE |
| SC-S02b-38 | MCP (conformance) | test_s02b_group_member_list_does_not_use_apply_path | panic: group_member_list not registered | 261/261 pass | `tools/group.rs` | same | same | DONE |
| SC-S02b-39 | MCP | test_s02b_group_member_list_ordering_deterministic | panic: group_member_list not registered | 261/261 pass | `tools/group.rs` | same | same | DONE |
| SC-S02b-40 | MCP (conformance) | test_s02b_all_five_tools_registered_at_startup | panic: relation_get not registered | 261/261 pass | `server.rs`, `main.rs`, `tools/mod.rs` | same | same | DONE |
| SC-S02b-41 | MCP (conformance) | test_s02b_group_member_list_does_not_mutate_state | panic: group_member_list not registered | 261/261 pass | `tools/group.rs` | same | same | DONE |
| SC-S02b-42 | MCP (conformance) | test_s02b_no_new_tool_invokes_write_command_path | panic: relation_get not registered | 261/261 pass | `tools/relation.rs`, `tools/group.rs` | same | same | DONE |
