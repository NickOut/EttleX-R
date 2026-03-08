# Leaf Bundle: EttleX Product > Storage Spine (SQLite + CAS + Seed Import)

## WHY (Rationale)

1. Establish the product-level framing for EttleX as a semantic evolution engine.
   This EP is non-load-bearing for the storage/commit milestones; it exists to keep the tree rooted in a
   recognisable product-level node for rendering and navigation.

2. Provide a durable substrate so semantic state can be anchored, reproduced, and evolved safely.

3. EttleCreate and EpCreate both accept caller-supplied identity fields (ettle_id and ep_id) but silently ignore them, always generating ULIDs. This is a silent contract violation: callers cannot control identity through the Apply surface, but the schema implies they can.

The seed import path that motivated caller-supplied IDs is a throwaway bootstrapping mechanism being phased out in favour of MCP/CLI/UI. There is no production use case for caller-supplied entity IDs through Apply commands. The store owns identity, always.

Separately, PolicyCreate is absent from the Apply command vocabulary. Policies are currently file-backed only. This creates a bootstrapping deadlock: SnapshotCommit requires a policy_ref, but the MCP cannot create policies. PolicyCreate must be added as a first-class Apply command.

Note on identity vs reference: policy_ref is a stable semantic reference (e.g. codegen_handoff@1), not a database primary key. Caller-supplied policy_ref in PolicyCreate is correct and is the intended exception to the ULID-only rule. The distinction is: entity identity (Ettle, EP) is always store-generated; semantic references (policy_ref, profile_ref) are caller-supplied stable strings.

Non-goals: changing ULID generation; altering other command signatures; policy enforcement semantics.

## WHAT (Description)

1. Maintain a minimal product framing so leaf Ettles can be rendered in context.
   This EP does not impose implementation requirements on Phase 1/2 milestones.

2. Establish the platform foundations as refined milestones under this EP:

- Storage spine (SQLite + CAS + seed import)
- Snapshot commit pipeline (manifest + ledger anchor)

3. 1. EttleCreate — ep_id field removed from command schema
   - The EttleCreate command MUST NOT accept an ettle_id field.
   - The store MUST always generate a ULID for new Ettle identity.
   - If an ettle_id field is present in the EttleCreate payload the command MUST be rejected with error code InvalidInput before any mutation occurs.
   - The generated ettle_id MUST be returned in the command result.

4. EpCreate — ep_id field removed from command schema
   - The EpCreate command MUST NOT accept an ep_id field.
   - The store MUST always generate a ULID for new EP identity.
   - If an ep_id field is present in the EpCreate payload the command MUST be rejected with error code InvalidInput before any mutation occurs.
   - The generated ep_id MUST be returned in the command result.

5. PolicyCreate — added to Apply command vocabulary
   - A new Apply command MUST exist: PolicyCreate { policy_ref: String, text: String }
   - policy_ref is the caller-supplied stable semantic reference (e.g. codegen_handoff@1). It is NOT store-generated.
   - text is the full policy body including HANDOFF markers.
   - The store MUST persist the policy as a file-backed document addressable by policy_ref.
   - PolicyCreate MUST fail with PolicyConflict if a policy with the same policy_ref already exists. No silent overwrite is permitted.
   - PolicyCreate MUST fail with InvalidInput if text is empty.
   - PolicyCreate MUST fail with InvalidInput if policy_ref is empty or malformed.
   - After successful PolicyCreate, the policy MUST be immediately retrievable via policy.get(policy_ref).
   - PolicyCreate MUST be atomic: either the policy is persisted and retrievable, or it is not. Partial persistence is not permitted.
   - PolicyCreate MUST increment state_version by exactly 1 on success.

6. Identity vs reference invariant (binding across all commands)
   - Entity identity fields (ettle_id, ep_id) MUST always be store-generated ULIDs.
   - Semantic reference fields (policy_ref, profile_ref) MUST be caller-supplied stable strings.
   - No command MUST silently discard a supplied field; presence of a disallowed field MUST be rejected with InvalidInput.

