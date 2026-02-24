# Design Decision Schema Stubs (canonical decisions + EP/Ettle linkage; MCP-ready)

## EP 0

**Normative**: Yes

**WHY**: Human/AI authoring and refinement produces binding design decisions before and during Ettle/EP authoring.
If those decisions are not captured as first-class canonical artefacts, the system loses provenance and
governance traceability; rationale becomes scattered across chat logs (non-portable) or embedded ad hoc
in EP.HOW prose (hard to diff, hard to audit).

This seed introduces the minimal canonical persistence and query-friendly linkage stubs needed so that:

- MCP (and later CLI/UI) can capture decisions at the moment they are made,
- decisions can be viewed in context with EPs and the EPT projection,
- later work can add policy gates, evidence workflows, and snapshot projections without schema churn.

This seed is intentionally non-load-bearing for snapshot semantics:

- Decisions are canonical governance artefacts, not part of snapshot semantic closure in this tier.
- Snapshot manifests and semantic_manifest_digest MUST remain unaffected by decision content/links.

**WHAT**: Implement the minimal canonical persistence, linkage, and deterministic query surfaces for design decisions.

Success criteria (binding):

1. Decision entities exist in canonical state as first-class artefacts.
2. Decisions can be linked to EPs and/or Ettles with explicit relation kinds.
3. Decisions can be queried deterministically, including "inherited" context over an EPT (ancestor decisions).
4. MCP (when present) can act as the capture mechanism by calling action:commands and reading via action:queries.
5. Nothing in this seed requires chat-UI-only references. Evidence MUST be portable (stored text or repo path).
6. Snapshot commit and snapshot manifests are NOT changed by this seed (decisions are non-snapshot-semantic here).
7. No decision enforcement, evaluation, or policy gating is introduced (deferred).

Binding rules:

- Canonical mutations MUST be expressed as action:commands (Apply).
- Read access MUST be exposed as action:queries (read-only).
- Determinism: identical inputs to queries MUST produce byte-identical outputs (after canonical serialization).
- Ordering MUST be deterministic under all list/query operations.
- MCP MUST be a thin transport wrapper over the same action layer (commands + queries).

Non-snapshot-semantic rule (binding; Phase 1/early Phase 2):

- Decision content and decision link state MUST NOT affect:
  - snapshot_commit manifest bytes
  - semantic_manifest_digest
  - snapshot.diff results
- Implementations MUST NOT add decisions into manifests in this seed.
- If an implementation chooses to add optional decision projection later, it MUST be additive-only and
  must be introduced by a dedicated seed that updates snapshot_commit, manifest schema, and snapshot_diff.

Required schema changes (minimum; additive):

1. decisions
   - decision_id TEXT PRIMARY KEY
   - title TEXT NOT NULL
   - status TEXT NOT NULL # proposed|accepted|superseded|rejected (open set allowed)
   - decision_text TEXT NOT NULL
   - rationale TEXT NOT NULL
   - alternatives_text TEXT NULL
   - consequences_text TEXT NULL
   - evidence_kind TEXT NOT NULL # none|excerpt|capture|file (open set allowed)
   - evidence_excerpt TEXT NULL # portable short excerpt (recommended for 'excerpt' and 'capture')
   - evidence_capture_id TEXT NULL # optional FK into decision_evidence_items when used
   - evidence_file_path TEXT NULL # optional repo-relative path when used (portable)
   - evidence_hash TEXT NULL # sha256 over canonical evidence content when applicable
   - created_at TEXT NOT NULL
   - updated_at TEXT NOT NULL
   - tombstoned_at TEXT NULL # tombstone, optional but recommended

2. decision_evidence_items (OPTIONAL but RECOMMENDED; portable conversation capture)
   - evidence_capture_id TEXT PRIMARY KEY
   - source TEXT NOT NULL # mcp_chat_capture|manual_copy|meeting_notes|export (open set)
   - content TEXT NOT NULL # portable text/markdown blob (may be partial excerpt)
   - content_hash TEXT NOT NULL # sha256 of canonical content bytes
   - created_at TEXT NOT NULL

3. decision_links (links decisions to targets; additive)
   - decision_id TEXT NOT NULL
   - target_kind TEXT NOT NULL # ep|ettle|constraint|snapshot (open set; snapshot reserved)
   - target_id TEXT NOT NULL
   - relation_kind TEXT NOT NULL # grounds|constrains|motivates|supersedes|evidence_for (open set)
   - ordinal INTEGER NOT NULL DEFAULT 0 # deterministic ordering surface
   - created_at TEXT NOT NULL
   - tombstoned_at TEXT NULL
   - PRIMARY KEY (decision_id, target_kind, target_id, relation_kind)
   - FOREIGN KEY(decision_id) REFERENCES decisions(decision_id)

