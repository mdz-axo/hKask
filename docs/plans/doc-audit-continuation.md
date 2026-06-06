# Document Audit & Update — Continuation Prompt

## Session Purpose

Audit and update all hKask documentation for internal consistency with the current codebase. Multiple agent sessions have changed code (credential names, tool counts, crate sizes, consolidation surfaces) without updating the corresponding docs. The staleness pattern is general: status docs, architecture references, and specifications all lag behind the source of truth (the Rust code).

**Rule:** When docs and code disagree, the code is correct. Update the docs.

---

## What Changed in the Codebase (Source of Truth)

### Credential Unification

All memory-related MCP servers now use `HKASK_MEMORY_DB` (pointing to the per-agent memory DB `hkask-memory-{agent}.db`):

| MCP Server | Old Credential | New Credential | Status |
|---|---|---|---|
| `hkask-mcp-episodic` | `HKASK_EPISODIC_DB` | `HKASK_MEMORY_DB` (required) | ✅ Fixed |
| `hkask-mcp-semantic` | `HKASK_MEMORY_DB` | `HKASK_MEMORY_DB` (required) | ✅ Fixed |
| `hkask-mcp-doc-knowledge` | `HKASK_SEMANTIC_DB` | `HKASK_MEMORY_DB` (optional) | ✅ Fixed |
| `hkask-mcp-condenser` | `HKASK_DB_PATH` | `HKASK_MEMORY_DB` (optional) | ✅ Fixed |

Non-memory MCP servers use domain-specific credentials that are **correct and should not be changed**:
- `HKASK_DB_PATH` in `hkask-cli/src/commands/config.rs` and `hkask-mcp-replicant/src/agent_loader.rs` → **registry** DB (`hkask.db`), NOT the memory DB. Leave these alone.
- `HKASK_GOAL_DB`, `HKASK_RSS_DB`, `HKASK_SPEC_DB_PATH`, `HKASK_REGISTRY_DB`, `HKASK_OCAP_SECRET`, `HKASK_FAL_API_KEY`, `HKASK_FMP_API_KEY`, `HKASK_TELNYX_API_KEY`, `HKASK_GITHUB_TOKEN`, `HKASK_WEB_*` → all correct, leave alone.

### Tool Count Changes

| MCP Server | Old Tool Count | New Tool Count | What Changed |
|---|---|---|---|
| `hkask-mcp-episodic` | 4 | 5 | Added `episodic_consolidate_status` (read-only status tool) |
| `hkask-mcp-semantic` | 6 | 10 | Added `semantic_centroid`, `semantic_purge`, `semantic_chunk`, `semantic_consolidate`; restored `EpisodicMemory` + `ConsolidationBridge` fields for consolidation |

### Consolidation Surface Changes

Four surfaces now perform consolidation (same `ConsolidationService` algorithm):

1. **CLI** — `kask consolidate --passphrase` (passphrase-verified)
2. **API** — `POST /api/consolidate` (passphrase-verified + rate-limited)
3. **Chat** — `/consolidate run` (single-user, no passphrase)
4. **MCP Semantic** — `semantic_consolidate` (OCAP-gated, no passphrase needed)

The MCP episodic server has a read-only status tool (`episodic_consolidate_status`) that reports candidates but cannot consolidate (only has `EpisodicMemory`).

### LOC Changes

| MCP Server | Old LOC (audit doc) | Current LOC |
|---|---|---|
| `hkask-mcp-episodic` | 190 | 219 |
| `hkask-mcp-semantic` | 290 | 532 |
| `hkask-mcp-condenser` | 761 | 291 |

### ADR-031 Already Updated

`docs/architecture/ADR-031-consolidation-authorization.md` is at v2.0.0 and correctly reflects the 4-surface consolidation architecture. Do NOT re-edit it.

### `domain-and-capability.md` §10.8 Already Updated

The surfaces table in §10.8 was already updated to show MCP Semantic can consolidate. Do NOT re-edit it.

---

## Documents to Update

### Priority 1 — Status Docs (directly contradict code)

**`docs/status/mcp-server-audit.md`** (v1.2.0, last_updated 2026-06-03)

| Line(s) | Current | Correct |
|---|---|---|
| 40 | `hkask-mcp-episodic` 190 LOC, 4 tools | 219 LOC, 5 tools |
| 41 | `hkask-mcp-semantic` 290 LOC, 6 tools | 532 LOC, 10 tools |
| 78 | `hkask-mcp-condenser` 761 LOC, 5 tools | 291 LOC, 5 tools |
| 138–141 | Episodic: 4 tools, list missing `episodic_consolidate_status` | 5 tools, add `episodic_consolidate_status` to list |
| 143–145 | Semantic: 6 tools, list missing centroid/purge/chunk/consolidate | 10 tools, add `semantic_centroid`, `semantic_purge`, `semantic_chunk`, `semantic_consolidate` |
| 216 | `HKASK_DB_PATH` for replicant registry | This is correct (registry DB, not memory DB) — but add a clarifying note that this is the registry DB, distinct from `HKASK_MEMORY_DB` |
| Throughout | No credential table for episodic/semantic | Episodic requires `HKASK_MEMORY_DB` + `HKASK_DB_PASSPHRASE`; semantic requires same |

**`docs/status/mcp-tools-inventory.md`** (v1.0.0, last_updated 2026-06-04)