## HOW (Implementation)

1. No scenarios. This EP is informational only.

2. Refinement only. Implementation scenarios live in child Ettles.

3. Scenarios (all MUST be implemented as tests; Gherkin is normative):

Feature: Store owns entity identity; caller-supplied IDs are rejected

Background:
Given a repository with SQLite + CAS store initialised
And MCP server is started in dev mode

# --- Happy path ---

Scenario: EttleCreate with no ettle_id generates a ULID and returns it
When I call ettlex.apply with Command::EttleCreate{title="My Ettle"}
Then the response includes a store-generated ettle_id in ULID format
And new_state_version increments by 1
And ettle.get(that ettle_id) returns the created Ettle

Scenario: EpCreate with no ep_id generates a ULID and returns it
Given an Ettle exists
When I call ettlex.apply with Command::EpCreate{ettle_id=<ettle>, ordinal=0}
Then the response includes a store-generated ep_id in ULID format
And new_state_version increments by 1
And ep.get(that ep_id) returns the created EP

# --- Negative cases ---

Scenario: EttleCreate rejects supplied ettle_id
When I call ettlex.apply with Command::EttleCreate{title="T", ettle_id="ettle:custom"}
Then error_code InvalidInput is returned
And no Ettle is created
And state_version is unchanged

Scenario: EpCreate rejects supplied ep_id
Given an Ettle exists
When I call ettlex.apply with Command::EpCreate{ettle_id=<ettle>, ordinal=0, ep_id="ep:custom"}
Then error_code InvalidInput is returned
And no EP is created
And state_version is unchanged

# --- Explicit error paths ---

Scenario: EttleCreate with empty title fails
When I call ettlex.apply with Command::EttleCreate{title=""}
Then error_code InvalidInput is returned
And state_version is unchanged

Scenario: EpCreate referencing missing ettle fails
When I call ettlex.apply with Command::EpCreate{ettle_id="ettle:missing", ordinal=0}
Then error_code NotFound is returned
And state_version is unchanged

# --- Boundary conditions ---

Scenario: EttleCreate with maximum-length title succeeds
When I call ettlex.apply with Command::EttleCreate{title=<255-char string>}
Then the Ettle is created successfully
And the stored title matches the supplied value

Scenario: EpCreate with ordinal 0 on an Ettle that already has ordinal 0 fails
Given an Ettle exists with an EP at ordinal 0
When I call ettlex.apply with Command::EpCreate{ettle_id=<ettle>, ordinal=0}
Then error_code OrdinalConflict (or equivalent) is returned
And state_version is unchanged

# --- Invariants ---

Scenario: Generated ettle_id is a valid ULID format
When I call ettlex.apply with Command::EttleCreate{title="T"}
Then the returned ettle_id matches ULID format [0-9A-Z]{26} (case-insensitive)

Scenario: Generated ep_id is a valid ULID format
Given an Ettle exists
When I call ettlex.apply with Command::EpCreate{ettle_id=<ettle>, ordinal=0}
Then the returned ep_id matches ULID format

Scenario: Two successive EttleCreate calls produce distinct ettle_ids
When I call ettlex.apply with Command::EttleCreate{title="A"}
And I call ettlex.apply with Command::EttleCreate{title="B"}
Then the two returned ettle_ids are distinct

# --- Idempotency / repeatability ---

# Not applicable: EttleCreate and EpCreate are non-idempotent by design (append-only).

# Repeated calls with identical inputs produce new entities with distinct ULIDs.

Scenario: Repeated EttleCreate with identical title produces distinct Ettles
When I call ettlex.apply with Command::EttleCreate{title="Duplicate"} twice
Then two distinct ettle_ids are returned
And ettle.list() includes both

# --- Determinism / ordering ---

# Not applicable to identity generation (ULIDs are monotonic but not deterministic by design).

# Ordering of list results is covered by ettle.list and ep.list scenarios.

# --- State transitions ---

