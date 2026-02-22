# Functional-Boundary Refactor Traceability Matrix

This document maps Gherkin scenarios from the handoff specification to test implementations and production code for the functional-boundary refactor.

## Overview

**Refactor Goal:** Transition from mutation-based API to functional-boundary API with anchored deletion.

**Handoff Spec:** `handoff/EttleX_Phase0_5_Refactor_Functional_Boundary_and_Anchored_Delete_v2.md`

**Test Files:**

- `crates/ettlex-core/tests/apply_atomicity_tests.rs` (6 tests)
- `crates/ettlex-core/tests/apply_command_coverage_tests.rs` (11 tests)
- `crates/ettlex-core/tests/anchored_deletion_tests.rs` (8 tests)

**Production Modules:**

- `src/commands.rs` - Command enum
- `src/policy.rs` - AnchorPolicy trait
- `src/apply.rs` - apply() function
- `src/ops/store.rs` - Helper methods for testing

## Traceability Matrix

### Feature: Functional-Boundary Apply

| Scenario                                     | Test File                | Test Name                                     | Production Code   | Spec Section       |
| -------------------------------------------- | ------------------------ | --------------------------------------------- | ----------------- | ------------------ |
| Apply returns new valid state on success     | apply_atomicity_tests.rs | test_apply_returns_new_valid_state_on_success | apply.rs::apply() | Atomicity Contract |
| Apply fails without partial mutation         | apply_atomicity_tests.rs | test_apply_fails_without_partial_mutation     | apply.rs::apply() | Atomicity Contract |
| Apply surfaces typed errors and never panics | apply_atomicity_tests.rs | test_apply_surfaces_typed_errors_never_panics | apply.rs::apply() | Error Handling     |
| State ownership transfer                     | apply_atomicity_tests.rs | test_state_ownership_transfer                 | apply.rs::apply() | Move Semantics     |
| Apply chaining                               | apply_atomicity_tests.rs | test_apply_chaining                           | apply.rs::apply() | State Threading    |
| Apply error preserves original state         | apply_atomicity_tests.rs | test_apply_error_preserves_original_state     | apply.rs::apply() | Atomicity Contract |

### Feature: Command Coverage

| Scenario                        | Test File                       | Test Name                                               | Production Code                         | Spec Section      |
| ------------------------------- | ------------------------------- | ------------------------------------------------------- | --------------------------------------- | ----------------- |
| Ettle create with metadata      | apply_command_coverage_tests.rs | test_command_ettle_create_with_metadata                 | commands.rs::Command::EttleCreate       | Command Inventory |
| Ettle update                    | apply_command_coverage_tests.rs | test_command_ettle_update                               | commands.rs::Command::EttleUpdate       | Command Inventory |
| Ettle delete                    | apply_command_coverage_tests.rs | test_command_ettle_delete                               | commands.rs::Command::EttleDelete       | Command Inventory |
| EP create                       | apply_command_coverage_tests.rs | test_command_ep_create                                  | commands.rs::Command::EpCreate          | Command Inventory |
| EP update                       | apply_command_coverage_tests.rs | test_command_ep_update                                  | commands.rs::Command::EpUpdate          | Command Inventory |
| EP update partial               | apply_command_coverage_tests.rs | test_command_ep_update_partial                          | commands.rs::Command::EpUpdate          | Command Inventory |
| Refine link child               | apply_command_coverage_tests.rs | test_command_refine_link_child                          | commands.rs::Command::RefineLinkChild   | Command Inventory |
| Refine unlink child             | apply_command_coverage_tests.rs | test_command_refine_unlink_child                        | commands.rs::Command::RefineUnlinkChild | Command Inventory |
| Error: Invalid title            | apply_command_coverage_tests.rs | test_command_error_ettle_create_invalid_title           | apply.rs::apply()                       | Error Paths       |
| Error: Invalid WHAT             | apply_command_coverage_tests.rs | test_command_error_ep_create_invalid_what               | apply.rs::apply()                       | Error Paths       |
| Error: Child already has parent | apply_command_coverage_tests.rs | test_command_error_refine_link_child_already_has_parent | apply.rs::apply()                       | Error Paths       |

### Feature: Anchored Deletion

| Scenario                                   | Test File                  | Test Name                                       | Production Code                    | Spec Section         |
| ------------------------------------------ | -------------------------- | ----------------------------------------------- | ---------------------------------- | -------------------- |
| Hard delete removes EP completely          | anchored_deletion_tests.rs | test_hard_delete_removes_ep_completely          | apply.rs::hard_delete_ep()         | Hard Delete          |
| Tombstone preserves EP                     | anchored_deletion_tests.rs | test_tombstone_preserves_ep                     | apply.rs::apply() (tombstone path) | Tombstone Delete     |
| Hard delete maintains membership integrity | anchored_deletion_tests.rs | test_hard_delete_maintains_membership_integrity | apply.rs::hard_delete_ep()         | Membership Integrity |
| Policy controls deletion strategy          | anchored_deletion_tests.rs | test_policy_controls_deletion_strategy          | policy.rs::AnchorPolicy            | Policy Injection     |
| Hard delete cannot delete EP0              | anchored_deletion_tests.rs | test_hard_delete_cannot_delete_ep0              | apply.rs::hard_delete_ep()         | Safety Checks        |
| Hard delete prevents stranding child       | anchored_deletion_tests.rs | test_hard_delete_prevents_stranding_child       | apply.rs::hard_delete_ep()         | Safety Checks        |
| Hard delete allowed with multiple mappings | anchored_deletion_tests.rs | test_hard_delete_allowed_with_multiple_mappings | apply.rs::hard_delete_ep()         | Safety Checks        |
| Hard delete vs tombstone comparison        | anchored_deletion_tests.rs | test_hard_delete_vs_tombstone_comparison        | apply.rs::apply()                  | Policy Behavior      |

