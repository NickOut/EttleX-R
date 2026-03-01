# Policy System (`ep:policy_codegen_handoff:0`)

Defines the backend-agnostic `PolicyProvider` indirection layer that gates snapshot commits, exposes policy discovery and export, and enforces `PolicyRefMissing` before any writes.

---

## Architecture

```
Snapshot Commit Pipeline
        ↓
  Step 1a: policy_ref empty? → PolicyRefMissing (fail fast)
  Step 1b: policy_provider.policy_check(...)  → PolicyDenied (fail)
        ↓                                       Ok (proceed)
  Steps 2–8: CAS / ledger writes
```

The policy check fires at step 1 — **before** the dry_run short-circuit (step 7) and before any durable writes. This means `DenyAllPolicyProvider` blocks even dry-run operations.

---

## `PolicyProvider` Trait

```rust
pub trait PolicyProvider: Send + Sync {
    fn policy_check(
        &self,
        policy_ref: &str,
        profile_ref: Option<&str>,
        operation: &str,
        entity_id: Option<&str>,
    ) -> Result<(), ExError>;

    fn policy_read(&self, policy_ref: &str) -> Result<String, ExError>;

    fn policy_export(&self, policy_ref: &str, export_kind: &str) -> Result<String, ExError>;

    fn policy_list(&self) -> Result<Vec<PolicyListEntry>, ExError>;
}
```

All methods return `ExError` on failure. The trait is `Send + Sync` to support multi-threaded callers.

### Built-in Implementations

| Type                    | Behaviour                                                                         |
| ----------------------- | --------------------------------------------------------------------------------- |
| `NoopPolicyProvider`    | `policy_check` always `Ok(())`. `policy_read`/`policy_export` → `PolicyNotFound`. |
| `DenyAllPolicyProvider` | `policy_check` always `Err(PolicyDenied)`. Others → `PolicyNotFound`.             |
| `FilePolicyProvider`    | Reads `.md` files from a directory on the local filesystem.                       |

---

## `FilePolicyProvider`

Backed by a directory of Markdown files. Each `.md` file is one policy document.

```rust
use ettlex_store::file_policy_provider::FilePolicyProvider;

let provider = FilePolicyProvider::new("policies");
// Override the default 1 MiB export limit:
let provider = FilePolicyProvider::new("policies").with_max_bytes(512_000);
```

- **`policy_ref`**: the file stem (e.g. `codegen_handoff_policy_v1` for `codegen_handoff_policy_v1.md`).
- **`policy_list()`**: returns all `.md` files sorted by `policy_ref`, each with `version = "0"`.
- **`policy_read(ref)`**: returns the full UTF-8 text of the file.
- **`policy_export(ref, kind)`**: extracts HANDOFF blocks (see below).
- **`policy_check(ref, ...)`**: verifies the file exists; `Ok(())` if it does.

---

## HANDOFF Marker Format

Policy export with `export_kind = "codegen_handoff"` extracts all blocks delimited by:

```markdown
<!-- HANDOFF: START -->

... obligation text ...

<!-- HANDOFF: END -->
```

Rules:

- Multiple HANDOFF blocks are concatenated in document order (separated by `\n`).
- Each block's content is trimmed of leading/trailing whitespace.
- An unterminated `<!-- HANDOFF: START -->` → `PolicyExportFailed`.
- An `<!-- HANDOFF: END -->` without a preceding `START` → `PolicyExportFailed`.
- A nested `<!-- HANDOFF: START -->` inside a block → `PolicyExportFailed`.
- A file with no HANDOFF blocks at all → `PolicyExportFailed`.
- Exported text exceeding `max_export_bytes` → `PolicyExportTooLarge`.

---

## `policy_ref` Conventions

- Policy references are stable opaque strings (e.g. `codegen_handoff_policy_v1`, `policy/default@0`).
- `FilePolicyProvider` maps `policy_ref` to `{policies_dir}/{policy_ref}.md`.
- An empty `policy_ref` string passed to the snapshot commit pipeline returns `PolicyRefMissing` immediately (before the policy check).

---

## `PolicyRefMissing` Enforcement

The snapshot commit pipeline (`snapshot_commit_by_leaf`) enforces a non-empty `policy_ref` **before** calling `policy_provider.policy_check`. This applies in both normal and `dry_run=true` modes.

```
policy_ref = "" → Err(PolicyRefMissing) — no writes, no policy check
policy_ref = "..." → policy_provider.policy_check(...) → Ok/Err
```

---

## `PolicyProviderAnchorAdapter`

Wraps any `PolicyProvider` and implements `AnchorPolicy` with **NeverAnchored** semantics — both `is_anchored_ep` and `is_anchored_ettle` return `false` for all inputs.

```rust
use ettlex_core::policy::{AnchorPolicy, PolicyProviderAnchorAdapter};
use ettlex_core::policy_provider::NoopPolicyProvider;

let adapter = PolicyProviderAnchorAdapter::new(&NoopPolicyProvider);
assert!(!adapter.is_anchored_ep("ep:root:0"));
```

This is a Phase 1 adapter. Phase 2 will add selective anchoring via policy document lookup.

---

## Error Contract

| `ExErrorKind`          | When raised                                                                 |
| ---------------------- | --------------------------------------------------------------------------- |
| `PolicyDenied`         | `policy_check` rejected the operation                                       |
| `PolicyNotFound`       | `policy_ref` does not exist in the provider                                 |
| `PolicyRefMissing`     | Empty `policy_ref` passed to snapshot commit                                |
| `PolicyExportFailed`   | Malformed/unterminated HANDOFF markers, unknown `export_kind`, or no blocks |
| `PolicyExportTooLarge` | Exported HANDOFF content exceeds `max_export_bytes` limit                   |
| `PolicyParseError`     | Policy file contains invalid UTF-8                                          |

---

## Engine Query Integration

Policy operations are accessible through `apply_engine_query`:

```rust
use ettlex_engine::commands::engine_query::{apply_engine_query, EngineQuery, EngineQueryResult};
use ettlex_store::file_policy_provider::FilePolicyProvider;

let provider = FilePolicyProvider::new("policies");

// List all policies
let result = apply_engine_query(EngineQuery::PolicyList, &conn, &cas, Some(&provider))?;

// Read a policy
let result = apply_engine_query(
    EngineQuery::PolicyRead { policy_ref: "codegen_handoff_policy_v1".into() },
    &conn, &cas, Some(&provider),
)?;

// Export HANDOFF blocks
let result = apply_engine_query(
    EngineQuery::PolicyExport {
        policy_ref: "codegen_handoff_policy_v1".into(),
        export_kind: "codegen_handoff".into(),
    },
    &conn, &cas, Some(&provider),
)?;

// Look up policy_ref from a committed snapshot manifest
let result = apply_engine_query(
    EngineQuery::SnapshotManifestPolicyRef { manifest_digest: digest },
    &conn, &cas, None,
)?;
```

See [`docs/action-read-tools.md`](./action-read-tools.md) for the full query reference.