Deterministic ordering rules (binding):

- decision.list ordering: (created_at ASC, decision_id ASC) unless explicit sort options provided.
- decision_links ordering for a given target: (ordinal ASC, relation_kind ASC, decision_id ASC).
- If ordinal is unused (all 0), the fallback ordering MUST still be stable (relation_kind, decision_id).

Required action:commands (minimum; names are normative within the action layer):

- DecisionCreate(decision_id?, title, status?, decision_text, rationale,
  alternatives_text?, consequences_text?,
  evidence_kind, evidence_excerpt?, evidence_capture_content?, evidence_file_path?)
  -> { decision_id }
  Notes:
  - decision_id MAY be supplied (deterministic external id) or generated (ULID recommended).
  - If evidence_capture_content is supplied, the implementation MUST store a decision_evidence_item and
    link it to the decision (evidence_capture_id), computing content_hash deterministically.

- DecisionUpdate(decision_id, title?, status?, decision_text?, rationale?,
  alternatives_text?, consequences_text?,
  evidence_kind?, evidence_excerpt?, evidence_capture_content?, evidence_file_path?)
  -> ()

- DecisionTombstone(decision_id) -> ()
  Notes:
  - Tombstoning MUST NOT delete rows; it marks tombstoned_at and prevents new links by default.

- DecisionLink(decision_id, target_kind, target_id, relation_kind, ordinal?) -> ()
- DecisionUnlink(decision_id, target_kind, target_id, relation_kind) -> ()

- DecisionSupersede(old_decision_id, new_decision_id) -> ()
  Notes:
  - This MUST be represented as a DecisionLink(old -> new, relation_kind="supersedes") or equivalent.
  - Supersede MUST NOT implicitly tombstone the old decision.

Required action:queries (minimum; read-only):

- decision.get(decision_id) -> decision record + evidence summary + outgoing links
- decision.list(filters?) -> ordered list (deterministic; paginated)
- decision.list_by_target(target_kind, target_id, include_tombstoned=false, relation_filter?) -> ordered list
- ep.list_decisions(ep_id, include_ancestors=false, status_filter?, relation_filter?) -> ordered list
- ettle.list_decisions(ettle_id, include_eps=false, include_ancestors=false, ...) -> ordered list
- ept.compute_decision_context(leaf_ep_id, status_filter?, relation_filter?)
  -> { direct_by_ep: {ep_id: [decisions...]}, inherited_for_leaf: [decisions...] }
  Notes:
  - This query MUST use the canonical refinement graph / EPT projection for ancestor enumeration.
  - This query is read-only and MUST be deterministic.

Read/Render integration contract (binding; via action:queries):

- Existing EP and EPT read queries (if present) MUST be able to optionally include decision summaries.
- Implementations MAY add flags like include_decisions/include_ancestor_decisions to ep.get / ept.compute views.
- If such flags are added, they MUST call the decision queries above and MUST NOT introduce bespoke store reads.

Out of scope (explicitly deferred):

- Automatic extraction from transcripts without user/agent-curated capture content
- Policy gating (e.g. "accepted decision required to commit")
- Decision evaluation semantics (e.g. UngroundedDecision) and evidence workflows
- Snapshot manifest projection of decisions and decision diffs

**HOW**: Scenarios (all MUST be implemented as tests; unit/integration; Gherkin is normative):

Feature: Decision schema stubs provide portable decision capture and deterministic queries

Background:
Given a repository with SQLite + CAS store initialised
And the refinement graph contains at least one Ettle with EPs and refine links
And the action layer is available with Apply (commands) and Query (read tools)

# --- CRUD: create/update/tombstone ---

Scenario: Create decision succeeds with portable excerpt evidence
When I apply Command::DecisionCreate{title="Use manifest-bytes diff", decision_text="...", rationale="...",
evidence_kind="excerpt", evidence_excerpt="We need determinism ..."}
Then the decision exists in canonical state
And its status is "proposed" by default
And its evidence_kind is "excerpt"
And evidence_hash is computed deterministically over the excerpt bytes

Scenario: Create decision rejects missing title
When I apply Command::DecisionCreate{title="", decision_text="x", rationale="y", evidence_kind="none"}
Then a typed error InvalidDecision is returned

Scenario: Create decision rejects missing decision_text
When I apply Command::DecisionCreate{title="t", decision_text="", rationale="y", evidence_kind="none"}
Then a typed error InvalidDecision is returned