Scenario: EttleCreate followed by ettle.get returns consistent state
When I call ettlex.apply with Command::EttleCreate{title="StateTest"}
Then ettle.get(returned ettle_id) returns title "StateTest"
And the Ettle has no EPs yet

# --- Concurrency ---

Scenario: Concurrent EttleCreate calls each produce distinct ULIDs
Given two concurrent callers A and B
When both call Command::EttleCreate{title="Concurrent"} simultaneously
Then each receives a distinct ettle_id
And both Ettles are present in ettle.list()
And no ettle_id collision occurs

# --- Security / authorisation ---

# Auth enforcement is governed by ep:mcp_thin_slice. These scenarios cover command-level rejection only.

# --- Observability ---

# state_version increment is the observable signal for successful mutation (covered in happy path).

# --- Compatibility / migration ---

Scenario: Seed import files that supplied ettle_id or ep_id are rejected at import time
Given a seed import file containing EttleCreate with an explicit ettle_id
When the seed importer processes the file
Then the import fails with a descriptive error identifying the disallowed field
And no partial state is written

# --- Resource / performance ---

# Not specified for this command; no performance obligations defined.

# --- Explicit prohibition ---

Scenario: EttleCreate MUST NOT silently discard a supplied ettle_id
When I call ettlex.apply with Command::EttleCreate{title="T", ettle_id="ettle:x"}
Then the command does NOT succeed with a ULID-generated ettle_id
And error_code InvalidInput is returned

Scenario: EpCreate MUST NOT silently discard a supplied ep_id
Given an Ettle exists
When I call ettlex.apply with Command::EpCreate{ettle_id=<ettle>, ordinal=0, ep_id="ep:x"}
Then the command does NOT succeed with a ULID-generated ep_id
And error_code InvalidInput is returned

# --- Byte-level equivalence ---

# Not applicable: ULID generation is intentionally non-deterministic.

# --- Concurrency conflict ---

# Covered under Concurrency above.

Feature: PolicyCreate adds policies to the Apply command vocabulary

Background:
Given a repository with SQLite + CAS store initialised
And MCP server is started in dev mode

# --- Happy path ---

Scenario: PolicyCreate with valid policy_ref and text succeeds
When I call ettlex.apply with Command::PolicyCreate{policy_ref="codegen_handoff@1", text="# Policy\n<!-- HANDOFF: START -->\n## B1.1\nContent.\n<!-- HANDOFF: END -->"}
Then new_state_version increments by 1
And result.tag is PolicyCreate
And policy.get("codegen_handoff@1") returns a document with that text

# --- Negative cases ---

Scenario: PolicyCreate rejects duplicate policy_ref
Given policy "codegen_handoff@1" already exists
When I call ettlex.apply with Command::PolicyCreate{policy_ref="codegen_handoff@1", text="different text"}
Then error_code PolicyConflict is returned
And the existing policy text is unchanged
And state_version is unchanged

Scenario: PolicyCreate rejects empty text
When I call ettlex.apply with Command::PolicyCreate{policy_ref="new@0", text=""}
Then error_code InvalidInput is returned
And no policy is created
And state_version is unchanged

Scenario: PolicyCreate rejects empty policy_ref
When I call ettlex.apply with Command::PolicyCreate{policy_ref="", text="content"}
Then error_code InvalidInput is returned
And state_version is unchanged

# --- Explicit error paths ---

Scenario: PolicyCreate rejects malformed policy_ref (no version separator)
When I call ettlex.apply with Command::PolicyCreate{policy_ref="notype", text="content"}
Then error_code InvalidInput is returned
And state_version is unchanged

Scenario: PolicyCreate surfaces storage failure as typed error
Given the policy file store is configured to fail writes
When I call ettlex.apply with Command::PolicyCreate{policy_ref="new@0", text="content"}
Then a typed error StorageError (or Io) is returned
And state_version is unchanged
And no partial policy file exists

# --- Boundary conditions ---

Scenario: PolicyCreate with maximum-length policy_ref succeeds
When I call ettlex.apply with Command::PolicyCreate{policy_ref=<max-length valid ref>, text="content"}
Then the policy is created and retrievable

