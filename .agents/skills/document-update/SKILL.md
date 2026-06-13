---
name: document-update
visibility: public
description: >
  Systematic documentation corpus maintenance workflow. Activates when user says
  "update docs", "documentation sweep", "align specs", "fix metadata", or "audit
  documentation". Wraps the hkask-mcp-spec server's 5-tool surface into a 7-task
  document-specific workflow: inventory classification, metadata alignment,
  writing quality gate, cross-corpus coherence, spec-code drift resolution,
  archive/consolidation, and portal refresh. Follows deep-module discipline
  (Ousterhout): ≤7 public operations, each doing one thing well. The deletion
  test: if you delete this skill, does the complexity of document updates
  reappear scattered across agent prompts? If yes, the skill earns its existence.
---

# Document Update Skill

You are a documentation corpus maintainer. Your job is to execute a systematic
7-task documentation sweep using the hKask documentation infrastructure:
`hkask-mcp-spec` (5-tool surface), `hkask-mcp-replica` (style tools), condenser
(persistence), and CI verification scripts.

## When to Activate

Activate this skill when the user says any of:
- "update docs"
- "documentation sweep"
- "align specs"
- "fix metadata"
- "audit documentation"
- "refresh portal"
- "check cross-references"

## The 7-Task Workflow

Execute tasks sequentially. Each task has defined Input, Action, Output,
Infrastructure, and Verification. Do not skip verification.

### Task 1 — Corpus Inventory & Lifecycle Classification

**Input:** `docs/` directory tree, `docs/specifications/MDS_SCAFFOLD.md` §3
lifecycle policy.

**Action:** Walk every `.md` file in `docs/architecture/`, `docs/specifications/`,
`docs/status/`, `docs/plans/`, `docs/user-guides/`. For each document, extract
the YAML frontmatter metadata block. Classify each document's lifecycle state
against the MDS_SCAFFOLD state machine (`Draft → Active → Deprecated →
Superseded → Removed`). Flag documents where:
- `status` field is stale (claims "Active" but `last_updated` predates a
  structural change)
- `mds_categories` is missing or references the deprecated 9-category DDMVSS
  taxonomy instead of the current 5-category MDS taxonomy
  (`[domain, composition, trust, lifecycle, curation]`)
- Document content references removed/superseded subsystems (violating
  `AGENTS.md` §2.4)

**Output:** `docs/status/corpus_inventory.yaml` mapping
`path → {status, mds_categories, last_updated, lifecycle_action}` where
`lifecycle_action ∈ {keep, deprecate, supersede, remove, fix_metadata}`.

**Infrastructure:** Use `spec/graph/query` with `query: "lifecycle"` and
`depth: 2` to discover cross-reference chains. Use `spec/goal/capture` with
`context: "lifecycle"` to register each lifecycle transition as a governed goal.

**Verification:** `bash docs/ci/check-metadata.sh` passes with zero violations.

### Task 2 — Metadata Header Alignment

**Input:** `corpus_inventory.yaml` from Task 1,
`docs/specifications/DOCUMENTATION_STANDARDS.md` §2 (6-field metadata header),
§11.1 (mds_categories extension).

**Action:** For every document flagged with metadata violations, apply surgical
edits to the YAML frontmatter. Specifically:
- (a) Ensure `mds_categories` uses the 5-category MDS taxonomy
  (`[domain, composition, trust, lifecycle, curation]`), not the deprecated
  9-category DDMVSS taxonomy
- (b) Update `last_updated` to the date of the edit
- (c) Set `status` to the correct lifecycle state
- (d) Ensure `version` matches the project version (`0.27.0`) for framework
  documents
Do not touch document body content — this task is metadata-only, following the
surgical-change principle (Karpathy).

**Output:** Updated frontmatter on all flagged documents.
`corpus_inventory.yaml` updated with `metadata_aligned: true` flags.

**Infrastructure:** The `spec/require/writing-quality` tool's Gentle dimension
(agent-correctness) provides the verification lens: would an agent consuming
only the metadata header route correctly to the right document?

**Verification:** `bash docs/ci/check-metadata.sh` passes.
`grep -r "ddmvss_categories" docs/` returns zero matches (all migrated to
`mds_categories`).

### Task 3 — Writing Quality Gate

**Input:** All specification documents in `docs/specifications/` and
`docs/architecture/`.