| Line(s) | Current | Correct |
|---|---|---|
| 30 | Episodic: 4 tools | 5 tools |
| 31 | Semantic: 6 tools | 10 tools |
| 107–114 | Episodic tool list missing `episodic_consolidate_status` | Add it |
| 116–125 | Semantic tool list missing `semantic_centroid`, `semantic_purge`, `semantic_chunk`, `semantic_consolidate` | Add all 4 |
| 332 | "Servers without credential requirements: … episodic, semantic …" | Remove `episodic` and `semantic` from this list — both require `HKASK_MEMORY_DB` + `HKASK_DB_PASSPHRASE` |

**`docs/status/PROJECT_STATUS.md`** (v0.21.5, last_updated 2026-06-03)

- Version should be `0.22.0` (current workspace version per `Cargo.toml`)
- Any consolidation-related sections need to reflect the 4-surface architecture
- Scan for tool counts or credential names that lag

### Priority 2 — Architecture References (may contain stale details)

**`docs/architecture/loop-architecture.md`** (v2.3.0, last_updated 2026-06-03)

- Line 110: "memory is consolidated … handled by the consolidation bridge" — still accurate
- Line 398: "consolidation bridge (one-way, ConsolidationToken)" — still accurate
- Line 425: consolidation bridge description — still accurate
- **But:** No mention that MCP semantic server provides `semantic_consolidate`. Add a note if the document describes MCP server responsibilities.

**`docs/architecture/persistence-and-lifecycle.md`** (v2.2.1, last_updated 2026-05-29)

- Line 33: "4,010 LOC" for `hkask-storage` — verify current LOC
- Line 134: "695 LOC" for `hkask-memory` — verify current LOC
- No mention of `HKASK_MEMORY_DB` credential — add if the document describes database configuration

**`docs/architecture/reference/distillation-erd.md`** (v0.22.0, last_updated 2026-06-02)

- Line 25: "EPI →|consolidation bridge|→ SEM" — accurate
- Line 35: "consolidation bridge (EPI→SEM) is one-way, gated by ConsolidationToken" — accurate
- No changes likely needed unless the ERD should show which MCP servers have both memory types

**`docs/architecture/reference/ports-inventory.md`** (v0.21.0-p4-parity, last_updated 2026-05-28)

- Very stale version (0.21.0 vs current 0.22.0). Scan for port changes.
- Check if `ConsolidationPort` and `ConsolidationToken` are documented correctly.

**`docs/architecture/hKask-architecture-master.md`** (v2.2.2, last_updated 2026-06-03)

- ADR-031 reference — already correct
- Scan for any tool count or credential references

**`docs/architecture/PRINCIPLES.md`** (v1.1.0, last_updated 2026-05-28)

- Lines 29, 69, 98, 140, 171: "21 MCP servers" — updated from 19 (doc-knowledge + markitdown added)
- Scan for any consolidation or credential references that lag

**`docs/specifications/REQUIREMENTS.md`** (v1.2.0, last_updated 2026-05-29)

- Scan for MCP ≡ CLI ≡ API equivalence claims that should now include MCP semantic consolidation
- Scan for credential name references

### Priority 3 — Generated/User-Facing Docs

**`docs/generated/cli-reference.md`** — Verify `kask consolidate` command docs are current

**`docs/user-guides/AGENT-POD-CREATION-GUIDE.md`** and `docs/user-guides/COMMON-AGENT-PATTERNS.md`** — Scan for stale credential names or tool references

**`docs/OPEN_QUESTIONS.md`** — Scan for consolidation-related open questions that are now resolved

---

## How to Verify

After updating each doc, cross-reference against the Rust source:

```bash
# Verify tool counts
grep -c "#\[tool(" mcp-servers/hkask-mcp-episodic/src/main.rs   # expect 5
grep -c "#\[tool(" mcp-servers/hkask-mcp-semantic/src/main.rs   # expect 10
grep -c "#\[tool(" mcp-servers/hkask-mcp-condenser/src/main.rs  # expect 5

# Verify credential names
grep "HKASK_" mcp-servers/hkask-mcp-episodic/src/main.rs
grep "HKASK_" mcp-servers/hkask-mcp-semantic/src/main.rs
grep "HKASK_" mcp-servers/hkask-mcp-condenser/src/main.rs
grep "HKASK_" mcp-servers/hkask-mcp-doc-knowledge/src/main.rs

# Verify no stale credential names anywhere
grep -r "HKASK_EPISODIC_DB\|HKASK_SEMANTIC_DB" --include="*.rs" --include="*.yaml" --include="*.md"
```

## Key Design Principles

1. **Consolidation = same algorithm everywhere.** CLI, API, Chat, and MCP Semantic all use `ConsolidationService` with `ConsolidationBridge` connecting `EpisodicMemory` + `SemanticMemory` from the same per-agent memory DB.

2. **`HKASK_MEMORY_DB` is the unified credential.** All 4 memory-related MCP servers use it. The only `HKASK_DB_PATH` remaining is for the **registry** DB (`hkask.db`), which is a completely different database.

3. **MCP Episodic is read-only for consolidation status.** It only has `EpisodicMemory` and reports candidates via `episodic_consolidate_status`. It cannot promote triples.

4. **MCP Semantic has full consolidation.** It builds both `EpisodicMemory` and `SemanticMemory` from `HKASK_MEMORY_DB` and exposes `semantic_consolidate`. The OCAP GovernedTool membrane replaces passphrase verification for this surface.

5. **Docs that are already correct should not be re-edited.** ADR-031 (v2.0.0) and `domain-and-capability.md` §10.8 are current — leave them alone.