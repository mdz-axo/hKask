---
title: "Condensation Continuation — MCP Server Consolidation"
audience: [architects, developers]
last_updated: 2026-06-09
version: "0.27.0"
status: "Complete"
domain: "Architecture"
mds_categories: [composition, trust]
---

# Condensation Continuation — MCP Server Consolidation

**Status:** Complete (2026-06-09). MCP server consolidation reduced 21→10 servers. Internal servers (inference, CNS, OCAP, keystore, registry, git, goals) removed from MCP workspace and callers updated to use direct crate calls. Replicant and ensemble converted to ACP ports.

---

## Background

PRINCIPLES.md §1.2 lists 21 MCP servers. The user's grill-me established:

> "The MCP servers which are suspect or which do not need to be fully integrated are... CNS communications should all be internal... episodic and semantic memory should be collapsed into a single server... MCP ensemble is redundant... MCP inference is similarly excess redundancy as are goal, keystore, ocap, registry and replicant."

## Target State

### Keep (9 — External Tool Integration)

These provide unique external API surfaces that cannot be internalized:

| Server | Purpose | Risk |
|--------|---------|------|
| `hkask-mcp-web` | Search, scrape, extract | Low — external API |
| `hkask-mcp-fmp` | FMP integration | Low — external API |
| `hkask-mcp-condenser` | Context condensation | Low — external Okapi dependency |
| `hkask-mcp-doc-knowledge` | Document parsing/chunking | Low — external dependency |
| `hkask-mcp-markitdown` | Document conversion + OCR | Low — external dependency |
| `hkask-mcp-fal` | FAL integration | Low — external API |
| `hkask-mcp-telnyx` | Telnyx integration | Low — external API |
| `hkask-mcp-rss-reader` | RSS feeds | Low — external API |
| `hkask-mcp-spec` | MDS spec capture | Low — spec server |

### Merge (2→1)

| Servers | Result | Rationale |
|---------|--------|-----------|
| `hkask-mcp-episodic` + `hkask-mcp-semantic` | `hkask-mcp-memory` | Episodic and semantic are facets of the same chat thread. Consolidation bridge already handles the episodic→semantic transition. One server with both endpoints. Cloud backup (currently GitHub) becomes a tool/option within memory. |

### Delete (9 — Internal Functions)

These are internal loops or functions that should not expose MCP ports:

| Server | Why Delete | Replacement |
|--------|-----------|-------------|
| `hkask-mcp-cns` | CNS is internal autonomic regulation | CNS runtime accessed directly via `CnsRuntime` |
| `hkask-mcp-git` | Git is internal CAS storage | `GitCasAdapter` accessed directly |
| `hkask-mcp-inference` | Inference is internal cognition | Okapi accessed directly via `InferencePort` |
| `hkask-mcp-goal` | Goal is internal coordination | `GoalRepository` accessed directly |
| `hkask-mcp-keystore` | Keystore is internal security | `Keychain` accessed directly |
| `hkask-mcp-ocap` | OCAP is internal enforcement | `GovernedTool` + `SovereigntyChecker` |
| `hkask-mcp-registry` | Registry is internal composition | `Registry` accessed directly |
| `hkask-mcp-replicant` | Replicant is ACP bridge | Replaced by ACP ports (not MCP) |
| `hkask-mcp-ensemble` | Ensemble is multi-agent chat | Replaced by ACP ports (not MCP) |

### GitHub Backup

GitHub backup (currently `hkask-mcp-github`) should become a tool or option within `hkask-mcp-memory`, not a standalone MCP server:

> "github - we need cloud backup - but this doesn't need to be github and it doesn't require the whole github mcp but should be more focused with memory backup as part of a memory mcp server"

Delete `hkask-mcp-github`. Add backup as a tool within `hkask-mcp-memory`.

## Approach

### Phase 1 — Audit Dependencies

For each server to be deleted, map every caller:
1. CLI commands that invoke the server
2. API routes that invoke the server
3. Other MCP servers that depend on it
4. Loop systems that dispatch to it

### Phase 2 — Replace Internal Access

For each deleted internal server, replace MCP tool dispatch with direct function calls:
- `hkask-mcp-cns` → `CnsRuntime` methods
- `hkask-mcp-keystore` → `Keychain` functions
- `hkask-mcp-ocap` → `GovernedTool::enforce()` + `SovereigntyChecker::can_access()`
- `hkask-mcp-registry` → `Registry` methods
- `hkask-mcp-git` → `GitCasAdapter` methods
- `hkask-mcp-inference` → `InferencePort::generate()`
- `hkask-mcp-goal` → `GoalRepository` methods

### Phase 3 — Merge Memory Servers

1. Create `hkask-mcp-memory` combining episodic + semantic endpoints
2. Add cloud backup tool (GitHub or generic)
3. Wire consolidation bridge
4. Delete old episodic/semantic servers

### Phase 4 — Replace ACP-Exposed Servers

- `hkask-mcp-replicant` → ACP ports for replicant chat
- `hkask-mcp-ensemble` → ACP ports for multi-agent chat

### Phase 5 — Verify

1. Run `cargo check --workspace && cargo test --workspace` at each deletion
2. Verify all 9 kept servers still function
3. Verify memory consolidation pipeline still works
4. Verify ACP-based ensemble chat works

## Risks

1. **CNS MCP deletion:** CNS is currently accessible via MCP for health queries. Direct `CnsRuntime` access must provide equivalent observability.
2. **Inference MCP deletion:** Inference is currently dispatched via MCP. Direct `InferencePort` access must preserve energy budget tracking via `GovernedTool`.
3. **OCAP MCP deletion:** OCAP enforcement is currently gated via MCP tool dispatch. Direct function calls must preserve the dual gate (`require_capability` + `require_sovereignty`).
4. **Ensemble MCP deletion:** Multi-agent chat currently uses MCP ensemble. ACP ports must provide equivalent functionality before MCP ensemble is deleted.
5. **Memory merge:** Episodic and semantic memory have different visibility rules (private vs. public). The merged server must preserve both access patterns.

## Verification

```bash
cargo check --workspace
cargo test --workspace
cargo clippy --workspace -- -D warnings
# Verify MCP server count: ls mcp-servers/ | wc -l → 9
# Verify memory server: hkask-mcp-memory (episodic + semantic + backup)
# Verify ACP ensemble: agents join ensemble via ACP ports, not MCP
# Verify no remaining internal MCP servers: cns, git, inference, goal, keystore, ocap, registry, replicant
```

## Predecessor Tasks

All preceding condensation work should be complete before starting this:
- [x] Candidate #5: EnergyBudget rename
- [x] Candidate #1: Visibility 3→2
- [x] Candidate #2: NuEvent/Span — resolved
- [x] Candidate #3: LoopMessage→tokio — deferred
- [x] Candidate #4: Pod/Agent/Service — deferred
- [x] Documentation cleanup
- [x] MDS specification

---

*This continuation prompt captures all context needed for the MCP server consolidation.*