Scenario: PolicyCreate with large text body (100KB) succeeds
When I call ettlex.apply with Command::PolicyCreate{policy_ref="large@0", text=<100KB text>}
Then the policy is created and retrievable
And policy.get("large@0") returns the full text

# --- Invariants ---

Scenario: PolicyCreate is atomic — no partial state on failure
Given the policy file store fails after partial write (simulated)
When I call ettlex.apply with Command::PolicyCreate{policy_ref="partial@0", text="content"}
Then either the policy is fully persisted and retrievable, or it is not present at all
And policy.list() does not include "partial@0" in a partially-written state

Scenario: policy_ref is the stable retrieval key
When I call PolicyCreate with policy_ref="myref@2"
Then policy.get("myref@2") retrieves exactly that policy
And no other policy_ref retrieves it

# --- Idempotency ---

Scenario: PolicyCreate is NOT idempotent — identical second call fails
When I call PolicyCreate{policy_ref="idem@0", text="t"} successfully
And I call PolicyCreate{policy_ref="idem@0", text="t"} again
Then error_code PolicyConflict is returned on the second call
And state_version increments only once total

# --- Determinism ---

# policy_ref is caller-supplied; no ordering concern for creation.

# policy.list ordering is covered by read tool scenarios.

# --- State transitions ---

Scenario: After PolicyCreate, policy.list includes the new policy
Given no policies exist
When I call PolicyCreate{policy_ref="listed@0", text="content"}
Then policy.list() includes an entry for "listed@0"

Scenario: After PolicyCreate, SnapshotCommit can reference the new policy_ref
Given a valid leaf EP exists
And profile "dev@0" exists
When I call PolicyCreate{policy_ref="snap_policy@0", text="content"}
And I call SnapshotCommit{leaf_ep_id=..., policy_ref="snap_policy@0", profile_ref="dev@0"}
Then the snapshot is committed successfully
And the snapshot manifest records policy_ref as "snap_policy@0"

# --- Concurrency ---

Scenario: Concurrent PolicyCreate calls with different policy_refs both succeed
Given two concurrent callers A and B
When A calls PolicyCreate{policy_ref="policy_a@0", text="a"}
And B calls PolicyCreate{policy_ref="policy_b@0", text="b"}
Then both succeed
And policy.list() includes both "policy_a@0" and "policy_b@0"

Scenario: Concurrent PolicyCreate calls with the same policy_ref produce exactly one success
Given two concurrent callers A and B
When both call PolicyCreate{policy_ref="conflict@0", text="content"} simultaneously
Then exactly one succeeds
And the other returns PolicyConflict
And policy.get("conflict@0") is consistent (not corrupted)

# --- Security / authorisation ---

# Auth enforcement is governed by ep:mcp_thin_slice. No additional auth at command level.

# --- Observability ---

Scenario: PolicyCreate success is reflected in state_version
Given state_version is V
When I call PolicyCreate{policy_ref="obs@0", text="t"} successfully
Then state.get_version() returns V+1

# --- Compatibility / migration ---

Scenario: Existing file-backed policies created outside Apply remain retrievable after PolicyCreate is introduced
Given a policy file exists at the configured policy directory with ref "legacy@0"
When I call policy.get("legacy@0")
Then the policy text is returned successfully
And PolicyCreate for a new ref does not affect legacy@0

# --- Resource / performance ---

# No specific performance obligations defined for PolicyCreate.

# --- Explicit prohibition ---

Scenario: PolicyCreate MUST NOT overwrite an existing policy
Given policy "immutable@0" exists with text "original"
When I call PolicyCreate{policy_ref="immutable@0", text="modified"}
Then error_code PolicyConflict is returned
And policy.get("immutable@0") still returns "original"

# --- Byte-level equivalence ---

# Not applicable to PolicyCreate itself; policy.export determinism is covered in ep:action_read_tools.

# --- Concurrency conflict ---

# Covered under Concurrency above.
