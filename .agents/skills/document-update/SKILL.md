---
name: document-update
visibility: public
description: >
  Document consolidation and maintenance. Activates when user says "update docs",
  "consolidate docs", "documentation sweep", or "audit documentation". Primary
  goal: sweep information into core documents and archive the originals, reducing
  corpus size and eliminating redundancy. Works with plain files and shell tools
  — no running hKask instance required.
---

# Document Update Skill

You are a documentation consolidator. Your job is to reduce document sprawl by
merging information into core documents and archiving the originals. You work
with the file system, grep, and sed — no MCP server needed.

**Core principle:** Information belongs in the fewest authoritative documents
possible. Every document that isn't a consolidation target must justify its
existence by the deletion test: if you delete it, does unique information
disappear? If yes, merge that information into its parent target, then delete.

## When to Activate

- "update docs" / "documentation sweep" / "consolidate docs"
- "audit documentation" / "fix docs"
- Any task involving the document corpus

## The 5-Phase Workflow

### Phase 0 — Audit & Map

**Goal:** Understand what exists and what should absorb what.

Walk `docs/` (excluding `archive/` and `generated/`). For each document, note:
- Line count (thin docs are consolidation candidates)
- Whether it's a consolidation target (see below) or a child
- Whether it contains unique information or just rephrases its parent

**Consolidation targets** (these absorb related docs — never delete these):
| Target | Absorbs |
|--------|---------|
| `docs/architecture/hKask-architecture-master.md` | Architecture patterns, kata-kanban, loop-architecture, energy architecture, PUBLIC_SURFACE |
| `docs/architecture/core/PRINCIPLES.md` | P12-replicant-host-mandate (if wholly contained) |
| `docs/architecture/core/MDS.md` | MDS_SCAFFOLD (if MDS.md already covers directory/lifecycle) |
| `docs/architecture/core/TESTING_DISCIPLINE.md` | Testing methodology docs |
| `docs/architecture/core/FUNCTIONAL_SPECIFICATION.md` | CNS-domain-spec, CNS memory verb contracts |
| `docs/specifications/standards/DOCUMENTATION_STANDARDS.md` | DOCUMENT_OWNERSHIP, template-header-standard |
| `docs/plans/deployment-and-backup.md` | DEPLOYMENT guide, admin-install-guide |

**Verification:** `find docs/ -name '*.md' -not -path '*/archive/*' -not -path '*/generated/*' | wc -l` — know the starting count. Target: reduce by at least 20%.

### Phase 1 — Identify Candidates

**Goal:** Flag documents that should be merged.

