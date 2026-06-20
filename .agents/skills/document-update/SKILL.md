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

### Task 0 — Docs-vs-Code Reconciliation (NEW — run first, every sweep)

**Goal:** Every document assertion must be verifiable against codebase ground truth. Documents follow code — not the reverse. When code changes (crate added/removed/renamed, command added/removed, API surface changes), the docs MUST be updated.

**Principle:** Don't search for known stale patterns. Extract the set of "what exists" from the codebase, then find everything in docs that references something that doesn't.

**Step 1: Build the ground-truth set from code.**

```bash
# Crate names (from workspace)
grep -oP 'crates/hkask-[a-z0-9_-]+' Cargo.toml | cut -d/ -f2 | sort -u > /tmp/exists-crates.txt
grep -oP 'mcp-servers/hkask-mcp-[a-z0-9_-]+' Cargo.toml | cut -d/ -f2 | sort -u > /tmp/exists-mcps.txt

# CLI subcommand names
./target/release/kask --help 2>&1 | grep -A100 'Commands:' | grep -oP '^  \K[a-z][a-z-]+' | sort -u > /tmp/exists-commands.txt

# Skill names
ls .agents/skills/ | sort -u > /tmp/exists-skills.txt

# API route names
ls crates/hkask-api/src/routes/ | sed 's/\.rs$//' | sort -u > /tmp/exists-routes.txt

# Active doc files (to verify doc cross-references)
find docs/ -name '*.md' -not -path '*/archive/*' | sed 's|^docs/||' | sort -u > /tmp/exists-docs.txt
```

**Step 2: Extract all references from docs.**

```bash
# All crate-like names mentioned in docs
grep -roPh 'hkask-[a-z0-9_-]+' docs/ --include='*.md' | sort -u > /tmp/refs-crates.txt

# All CLI commands mentioned in docs (look for `kask <word>` patterns)
grep -roPh '(?<=\bkask )[a-z][a-z-]+' docs/ --include='*.md' | sort -u > /tmp/refs-commands.txt
```

**Step 3: Diff — find references to things that don't exist.**

```bash
# Crates mentioned in docs but missing from Cargo.toml
echo "=== Stale crate references in docs ==="
comm -23 /tmp/refs-crates.txt /tmp/exists-crates.txt | while read name; do
  echo "  STALE: $name"
  grep -rl "$name" docs/ --include='*.md' | head -5
  echo ""
done

# CLI commands mentioned in docs but not actual subcommands
echo "=== Stale command references in docs ==="
comm -23 /tmp/refs-commands.txt /tmp/exists-commands.txt | while read cmd; do
  echo "  STALE: kask $cmd"
  grep -rl "kask $cmd" docs/ --include='*.md' | head -5
  echo ""
done
```

**Step 4: Verify the specific README files.**

For `./README.md`, `./AGENTS.md`, `./docs/README.md`, `./docs/status/PROJECT_STATUS.md`:
- Version number vs `grep '^version' Cargo.toml`
- Every numeric assertion (LOC, counts, test numbers) vs codebase
- Every listed crate name exists in Cargo.toml
- Every listed MCP server exists in filesystem
- Phase/roadmap status reflects current reality
- No references to removed crates, commands, or concepts

**Step 5: Missing documentation check — every component MUST have a README.**

```bash
# Every MCP server must have a README.md
for dir in mcp-servers/hkask-mcp-*/; do
  if [ ! -f "$dir/README.md" ]; then
    echo "MISSING README: $(basename $dir)"
  fi
done

# Every crate must have a README.md (or document why not)
for dir in crates/hkask-*/; do
  if [ ! -f "$dir/README.md" ]; then
    echo "MISSING README: $(basename $dir)"
  fi
done
```

**Rule:** Every public-facing component (MCP server, core crate) MUST have a `README.md`. Internal implementation crates (services, adapters) may defer, but the absence must be intentional and documented. When a new crate or MCP server is added, its README must be created in the same commit.

The README must contain at minimum:
- Component name and one-line purpose
- Tool/function listing (for MCP servers: every tool by name)
- Configuration requirements (environment variables, dependencies)

**Step 6: Frontmatter freshness — `last_updated` and `version` in every doc.**

```bash
# Check frontmatter version matches Cargo.toml
cargo_version=$(grep '^version' Cargo.toml | grep -oP '[0-9]+\.[0-9]+\.[0-9]+')
for f in $(find docs/ -name '*.md' -not -path '*/archive/*'); do
  doc_version=$(grep -oP 'version:\s*"?\K[0-9]+\.[0-9]+\.[0-9]+' "$f" 2>/dev/null)
  if [ -n "$doc_version" ] && [ "$doc_version" != "$cargo_version" ]; then
    echo "VERSION MISMATCH: $f → $doc_version (should be $cargo_version)"
  fi
done

# Check last_updated is not stale (>30 days)
thirty_days_ago=$(date -d '30 days ago' +%s)
for f in $(find docs/ -name '*.md' -not -path '*/archive/*'); do
  doc_date=$(grep -oP 'last_updated:\s*\K[0-9]{4}-[0-9]{2}-[0-9]{2}' "$f" 2>/dev/null)
  if [ -n "$doc_date" ]; then
    doc_epoch=$(date -d "$doc_date" +%s 2>/dev/null)
    if [ -n "$doc_epoch" ] && [ "$doc_epoch" -lt "$thirty_days_ago" ]; then
      echo "STALE: $f last_updated $doc_date (>30 days)"
    fi
  fi
done
```

**Step 7: Tool count verification — MCP server READMEs must list every tool.**

```bash
# For each MCP server, compare README tool count to code
for dir in mcp-servers/hkask-mcp-*/; do
  name=$(basename "$dir")
  readme_tools=$(grep -c '| \`' "$dir/README.md" 2>/dev/null || echo 0)
  code_tools=$(grep -rn 'pub async fn' "$dir/src" --include='*.rs' 2>/dev/null | wc -l)
  if [ "$readme_tools" -lt "$code_tools" ]; then
    echo "TOOL GAP: $name — README lists $readme_tools, code has $code_tools pub async fn"
  fi
done
```

**Correction Rules:**

1. **Delete stale references** — if a doc mentions a crate/command/feature that no longer exists, remove or update.
2. **Numbers must match** — rerun the counting command and update.
3. **Crate names must match filesystem** — no renamed-but-not-updated names.
4. **Phase status must reflect current reality** — don't claim something is "in progress" if it shipped.
5. **Documents follow code** — Magna Carta and PRINCIPLES excepted (those constrain code).

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
- **Task 0 complete:** All README files pass fact-checking — version matches Cargo.toml, all counts verified, zero stale crate names or references. Every MCP server has a README. Every core crate has a README (or documented exemption). All frontmatter `last_updated` dates are ≤30 days. All MCP server README tool counts match code.
- Active document count is ≤40 (from current ~56)
- Zero stubs or redirect documents exist
- `bash docs/ci/check-links.sh` passes with zero broken links
- Every consolidation target has absorbed its children's unique content
- No information was lost (verify by diffing pre-merge child against post-merge target section)
