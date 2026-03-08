# Completion Report: ep:mcp_thin_slice ordinal 2 — Wire Missing MCP Tools

**EP:** `ep:mcp_thin_slice:2`
**Date completed:** 2026-03-08
**Classification:** B — Behavioural Extension (17 new MCP tools delegating to existing EngineQuery surface)

---

## Summary

Wired 17 `EngineQuery` variants to MCP dispatch plus `policy_export` (already an EngineQuery variant
needing a handler). All EngineQuery variants were pre-existing; this EP provides the MCP transport
layer (param parsing, JSON serialisation, routing).

---

## Production Changes

### New tool modules (`ettlex-mcp/src/tools/`)

| Module | Handler functions |
|---|---|
| `state.rs` (new) | `handle_state_get_version` |
| `constraint.rs` (new) | `handle_constraint_get`, `handle_constraint_list_by_family` |
| `decision.rs` (new) | `handle_decision_get`, `handle_decision_list`, `handle_decision_list_by_target` |
| `ept.rs` (new) | `handle_ept_compute`, `handle_ept_compute_decision_context` |

### Extended tool modules

| Module | New handlers |
|---|---|
| `ep.rs` | `handle_ep_list_children`, `handle_ep_list_parents`, `handle_ep_list_constraints`, `handle_ep_list_decisions` |
| `ettle.rs` | `handle_ettle_list_decisions` |
| `snapshot.rs` | `handle_manifest_get_by_digest` |
| `profile.rs` | `handle_profile_resolve` |
| `approval.rs` | `handle_approval_list` |
| `policy.rs` | `handle_policy_export` |

### `tools/mod.rs`
Added: `pub mod constraint;`, `pub mod decision;`, `pub mod ept;`, `pub mod state;`

### `server.rs` — new dispatch arms

```
ep_list_children         → ep::handle_ep_list_children
ep_list_parents          → ep::handle_ep_list_parents
ep_list_constraints      → ep::handle_ep_list_constraints
ep_list_decisions        → ep::handle_ep_list_decisions
ettle_list_decisions     → ettle::handle_ettle_list_decisions
constraint_get           → constraint::handle_constraint_get
constraint_list_by_family → constraint::handle_constraint_list_by_family
decision_get             → decision::handle_decision_get
decision_list            → decision::handle_decision_list
decision_list_by_target  → decision::handle_decision_list_by_target
ept_compute              → ept::handle_ept_compute
ept_compute_decision_context → ept::handle_ept_compute_decision_context
state_get_version        → state::handle_state_get_version
manifest_get_by_digest   → snapshot::handle_manifest_get_by_digest
policy_export            → policy::handle_policy_export
profile_resolve          → profile::handle_profile_resolve
approval_list            → approval::handle_approval_list
```

---

## Tool → EngineQuery Mapping

| MCP Tool | EngineQuery Variant | Params |
|---|---|---|
| `state_get_version` | `StateGetVersion` | none |
| `ep_list_children` | `EpListChildren { ep_id }` | ep_id: String |
| `ep_list_parents` | `EpListParents { ep_id }` | ep_id: String |
| `ep_list_constraints` | `EpListConstraints { ep_id }` | ep_id: String |
| `constraint_get` | `ConstraintGet { constraint_id }` | constraint_id: String |
| `constraint_list_by_family` | `ConstraintListByFamily { family, include_tombstoned }` | family: String, include_tombstoned?: bool |
| `decision_get` | `DecisionGet { decision_id }` | decision_id: String |
| `decision_list` | `DecisionList(ListOptions)` | limit?: u64, cursor?: String |
| `decision_list_by_target` | `DecisionListByTarget { target_kind, target_id, include_tombstoned }` | target_kind, target_id, include_tombstoned?: bool |
| `ep_list_decisions` | `EpListDecisions { ep_id, include_ancestors }` | ep_id, include_ancestors?: bool |
| `ettle_list_decisions` | `EttleListDecisions { ettle_id, include_eps, include_ancestors }` | ettle_id, include_eps?, include_ancestors? |
| `ept_compute_decision_context` | `EptComputeDecisionContext { leaf_ep_id }` | leaf_ep_id: String |
| `manifest_get_by_digest` | `ManifestGetByDigest { manifest_digest }` | manifest_digest: String |
| `ept_compute` | `EptCompute { leaf_ep_id }` | leaf_ep_id: String |
| `profile_resolve` | `ProfileResolve { profile_ref: Option<String> }` | profile_ref?: String |
| `approval_list` | `ApprovalList(ListOptions)` | limit?: u64, cursor?: String |
| `policy_export` | `PolicyExport { policy_ref, export_kind }` | policy_ref: String, export_kind: String |

---

## Tests (`ettlex-mcp/tests/mcp_missing_tools_tests.rs`)

27 tests total: 17 happy path + 8 error + 2 invariant.

### Happy Path (S-MT-HP-1 to S-MT-HP-17)
One test per tool, each seeding minimal required data and asserting non-error response with expected shape.

### Error Tests (S-MT-ERR-1 to S-MT-ERR-8)
| ID | Test | Expected Code |
|---|---|---|
| S-MT-ERR-1 | ep_list_children missing ep_id param | `InvalidInput` |
| S-MT-ERR-2 | constraint_get with missing constraint | `NotFound` |
| S-MT-ERR-3 | decision_get with missing decision | `NotFound` |
| S-MT-ERR-4 | manifest_get_by_digest with bad digest | `MissingBlob` |
| S-MT-ERR-5 | ept_compute with missing EP | `NotFound` |
| S-MT-ERR-6 | profile_resolve with missing ref | `ProfileNotFound` |
| S-MT-ERR-7 | policy_export nonexistent policy_ref | `PolicyNotFound` |
| S-MT-ERR-8 | policy_export unknown export_kind | `PolicyExportFailed` |

### Invariant Tests
| ID | Test |
|---|---|
| S-MT-INV-1 | All 14 query tools called → state_version unchanged |
| S-MT-INV-2 | state_get_version returns V+1 after any Apply command |

---

## Formally Deferred Constraints

| Constraint | Rationale | Location |
|---|---|---|
| ep_list_parents RefinementIntegrityViolation | Requires corrupted DB state; aligned with existing `#[ignore]` guard in suite | `// DEFERRED:` comment in test file |
| ept_compute EptAmbiguous | BTreeMap determinism makes this unreachable in Phase 1; existing `#[ignore]` guard | `// DEFERRED:` comment in test file |
| Scale/time-budget tests | Performance obligations deferred to dedicated load test sprint | `// DEFERRED:` comment in test file |

---

## Acceptance Gates

- `make lint` — PASS
- `make test` — 841/841 PASS, 3 skipped
- `make coverage-check` — 87% ≥ 80% threshold PASS
- `make doc` — PASS
