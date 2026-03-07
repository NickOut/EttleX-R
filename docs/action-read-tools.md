# Action Read Tools (`ep:action_read_tools:0`)

Read-only query surface exposed through `apply_engine_query`.
All queries accept `&Connection` (not `&mut`) and `&FsStore`.
None of these operations write to the database or CAS.

---

## Entry Point

```rust
use ettlex_engine::commands::engine_query::{apply_engine_query, EngineQuery, EngineQueryResult};

let result = apply_engine_query(query, &conn, &cas, policy_provider)?;
// policy_provider: Option<&dyn PolicyProvider> — pass Some(&provider) for policy queries, None otherwise
```

---

## Pagination

All list queries accept `ListOptions`:

```rust
pub struct ListOptions {
    pub limit: Option<usize>,          // defaults to DEFAULT_LIST_LIMIT (100)
    pub cursor: Option<String>,        // opaque base64-encoded sort key
    pub prefix_filter: Option<String>, // filter by ID/ref prefix
    pub title_contains: Option<String>,// substring filter on title
}
```

Results return `Page<T>`:

```rust
pub struct Page<T> {
    pub items: Vec<T>,
    pub cursor: Option<String>,  // pass to next request; None = last page
    pub has_more: bool,
}
```

**Cursor semantics**:

- Cursors are opaque base64 strings encoding the sort key of the last returned item.
- Cursors are stable across reads (append-only schema, sorted by ID or timestamp).
- Callers must not parse or construct cursors manually.

---

## Query Reference

### State

#### `StateGetVersion`

Returns the current state version and semantic head digest.

```rust
EngineQuery::StateGetVersion
// → EngineQueryResult::StateGetVersion(StateVersionResult {
//       state_version: u64,
//       semantic_head_digest: Option<String>,
//   })
```

---

### Ettle Queries

#### `EttleGet`

Returns Ettle metadata and the list of EP IDs for the Ettle.

```rust
EngineQuery::EttleGet { ettle_id: String }
// → EngineQueryResult::EttleGet(EttleGetResult {
//       ettle: Ettle,
//       ep_ids: Vec<String>,
//   })
// Errors: NotFound
```

#### `EttleList`

Paginated list of all Ettles, ordered by ID.

```rust
EngineQuery::EttleList(ListOptions { limit: Some(50), .. })
// → EngineQueryResult::EttleList(Page<Ettle>)
```

Supports `prefix_filter` (matches Ettle ID prefix) and `title_contains` (case-insensitive substring).

#### `EttleListEps`

All EPs belonging to an Ettle, ordered by `ordinal`.

```rust
EngineQuery::EttleListEps { ettle_id: String }
// → EngineQueryResult::EttleListEps(Vec<Ep>)
// Errors: NotFound (Ettle missing)
```

---

### EP Queries

#### `EpGet`

```rust
EngineQuery::EpGet { ep_id: String }
// → EngineQueryResult::EpGet(Ep)
// Errors: NotFound
```

#### `EpListChildren`

EPs that live inside the child Ettles of this EP (fan-out refinement).
Children are identified via `ettle.parent_ep_id == ep_id` (the authoritative join
field). The legacy `ep.child_ettle_id` is kept for backward compatibility but is
no longer the source of truth.

```rust
EngineQuery::EpListChildren { ep_id: String }
// → EngineQueryResult::EpListChildren(Vec<Ep>)
// Returns empty vec if no Ettle has parent_ep_id pointing to this EP.
```

#### `EpListParents`

EPs whose Ettle is the structural parent of this EP's Ettle.
Traverses up one level via `ettle.parent_id`.

```rust
EngineQuery::EpListParents { ep_id: String }
// → EngineQueryResult::EpListParents(Vec<Ep>)
// Errors: RefinementIntegrityViolation (more than one structural parent)
```

#### `EpListConstraints`

Constraints attached to an EP, ordered by `ep_constraint_refs.ordinal`.

```rust
EngineQuery::EpListConstraints { ep_id: String }
// → EngineQueryResult::EpListConstraints(Vec<Constraint>)
```

#### `EpListDecisions`

Decisions for an EP, optionally including decisions attached to ancestor Ettles.