**Action:** For each specification document, invoke
`spec/require/writing-quality` via the `hkask-mcp-spec` server. The server
assesses 4 dimensions (Hopper/Lovelace/Schriver/Gentle) per
`WRITING_EXCELLENCE.md` §3. Documents scoring below 3/4 are flagged for
revision. For each flagged document, apply the structural discipline from
`WRITING_EXCELLENCE.md` §2.3: every section must follow
Statement→Evidence→Diagram→Implications. Add missing citations per §2.4
(architecture docs: ≥1 external source per `##` section; specifications: ≥1
external source or code-path verification). Split sentences exceeding 35 words
(§2.2). Replace passive voice with active (§2.2).

**Output:** `docs/status/writing_quality_report.yaml` with per-document scores
and revision actions taken. All specification documents at ≥3/4.

**Infrastructure:** The `hkask-mcp-replica` server's `replica_compose` tool can
generate revision prose in the project's formal-technical voice (third person,
present tense, definite assertions) when fed the Curator persona as the
exemplar. Use `replica_compare` to measure stylistic distance between revised
and canonical documents — target distance <0.3 from the Curator centroid.

**Verification:** Re-run `spec/require/writing-quality` on each revised
document; all return `meets_publication_standard: true`.

### Task 4 — Cross-Corpus Coherence Validation

**Input:** Full document corpus (all active `.md` files).

**Action:** Invoke `spec/graph/coherence` on the corpus. The server computes
Jaccard similarity of declared vs. registered verbs across all specs, flags
category gaps, and identifies incomplete specs. For each violation and
suggestion in the response, apply the corresponding fix:
- (a) Missing categories → ensure at least one document anchors each of the 5
  MDS categories
- (b) Incomplete specs → add missing criteria per the category-specific
  requirements in `DOCUMENTATION_STANDARDS.md` §11.3
- (c) Cross-reference violations → verify every `cross_references` entry in
  MDS template manifests resolves to an existing document

**Output:** `docs/status/coherence_report.yaml` with `coherence_score ≥ 0.7`
(the MDS threshold), zero category gaps, and all cross-references resolved.

**Infrastructure:** Use `spec/graph/query` with `query: "cross-reference"` and
`depth: 3` to trace reference chains and detect dangling links. The condenser's
`condenser_persist` tool records each coherence fix as an episodic memory
triple `(document, has_coherence_fix, action)` for future audit.

**Verification:** Re-run `spec/graph/coherence`; `coherence_score ≥ 0.7` and
`violations` array is empty.

### Task 5 — Spec-Code Drift Resolution

**Input:** `docs/status/spec-code-drift.yaml` (drift items),
`docs/status/curation-decisions.yaml` (decisions per item).

**Action:** For each drift item with a `Merge` or `Revise` decision, apply the
resolution to either code or spec as specified in the curation decision
rationale. This is a surgical task — touch only the files named in the drift
item's `spec_reference` and `code_reality` fields.
- For `spec_ahead` items: the spec describes something code doesn't have yet —
  add minimal stubs with `FocusingAssumption` annotations per the curation
  decision
- For `code_ahead` items: code has evolved past the spec skeleton — update spec
  to reflect actual method names and types
- For `divergent` items: naming or structural mismatch — apply the type-alias
  or spec-update resolution
Follow the Graydon Hoare pattern: each type exists because it models a domain
truth, not because the spec mentioned it. If a stub earns its existence only by
satisfying a drift item, mark it with
`// REQ: DRIFT-<id> — existence justified by spec-code alignment, deletion test pending`.

**Output:** All drift items resolved. `spec-code-drift.yaml` updated with
`status: resolved` and `resolved_at` timestamps. Zero remaining `spec_ahead`,
`code_ahead`, or `divergent` items.

**Infrastructure:** Use `cargo check -p hkask-types -p hkask-templates -p
hkask-storage -p hkask-agents` to verify stub additions compile. Use
`cargo test -p hkask-storage` to verify SpecStore method name alignment doesn't
break tests.

**Verification:** Re-run the drift detection method described in
`spec-code-drift.yaml` header: set-difference of named entities from spec
documents against `pub` API surfaces. Zero new drift items.

### Task 6 — Archive & Consolidation

**Input:** `corpus_inventory.yaml` from Task 1 (documents flagged `deprecate`,
`supersede`, `remove`).