## Production Code Coverage

### New Modules

| Module       | Purpose                                                       | Lines | Tests                   |
| ------------ | ------------------------------------------------------------- | ----- | ----------------------- |
| commands.rs  | Command enum for all Phase 0.5 operations                     | ~110  | 3 unit + 11 integration |
| policy.rs    | AnchorPolicy trait + implementations                          | ~150  | 4 unit + 8 integration  |
| apply.rs     | Functional-boundary apply() function                          | ~250  | 8 unit + 25 integration |
| ops/store.rs | Helper methods for testing (ep_exists_in_storage, get_ep_raw) | +20   | All deletion tests      |

### Modified Modules

| Module    | Change            | Reason                                                                                      |
| --------- | ----------------- | ------------------------------------------------------------------------------------------- |
| errors.rs | +3 error variants | ApplyAtomicityBreach, HardDeleteForbiddenAnchoredEp, DeleteReferencesMissingEpInOwningEttle |
| lib.rs    | +3 module exports | Export commands, policy, apply modules                                                      |

## Test Coverage Summary

**Total New Tests:** 25

- Atomicity: 6 tests
- Command Coverage: 11 tests
- Anchored Deletion: 8 tests

**Code Coverage:**

- All new public APIs have rustdoc comments
- All new functions have unit tests
- All Gherkin scenarios have integration tests
- Zero compilation warnings
- All 147 tests pass (122 existing + 25 new)

## Handoff Spec Verification

| Handoff Requirement                 | Status      | Evidence                                                 |
| ----------------------------------- | ----------- | -------------------------------------------------------- |
| Apply function with move semantics  | ✅ Complete | apply.rs::apply(), test_state_ownership_transfer         |
| Command inventory for Phase 0.5 ops | ✅ Complete | commands.rs::Command enum (8 variants)                   |
| AnchorPolicy trait                  | ✅ Complete | policy.rs::AnchorPolicy + 2 implementations              |
| Hard delete for non-anchored EPs    | ✅ Complete | apply.rs::hard_delete_ep()                               |
| Tombstone for anchored EPs          | ✅ Complete | apply.rs::apply() (policy check)                         |
| Atomicity guarantee                 | ✅ Complete | All apply_atomicity_tests.rs                             |
| Never panics for invalid input      | ✅ Complete | test_apply_surfaces_typed_errors_never_panics            |
| Deterministic validation            | ✅ Complete | apply.rs calls validation::validate_tree()               |
| Backward compatibility              | ✅ Complete | All 122 existing tests pass, comprehensive_demo.rs works |
| New example demonstrating apply()   | ✅ Complete | examples/apply_demo.rs                                   |
| Documentation                       | ✅ Complete | README.md updated, rustdoc on all APIs                   |
| Traceability matrix                 | ✅ Complete | This document                                            |

## Examples

| Example               | Purpose                       | Status   |
| --------------------- | ----------------------------- | -------- |
| comprehensive_demo.rs | Mutation-based API (existing) | ✅ Works |
| apply_demo.rs         | Functional-boundary API (new) | ✅ Works |

## Error Taxonomy

| New Error                              | Purpose                                     | Tested In                                 |
| -------------------------------------- | ------------------------------------------- | ----------------------------------------- |
| ApplyAtomicityBreach                   | Internal assertion for apply() atomicity    | (Not intended for user-facing errors)     |
| HardDeleteForbiddenAnchoredEp          | Attempted hard delete of anchored EP        | (Currently unused - policy prevents this) |
| DeleteReferencesMissingEpInOwningEttle | Membership inconsistency during hard delete | apply.rs::hard_delete_ep()                |

## Acceptance Criteria

All acceptance criteria from handoff spec met:

- ✅ All 122 existing tests pass unchanged
- ✅ New tests cover all Gherkin scenarios
- ✅ Zero compilation warnings
- ✅ Formatting passes (`cargo fmt --all -- --check`)
- ✅ Clippy passes with no warnings
- ✅ Both examples run successfully
- ✅ README updated with new API
- ✅ All new public APIs have rustdoc
- ✅ Traceability matrix complete

---

**Implementation Date:** 2026-02-21
**Phase:** 0.5 Functional-Boundary Refactor
**Test Count:** 147 (122 existing + 25 new)
**Status:** ✅ Complete