```rust
EngineQuery::EpListDecisions { ep_id: String, include_ancestors: bool }
// → EngineQueryResult::EpListDecisions(Vec<Decision>)
```

When `include_ancestors = true`, walks up `ettle.parent_id` and accumulates
decisions linked to each ancestor Ettle (`target_kind = "ettle"`).

---

### Constraint Queries

#### `ConstraintGet`

```rust
EngineQuery::ConstraintGet { constraint_id: String }
// → EngineQueryResult::ConstraintGet(Constraint)
// Errors: NotFound
```

#### `ConstraintListByFamily`

All constraints in a family, optionally including tombstoned constraints.

```rust
EngineQuery::ConstraintListByFamily { family: String, include_tombstoned: bool }
// → EngineQueryResult::ConstraintListByFamily(Vec<Constraint>)
```

---

### Decision Queries

#### `DecisionGet`

```rust
EngineQuery::DecisionGet { decision_id: String }
// → EngineQueryResult::DecisionGet(Decision)
// Errors: NotFound
```

#### `DecisionList`

Paginated list of all decisions, ordered by `decision_id`.

```rust
EngineQuery::DecisionList(ListOptions::default())
// → EngineQueryResult::DecisionList(Page<Decision>)
```

#### `DecisionListByTarget`

All decisions linked to a specific target entity.

```rust
EngineQuery::DecisionListByTarget {
    target_kind: String,       // e.g. "ep", "ettle"
    target_id: String,
    include_tombstoned: bool,
}
// → EngineQueryResult::DecisionListByTarget(Vec<Decision>)
```

#### `EttleListDecisions`

Decisions for an Ettle, optionally expanding into its EPs and ancestor Ettles.

```rust
EngineQuery::EttleListDecisions {
    ettle_id: String,
    include_eps: bool,
    include_ancestors: bool,
}
// → EngineQueryResult::EttleListDecisions(DecisionContextResult)
```

`DecisionContextResult.by_ep` maps each EP ID to its decisions.
`DecisionContextResult.all_for_leaf` aggregates all decisions for the leaf EP.

#### `EptComputeDecisionContext`

Full decision context for every EP in the EPT chain of a leaf EP.

```rust
EngineQuery::EptComputeDecisionContext { leaf_ep_id: String }
// → EngineQueryResult::EptComputeDecisionContext(DecisionContextResult)
// Errors: NotFound (leaf EP missing), EptAmbiguous (ambiguous EPT)
```

---

### Snapshot / Manifest Queries

#### `SnapshotGet`

```rust
EngineQuery::SnapshotGet { snapshot_id: String }
// → EngineQueryResult::SnapshotGet(SnapshotRow)
// Errors: NotFound
```

`SnapshotRow` contains: `snapshot_id`, `root_ettle_id`, `manifest_digest`,
`semantic_manifest_digest`, `created_at`, `parent_snapshot_id`, `policy_ref`,
`profile_ref`, `status`.

#### `SnapshotList`

All snapshot rows, optionally filtered by root Ettle.

```rust
EngineQuery::SnapshotList { ettle_id: Option<String> }
// → EngineQueryResult::SnapshotList(Vec<SnapshotRow>)
```

Results ordered by `created_at, snapshot_id` ascending.

#### `SnapshotGetHead`

Manifest digest of the most recently committed snapshot for a given root Ettle.
Returns `null` if no committed snapshot exists yet.

```rust
EngineQuery::SnapshotGetHead { realised_ettle_id: String }
// → EngineQueryResult::SnapshotGetHead(Option<String>)
// Errors: Persistence (DB failure)
```

The returned value is the `manifest_digest` string (SHA-256 hex), suitable for
use as `expected_head` in a subsequent `SnapshotCommit` command to implement
optimistic concurrency.

#### `ManifestGetBySnapshot`

Manifest bytes + both digests for a snapshot.

```rust
EngineQuery::ManifestGetBySnapshot { snapshot_id: String }
// → EngineQueryResult::ManifestGet(ManifestGetResult {
//       snapshot_id, manifest_digest, semantic_manifest_digest, manifest_bytes
//   })
// Errors: NotFound (no snapshot row), MissingBlob (snapshot row exists but CAS blob gone)
```

#### `ManifestGetByDigest`

Fetch manifest bytes directly from CAS by digest (no snapshot row lookup).