**Action:** For each document flagged `deprecate` or `supersede`: move to
`docs/archive/YYYY-MM-DD-<label>/` where `<label>` is the document's title
slug. The `docs/archive/` directory is gitignored per
`DOCUMENTATION_STANDARDS.md` §3 — git history is the archive of record. For
documents flagged `remove`: `git rm` from the active tree. Update all
cross-references in remaining active documents to point to the replacement
document (for superseded) or remove the reference (for removed). No active-tree
document may contain `formerly`, `previously known as`, or
backward-compatibility annotations — git history serves this purpose.

**Output:** Clean active tree with zero deprecated/superseded/removed
documents. `docs/archive/` populated with date-stamped snapshots. Zero broken
cross-references.

**Infrastructure:** Use `spec/graph/query` with
`query: "<removed-document-path>"` to discover all inbound cross-references
before deletion. The condenser's `condenser_thread_summary` can generate a
one-paragraph archival note for each removed document, persisted via
`condenser_persist` as `(document, was_archived, reason)`.

**Verification:** `bash docs/ci/check-links.sh` passes with zero broken links.
`grep -r "formerly\|previously known as\|backward.compatible" docs/ --include="*.md"`
returns zero matches.

### Task 7 — Portal & Index Refresh

**Input:** Updated corpus from Tasks 1-6.

**Action:** Update `docs/README.md` (the Documentation Portal) to reflect the
current corpus:
- (a) Verify every document listed in the portal tables exists at its stated
  path
- (b) Add entries for any active documents not currently listed
- (c) Remove entries for archived/removed documents
- (d) Update the MDS category tags on each entry to match the document's actual
  `mds_categories` frontmatter
- (e) Update `docs/DIAGRAMS_INDEX.md` to reflect any diagram changes from
  Tasks 3-5
Update `docs/architecture/hKask-architecture-master.md` (the architecture
index) to point to current authoritative documents.

**Output:** Portal and index documents accurately reflect the active corpus.
Every document reachable from the portal in ≤2 clicks (Schriver findability:
30-second rule).

**Infrastructure:** Use `spec/graph/query` with `query: "portal"` and
`depth: 1` to verify the portal's link graph is complete. The
`spec/goal/capture` tool registers the portal refresh as a governed goal with
OCAP boundary `"portal:write requires CuratorId authority"`.

**Verification:** Manual walk: starting from `docs/README.md`, every document
in the active tree is reachable. `bash docs/ci/check-links.sh` passes.

## Deep-Module Discipline (Ousterhout)

This skill's public interface is exactly 7 operations (the 7 tasks above). Each
task does one thing well:
1. `inventory` — classify lifecycle states
2. `align-metadata` — fix frontmatter headers
3. `quality-gate` — assess and improve writing quality
4. `coherence` — validate cross-corpus consistency
5. `drift-resolve` — align spec with code
6. `archive` — remove deprecated documents
7. `refresh-portal` — update navigation indexes

**The deletion test:** If you delete this skill, does the complexity of
document updates reappear scattered across agent prompts? Yes — each task
requires knowledge of MDS_SCAFFOLD, DOCUMENTATION_STANDARDS, WRITING_EXCELLENCE,
spec-code-drift.yaml, curation-decisions.yaml, CI scripts, and the spec server
tool surface. Without this skill, an agent must rediscover these dependencies
on every documentation edit. The skill earns its existence.

## Registry Templates

This skill's runtime templates live in `registry/templates/document-update/`:

| Template | Type | Purpose |
|----------|------|--------|
| `doc-inventory.j2` | KnowAct | Walk corpus, extract frontmatter, classify lifecycle |
| `doc-align-metadata.j2` | KnowAct | Apply surgical metadata fixes to flagged documents |
| `doc-quality-gate.j2` | KnowAct | Invoke writing-quality assessment, apply revisions |
| `doc-coherence.j2` | KnowAct | Validate cross-corpus coherence, fix violations |
| `doc-drift-resolve.j2` | KnowAct | Resolve spec-code drift items per curation decisions |
| `doc-archive.j2` | KnowAct | Move deprecated documents to archive, fix cross-references |
| `doc-refresh-portal.j2` | KnowAct | Update portal and index to reflect current corpus |

The SKILL.md (this file) teaches the Zed coding agent the document-update
methodology. The `.j2` templates are executable process steps the hKask
runtime invokes during `kask chat` sessions.

## Anti-Patterns

1. Editing document body content during metadata-only tasks (Task 2)
2. Adding "formerly" or "previously known as" annotations (violates
   DOCUMENTATION_STANDARDS.md §3)
3. Creating archive indexes or migration guides (git history is the archive)
4. Skipping verification — every task has a verification command; run it
5. Touching files outside the task's scope (surgical changes only)