Scenario: Create decision rejects missing rationale
When I apply Command::DecisionCreate{title="t", decision_text="x", rationale="", evidence_kind="none"}
Then a typed error InvalidDecision is returned

Scenario: Create decision supports explicit decision_id
When I apply Command::DecisionCreate{decision_id="d:001", title="t", decision_text="x", rationale="y", evidence_kind="none"}
Then decision "d:001" exists

Scenario: Create decision rejects duplicate decision_id
Given decision "d:001" already exists
When I apply Command::DecisionCreate{decision_id="d:001", title="t2", decision_text="x2", rationale="y2", evidence_kind="none"}
Then a typed error AlreadyExists is returned

Scenario: Update decision modifies updated_at and preserves created_at
Given decision "d:001" exists with created_at "T1"
When I apply Command::DecisionUpdate{decision_id="d:001", status="accepted"}
Then decision "d:001" has status "accepted"
And created_at remains "T1"
And updated_at is greater than or equal to "T1"

Scenario: Tombstone decision prevents new linking by default
Given decision "d:002" exists
When I apply Command::DecisionTombstone{decision_id="d:002"}
Then decision "d:002" is marked tombstoned
When I apply Command::DecisionLink{decision_id="d:002", target_kind="ep", target_id="ep:x", relation_kind="grounds"}
Then a typed error DecisionTombstoned is returned

# --- Evidence capture portability ---

Scenario: Create decision stores capture content as evidence item when provided
When I apply Command::DecisionCreate{title="Capture mechanism", decision_text="...", rationale="...",
evidence_kind="capture",
evidence_excerpt="Short excerpt ...",
evidence_capture_content="# Notes\nWe discussed ..."}
Then the decision exists
And decision.evidence_capture_id is not null
And decision_evidence_items contains that evidence_capture_id
And content_hash is computed deterministically from the content bytes

Scenario: Create decision rejects capture kind without capture content or excerpt
When I apply Command::DecisionCreate{title="t", decision_text="x", rationale="y", evidence_kind="capture"}
Then a typed error InvalidEvidence is returned

Scenario: Create decision accepts file evidence with repo-relative path
When I apply Command::DecisionCreate{title="t", decision_text="x", rationale="y",
evidence_kind="file", evidence_file_path="evidence/2026-02-23/d-001.md"}
Then the decision exists
And evidence_file_path is stored verbatim
And the implementation does not attempt to read that file in this seed

Scenario: Create decision rejects file kind without file path
When I apply Command::DecisionCreate{title="t", decision_text="x", rationale="y",
evidence_kind="file"}
Then a typed error InvalidEvidence is returned

Scenario: Create decision rejects absolute file paths
When I apply Command::DecisionCreate{title="t", decision_text="x", rationale="y",
evidence_kind="file", evidence_file_path="/etc/passwd"}
Then a typed error InvalidEvidencePath is returned

# --- Linking: attach/unattach to EP/Ettle ---

Scenario: Link decision to EP with deterministic ordering
Given decision "d:010" exists
And EP "ep:x" exists
When I apply Command::DecisionLink{decision_id="d:010", target_kind="ep", target_id="ep:x", relation_kind="grounds", ordinal=1}
And I apply Command::DecisionLink{decision_id="d:011", target_kind="ep", target_id="ep:x", relation_kind="grounds", ordinal=0}
When I query ep.list_decisions("ep:x")
Then the first decision is "d:011"
And the second decision is "d:010"

Scenario: Duplicate link is rejected
Given decision "d:010" is already linked to EP "ep:x" as relation "grounds"
When I apply Command::DecisionLink{decision_id="d:010", target_kind="ep", target_id="ep:x", relation_kind="grounds"}
Then a typed error DuplicateLink is returned

Scenario: Unlink removes link but preserves decision history
Given decision "d:010" is linked to EP "ep:x"
When I apply Command::DecisionUnlink{decision_id="d:010", target_kind="ep", target_id="ep:x", relation_kind="grounds"}
Then ep.list_decisions("ep:x") does not include "d:010"
And decision.get("d:010") still returns the decision

Scenario: Link rejects unknown decision id
When I apply Command::DecisionLink{decision_id="d:missing", target_kind="ep", target_id="ep:x", relation_kind="grounds"}
Then a typed error NotFound is returned

Scenario: Link rejects unknown EP id
Given decision "d:010" exists
When I apply Command::DecisionLink{decision_id="d:010", target_kind="ep", target_id="ep:missing", relation_kind="grounds"}
Then a typed error NotFound is returned

