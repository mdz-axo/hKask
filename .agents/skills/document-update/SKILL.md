---
name: document-update
visibility: public
description: >
  Document consolidation and maintenance. Activates when user says "update docs",
  "consolidate docs", "documentation sweep", or "audit documentation". Primary
  goals: (1) fact-check all README files against codebase ground truth, (2) sweep
  information into core documents and archive the originals, reducing corpus size
  and eliminating redundancy. Works with plain files and shell tools — no running
  hKask instance required.
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

## The 7-Task Workflow

### Task 0 — README Fact Audit (NEW — run first, every sweep)

**Goal:** Verify every numeric assertion, crate listing, version number, and count in every README-like file in the project. These files rot silently because they contain hand-maintained stats that nobody updates when code changes.

**README-like files that MUST be audited every sweep:**

| File | Contains |
|------|----------|
| `./README.md` | Version, crate counts, LOC, subcommand/route counts, phase status, MCP server list, skill counts, doc counts |
| `./AGENTS.md` | Crate listing, command examples, verification commands |
| `./docs/README.md` | Document portal index, active doc count, section links |
| `./docs/status/PROJECT_STATUS.md` | Project phase, completion percentages, component status |
| `./mcp-servers/hkask-mcp-condenser/README.md` | Tool counts, test counts, dependency references |

#### Fact-Checking Protocol

For each README file, verify every assertion against the codebase. Do not trust any number. Extract ground truth from the codebase, not from other docs.

**Version number:**
```bash
grep '^version' Cargo.toml | head -1
# Compare against version in README header, footer, and docs references
```

**Crate counts:**
```bash
# Count workspace crates (exclude fuzz targets)
grep 'crates/hkask-' Cargo.toml | grep -v fuzz | wc -l
# Count MCP servers
grep 'mcp-servers/hkask-' Cargo.toml | grep -v fuzz | wc -l
# Verify every listed crate exists
ls -d crates/hkask-*/
ls -d mcp-servers/hkask-mcp-*/
```

**Line counts (LOC):**
```bash
# Core src/ only (exclude fuzz/ and tests/ if measuring separately)
find crates/ -path '*/src/*.rs' -not -path '*/fuzz/*' | xargs wc -l | tail -1
# MCP server src/ only
find mcp-servers/ -path '*/src/*.rs' -not -path '*/fuzz/*' | xargs wc -l | tail -1
# Per-crate breakdown (for README tables)
for crate in crates/hkask-*/; do
  name=$(basename "$crate")
  count=$(find "$crate/src" -name '*.rs' 2>/dev/null | xargs wc -l 2>/dev/null | tail -1 | awk '{print $1}')
  echo "  $name: ${count:-0}"
done
```

**CLI subcommand count:**
```bash
./target/release/kask --help 2>&1 | grep -A50 'Commands:' | grep -c '^  [a-z]'
# Or count from source:
grep -c 'pub enum Command' crates/hkask-cli/src/cli/mod.rs 2>/dev/null
```

**API route group count:**
```bash
ls crates/hkask-api/src/routes/*.rs | wc -l
```

**Test file count:**
```bash
# Files containing #[cfg(test)] modules (exclude fuzz/)
find crates/ mcp-servers/ -name '*.rs' -not -path '*/fuzz/*' | xargs grep -l '#\[cfg(test)\]' 2>/dev/null | wc -l
```

**Skill counts:**
```bash
ls .agents/skills/ | wc -l                              # installed skills
find registry/ -name 'manifest.yaml' | wc -l            # registry crates
find registry/ -name '*.j2' | wc -l                     # Jinja2 templates
```

**Document counts:**
```bash
find docs/ -name '*.md' -not -path '*/archive/*' -not -path '*/generated/*' | wc -l   # active
find docs/archive/ -name '*.md' | wc -l                                               # archived
```

**Crate existence (every listed crate must exist):**
```bash
# For each crate named in README: test -d "crates/$name" || echo "MISSING: $name"
# For each MCP server named in README: test -d "mcp-servers/$name" || echo "MISSING: $name"
```

**Stale references (names that no longer exist):**
```bash
# Search for old crate names, old version strings, removed concepts
grep -rni 'hkask-ensemble\|cybertest\|v0\.28\|v0\.29' README.md docs/README.md AGENTS.md
# Each hit is a correction candidate
```

#### Correction Rules

1. **Numbers must match exactly** (or be rounded to the nearest 1000 for LOC with `~` prefix).
2. **Crate names must match the filesystem** — no renamed-but-not-updated names.
3. **Phase status must reflect current reality** — if condenser is done, mark it done.
4. **Command examples must work** — run them to verify.
5. **Remove stale sections** — if a referenced crate/feature doesn't exist, remove or update the section.
6. **Don't add new content** — this task is audit-and-correct, not expand.

**Verification:** After corrections, re-run all fact-checking commands. Every assertion in every README must be verifiable from the codebase. Zero stale references. Version matches Cargo.toml.

---

### Task 1 — Audit & Map

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

### Task 2 — Identify Candidates

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

### Task 3 — Extract & Merge

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

### Task 4 — Archive

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

### Task 5 — Portal Refresh

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

## README Files — Update vs Regenerate

README files are special: they contain both **machine-verifiable facts** (counts, versions, crate names) and **human-authored content** (vision, design philosophy, roadmap). The audit-and-correct approach (Task 0) is preferred over deletion+regeneration because:

- Human-authored prose (Vision, Design Philosophy, Hallucinations to Avoid) cannot be generated from code
- Roadmap/phase status requires human judgment about what is "done"
- Success criteria and anchor descriptions are aspirational, not purely factual

However, if a README becomes **more than 50% stale** (half its assertions are wrong), consider:
1. Running Task 0 to identify every stale assertion
2. Rewriting the entire README from scratch against current codebase ground truth
3. Deleting the old README only after the new one is committed

Do not delete a README without a replacement. Do not archive READMEs — they are not docs, they are project portals.

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
- **Task 0 complete:** All README files pass fact-checking — version matches Cargo.toml, all counts verified, zero stale crate names or references
- Active document count is ≤40 (from current ~56)
- Zero stubs or redirect documents exist
- `bash docs/ci/check-links.sh` passes with zero broken links
- Every consolidation target has absorbed its children's unique content
- No information was lost (verify by diffing pre-merge child against post-merge target section)