```rust
EngineQuery::ManifestGetByDigest { manifest_digest: String }
// → EngineQueryResult::ManifestGet(ManifestGetResult)
// Errors: MissingBlob
```

---

### EPT Queries

#### `EptCompute`

Compute the EPT (Ettle Projection Tree) for a leaf EP. Returns the ordered list of
EP IDs in the chain and a deterministic `ept_digest`.

```rust
EngineQuery::EptCompute { leaf_ep_id: String }
// → EngineQueryResult::EptCompute(EptComputeResult {
//       leaf_ep_id, ept_ep_ids: Vec<String>, ept_digest: String
//   })
// Errors: NotFound, EptAmbiguous (guarded, unreachable in Phase 1)
```

#### `SnapshotDiff`

Diff two snapshots by snapshot ID or manifest digest.

```rust
EngineQuery::SnapshotDiff { a_ref: SnapshotRef, b_ref: SnapshotRef }
// → EngineQueryResult::SnapshotDiff(Box<SnapshotDiffResult>)
```

---

### Profile Queries

#### `ProfileGet`

```rust
EngineQuery::ProfileGet { profile_ref: String }
// → EngineQueryResult::ProfileGet(ProfileGetResult {
//       profile_ref, profile_digest, payload_json
//   })
// Errors: ProfileNotFound
```

`profile_digest` is the SHA-256 of the stored `payload_json` bytes.

#### `ProfileResolve`

Resolve a profile reference. If `profile_ref` is `None`, resolves the default profile
(the row with `is_default = 1`).

```rust
EngineQuery::ProfileResolve { profile_ref: Option<String> }
// → EngineQueryResult::ProfileResolve(ProfileResolveResult {
//       profile_ref, profile_digest, parsed_profile: serde_json::Value
//   })
// Errors: ProfileNotFound
```

#### `ProfileGetDefault`

Explicit default-profile lookup. Returns `ProfileNotFound` if no default is set.

```rust
EngineQuery::ProfileGetDefault
// → EngineQueryResult::ProfileResolve(ProfileResolveResult)
// Errors: ProfileNotFound
```

#### `ProfileList`

Paginated profile listing, ordered by `profile_ref`.

```rust
EngineQuery::ProfileList(ListOptions { limit: Some(10), cursor: None, .. })
// → EngineQueryResult::ProfileList(Page<ProfileGetResult>)
```

---

### Approval Queries

#### `ApprovalGet`

Fetch a full approval request. Retrieves the payload JSON from CAS via `request_digest`.

```rust
EngineQuery::ApprovalGet { approval_token: String }
// → EngineQueryResult::ApprovalGet(ApprovalGetResult {
//       approval_token, request_digest, semantic_request_digest,
//       payload_json: serde_json::Value
//   })
// Errors: ApprovalNotFound, ApprovalStorageCorrupt (row exists but CAS blob missing)
```

#### `ApprovalList`

Paginated approval listing, ordered by `created_at, approval_token` ascending.

```rust
EngineQuery::ApprovalList(ListOptions::default())
// → EngineQueryResult::ApprovalList(Page<ApprovalGetResult>)
```

#### `ApprovalListByKind`

**Phase 1 deferred.** Returns `ExErrorKind::NotImplemented`.

---

### Constraint Predicate Preview

Non-mutating simulation of constraint predicate resolution. Never creates
an `approval_requests` row regardless of the result.

```rust
EngineQuery::ConstraintPredicatesPreview {
    profile_ref: Option<String>,   // None → use default profile
    context: serde_json::Value,    // evaluation context
    candidates: Vec<String>,       // EP IDs to evaluate against
}
// → EngineQueryResult::PredicatePreview(PredicatePreviewResult {
//       status: PreviewStatus,
//       selected: Option<String>,
//       candidates: Vec<String>,
//   })
```

`PreviewStatus` values:

| Value               | Meaning                                            |
| ------------------- | -------------------------------------------------- |
| `Selected`          | Exactly one candidate selected                     |
| `NoMatch`           | No candidates passed evaluation                    |
| `Ambiguous`         | Multiple candidates matched (no tie-break applied) |
| `RoutedForApproval` | Would have routed for approval (simulated only)    |

**Key invariant**: preview never mutates `approval_requests`.

---