For each non-target document, ask:
1. **Does it contain information not present in its target?** → Extract unique content
2. **Is it a stub/redirect?** → Delete immediately (don't archive)
3. **Is it a historical status report?** → Archive, don't merge
4. **Does it overlap with another non-target?** → Merge them together, then merge into target
5. **Is it a standalone guide/spec with no overlap?** → Keep

Output a merge plan as a simple list:
```
MERGE: kata-kanban-integration.md → hKask-architecture-master.md (unique: kata-kanban pattern)
MERGE: loop-architecture.md → hKask-architecture-master.md (unique: four-loop decomposition, rate-limiting subsumption)
MERGE: energy-gas-payments-api-keys.md → hKask-architecture-master.md (unique: energy budget model)
MERGE: energy_accounting_hardening_audit.md → hKask-architecture-master.md (unique: tamper-evidence audit)
ARCHIVE: cargo-bolero-qa-plan.md (historical plan, not active architecture)
```

**Verification:** Every non-target document has a disposition: MERGE, ARCHIVE, or KEEP. No document is left unclassified.

### Phase 2 — Extract & Merge

**Goal:** Move unique information into target documents without losing anything.

For each MERGE candidate:
1. Read the child document fully
2. Read the target document to find the right insertion point
3. Identify information in the child that does NOT appear in the target
4. Add that information to the target as a new section or subsection
5. Preserve code references, diagrams, and citations from the child
6. If information conflicts (child says X, target says Y), flag for manual resolution — do not silently pick one

**Rules for merging:**
- Add a `> **Incorporated from:** <child-path>` attribution at the start of merged sections
- Keep section structure: each merged doc becomes a `##` or `###` section in the target
- Don't duplicate: if the target already covers a topic adequately, skip it
- Mermaid diagrams from the child should be moved to the target
- Code references (`crates/hkask-*/src/...`) must be verified against actual file paths

**Verification:** After merging, the target document contains all unique information from the child. The child can be deleted without information loss.

### Phase 3 — Archive

**Goal:** Remove merged documents and update cross-references.

For each merged document:
1. `git rm` the child document (or move to `docs/archive/` if preserving history)
2. Find all cross-references to the child across the corpus:
   ```bash
   grep -rl "child-filename" docs/ --include="*.md" | grep -v archive
   ```
3. Update each reference to point to the target document and section
4. Run `bash docs/ci/check-links.sh` to verify no broken links

**For stubs and redirects:** Delete immediately — don't waste time archiving a 6-line file that says "see other doc."

**Verification:** `bash docs/ci/check-links.sh` passes with zero broken links. `grep -r "deleted-filename" docs/ --include="*.md"` returns zero matches outside archive/ and historical TODO/status files.

### Phase 4 — Portal Refresh

**Goal:** Update navigation indexes to reflect the new corpus.

1. Update `docs/README.md`:
   - Remove entries for archived/deleted docs
   - Update entries for merged docs (new paths/sections)
   - Update the active document count in the footer
2. Update `docs/architecture/hKask-architecture-master.md`:
   - Remove archived references from the document tree
   - Update the total document count
3. Update `docs/DIAGRAMS_INDEX.md` if diagram references changed

**Verification:** Starting from `docs/README.md`, every listed document exists. `find docs/ -name '*.md' -not -path '*/archive/*' -not -path '*/generated/*' | wc -l` shows the reduced count.

## Consolidation Targets (Definitive)

These 7 documents are the "sinks" — they absorb related content:

| # | Target | Role |
|---|--------|------|
| 1 | `hKask-architecture-master.md` | Authoritative architecture index — absorbs all architecture pattern docs |
| 2 | `PRINCIPLES.md` | P1-P12 with elaboration — absorbs related mandates |
| 3 | `MDS.md` | MDS specification framework — absorbs MDS_SCAFFOLD |
| 4 | `TESTING_DISCIPLINE.md` | All testing methodology — single source of truth |
| 5 | `FUNCTIONAL_SPECIFICATION.md` | AgentService functional spec — absorbs CNS domain specs |
| 6 | `DOCUMENTATION_STANDARDS.md` | All documentation policies — absorbs ownership, template standards |
| 7 | `deployment-and-backup.md` | All deployment architecture — absorbs deployment guides |

Documents NOT in this list are consolidation candidates unless they are:
- Standalone specifications (wallet-spec, REPL-spec, salience-spec, etc.)
- User guides (these serve a different audience)
- ADRs (immutable decision records)
- Status reports (historical snapshots — archive, don't merge)

## utoipa / API Documentation

API documentation is auto-generated from code:
- `docs/generated/openapi.json` — OpenAPI 3.1 spec (generated by utoipa from `#[derive(ToSchema)]` annotations)
- `docs/generated/cli-reference.md` — CLI reference (generated from clap annotations)

The `docs/architecture/reference/utoipa-implementation.md` document describes *how* utoipa is wired into hKask. It should be a brief section in the architecture master, not a standalone document. The actual API reference is the generated OpenAPI spec and `cargo doc` output.

Do not manually maintain API documentation that can be generated from code. When the code changes, regenerate — don't update prose.

## User Guide Adequacy

When auditing user guides, check:
1. **Coverage:** Does every `kask` subcommand have a guide section? (check `kask --help` output)
2. **Accuracy:** Do the commands shown actually work? (test them)
3. **Completeness:** Does the guide cover error recovery, not just happy path?
4. **Freshness:** Is `last_updated` within the last 30 days, and does the content match current CLI behavior?

Guides that fail these checks should be flagged for revision — not silently kept.

## Anti-Patterns

1. Adding frontmatter to documents instead of consolidating them — metadata alignment is not consolidation
2. Creating new "overview" or "index" documents instead of merging into existing targets
3. Using MCP server tools (`spec_graph_query`, etc.) when grep and file reads suffice
4. Archiving without merging unique content first — information loss
5. Merging documents that serve different audiences (e.g., don't merge a user guide into an architecture spec)
6. Keeping stubs and redirects — delete them immediately
7. Adding "formerly" or "previously known as" annotations — git history is the archive

## Success Criteria

A documentation sweep is successful when:
- Active document count is ≤40 (from current ~60)
- Zero stubs or redirect documents exist
- `bash docs/ci/check-links.sh` passes with zero broken links
- Every consolidation target has absorbed its children's unique content
- No information was lost (verify by diffing pre-merge child against post-merge target section)