Scenario: Link rejects unknown target_kind unless explicitly allowed
Given decision "d:010" exists
When I apply Command::DecisionLink{decision_id="d:010", target_kind="weird", target_id="x", relation_kind="grounds"}
Then a typed error InvalidTargetKind is returned

# --- Supersession semantics (graph) ---

Scenario: Supersede creates a deterministic supersedes link
Given decisions "d:100" and "d:101" exist
When I apply Command::DecisionSupersede{old_decision_id="d:100", new_decision_id="d:101"}
Then decision.list_by_target(target_kind="decision", target_id="d:101") is not required (not implemented)
And decision.get("d:100") outgoing links include relation_kind="supersedes" to "d:101"

Scenario: Supersede does not tombstone the old decision
Given decisions "d:100" and "d:101" exist
When I apply Command::DecisionSupersede{old_decision_id="d:100", new_decision_id="d:101"}
Then decision.get("d:100").tombstoned_at is null

# --- Deterministic queries and pagination ---

Scenario: decision.list ordering is deterministic under repeated calls
Given at least 200 decisions exist
When I query decision.list(limit=50) twice
Then both results are byte-identical after canonical JSON serialization

Scenario: decision.list supports cursor-based pagination deterministically
Given at least 250 decisions exist
When I query decision.list(limit=100)
Then I receive page_1 with 100 decisions and cursor_1
When I query decision.list(limit=100, cursor=cursor_1)
Then I receive page_2 with 100 decisions and cursor_2
And page_1 and page_2 contain no duplicates
And repeating the same calls returns identical pages

Scenario: ep.list_decisions filters by status
Given decisions linked to EP "ep:x" include both proposed and accepted
When I query ep.list_decisions("ep:x", status_filter="accepted")
Then only accepted decisions are returned

# --- EPT context projection (ancestor inclusion) ---

Scenario: ep.list_decisions include_ancestors returns decisions from ancestor EPs along the closure
Given a refinement chain ep:root -> ep:mid -> ep:leaf exists
And decision "d:200" is linked to ep:root
And decision "d:201" is linked to ep:mid
And decision "d:202" is linked to ep:leaf
When I query ep.list_decisions(ep:leaf, include_ancestors=true)
Then results include d:200, d:201, d:202
And the ordering is deterministic

Scenario: ept.compute_decision_context returns direct_by_ep map deterministically
Given a leaf_ep_id whose computed EPT contains multiple EPs
And each EP has linked decisions
When I query ept.compute_decision_context(leaf_ep_id) twice
Then the returned structure is identical
And each per-EP decision list ordering matches the deterministic link ordering rules

Scenario: Ancestor enumeration rejects ambiguous refinement graphs
Given the refinement graph is corrupted such that an EP has multiple parents
When I query ep.list_decisions(ep:leaf, include_ancestors=true)
Then a typed error RefinementIntegrityViolation is returned
And the error includes the conflicting parent EP ids

# --- Non-snapshot-semantic invariant ---

Scenario: Decisions do not affect snapshot manifest bytes or semantic digest
Given a fixed canonical state for EPs and constraints
And a committed snapshot S1 exists for a leaf EP
When I create a new decision and link it to an EP within the closure
And I commit a new snapshot S2 for the same leaf without changing EPs/constraints
Then S2.semantic_manifest_digest equals S1.semantic_manifest_digest
And snapshot.diff(S1,S2) classifies as no_semantic_change

# --- Negative cases: determinism violations ---

Scenario: Non-deterministic iteration order in decision listing is detected
Given code changes that iterate decisions using hash-map iteration order
When I query decision.list() twice with identical canonical state
Then the determinism test fails
And the failure is reported as DeterminismViolation

Scenario: Non-canonical JSON serialization in query output is rejected
Given code changes that emit JSON with unstable key ordering
When I query ep.list_decisions("ep:x") twice
Then the determinism test fails with DeterminismViolation

# --- Boundary conditions ---

Scenario: Large evidence captures are supported without breaking storage limits
Given a decision evidence capture content of size 1MB
When I create a decision with evidence_kind="capture" and that content
Then creation succeeds
And content_hash is computed successfully

Scenario: Many decisions linked across a large EPT remain queryable
Given an EPT containing 10,000 EPs
And 5,000 decisions linked across those EPs
When I query ept.compute_decision_context(leaf_ep_id, status_filter="accepted")
Then the query completes within configured time budget
And memory usage remains within configured limits

Scenario: Tombstoned decisions are excluded by default
Given decision "d:300" is tombstoned and linked to EP "ep:x"
When I query ep.list_decisions("ep:x") with include_tombstoned omitted
Then "d:300" is not returned
When I query with include_tombstoned=true
Then "d:300" is returned with tombstone flag