## Policy Queries (`ep:policy_codegen_handoff:0`)

Policy queries require `policy_provider: Some(&provider)`. Passing `None` returns `NotImplemented`.

### `PolicyList`

Returns all policy documents known to the provider, sorted by `policy_ref`.

```rust
EngineQuery::PolicyList
// → EngineQueryResult::PolicyList(Vec<PolicyListEntry {
//       policy_ref: String,
//       version: String,
//   }>)
```

**Errors**: `Io` if the policies directory cannot be read.

---

### `PolicyRead`

Returns the full canonical text of a policy document.

```rust
EngineQuery::PolicyRead { policy_ref: String }
// → EngineQueryResult::PolicyRead(PolicyReadResult {
//       policy_ref: String,
//       text: String,
//   })
```

**Errors**: `PolicyNotFound` if `policy_ref` is unknown. `PolicyParseError` if the file contains invalid UTF-8.

---

### `PolicyExport`

Extracts structured HANDOFF blocks from a policy document.

```rust
EngineQuery::PolicyExport {
    policy_ref: String,
    export_kind: String,  // currently: "codegen_handoff"
}
// → EngineQueryResult::PolicyExport(PolicyExportResult {
//       policy_ref: String,
//       export_kind: String,
//       text: String,   // concatenated HANDOFF block content
//   })
```

**Errors**: `PolicyNotFound` if ref is unknown. `PolicyExportFailed` if markers are malformed or `export_kind` is unknown. `PolicyExportTooLarge` if result exceeds the configured byte limit. See [policy-system.md](./policy-system.md) for HANDOFF marker format.

---

### `SnapshotManifestPolicyRef`

Returns the `policy_ref` string stored in a committed snapshot manifest. Does not require a policy provider.

```rust
EngineQuery::SnapshotManifestPolicyRef { manifest_digest: String }
// → EngineQueryResult::SnapshotManifestPolicyRef(String)
```

**Errors**: `MissingBlob` if the manifest is not in CAS. `InvalidManifest` if the manifest cannot be deserialized. `MissingField` if the manifest has no `policy_ref` field.

---

### `PolicyProjectForHandoff`

Produces a deterministic byte projection of a policy document's HANDOFF content. Two calls with
identical inputs return byte-identical output. This is a read-only, non-mutating operation.

If `profile_ref` is provided, the profile must exist in the store; its existence is validated
before calling the provider.

```rust
EngineQuery::PolicyProjectForHandoff {
    policy_ref: String,
    profile_ref: Option<String>,
}
// → EngineQueryResult::PolicyProjectForHandoff(PolicyProjectForHandoffResult {
//       policy_ref: String,
//       profile_ref: Option<String>,
//       projection_bytes: Vec<u8>,
//   })
```

**Errors**: `PolicyNotFound` if `policy_ref` is unknown. `ProfileNotFound` if `profile_ref` is
`Some` and the profile does not exist in the store. `PolicyExportFailed` if the document has
malformed/unterminated HANDOFF markers. `PolicyExportTooLarge` if the result exceeds the
configured byte limit. `NotImplemented` if `policy_provider` is `None`.

---

## Error Contract

| `ExErrorKind`                  | When raised                                        |
| ------------------------------ | -------------------------------------------------- |
| `NotFound`                     | Generic entity lookup failure                      |
| `ProfileNotFound`              | Profile ref not found                              |
| `ApprovalNotFound`             | Approval token not found                           |
| `ApprovalStorageCorrupt`       | Row exists in DB but CAS blob is missing           |
| `RefinementIntegrityViolation` | EP has more than one structural parent             |
| `MissingBlob`                  | CAS blob not found for a snapshot manifest digest  |
| `NotImplemented`               | Query is valid but deferred to a future phase      |
| `InvalidManifest`              | Manifest bytes cannot be deserialized              |
| `Persistence`                  | SQLite query failure                               |
| `PolicyNotFound`               | `policy_ref` not found in the provider             |
| `PolicyExportFailed`           | Malformed HANDOFF markers or unknown `export_kind` |
| `PolicyExportTooLarge`         | Export size exceeds configured byte limit          |
| `PolicyParseError`             | Policy file contains invalid UTF-8                 |
| `Io`                           | Filesystem / CAS I/O failure                       |
