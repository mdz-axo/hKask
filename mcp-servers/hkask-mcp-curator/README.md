# hkask-mcp-curator

Curator daemon MCP tools. Provides the system's regulatory interface: algedonic escalation management, semantic memory search, and curator observability.

## Tools (8)

| Tool | Description |
|------|-------------|
| `curator_algedonic_log` | Retrieve algedonic alert history |
| `curator_escalation_dismiss` | Dismiss an algedonic escalation |
| `curator_escalation_resolve` | Resolve an algedonic escalation |
| `curator_escalations` | List active algedonic escalations |
| `curator_memory_recall` | Recall episodic/semantic memory by query |
| `curator_ping` | Health check — curator daemon alive? |
| `curator_semantic_search` | Search consolidated semantic index |
| `run` | Standard MCP server bootstrap |

## Architecture

The curator MCP server is the programmatic interface to the Curator daemon's regulatory functions. It is distinct from `hkask-mcp-memory` (which handles raw episodic/semantic storage) — the curator server provides the **curated, consolidated** view used by the algedonic escalation pathway.

## CNS Spans

All curator tools emit `cns.curation.*` spans with the Curator WebID as the replicant host (P12 compliance).

## See Also

- [`architecture/reference/hKask-Curator-persona.md`](../../docs/architecture/reference/hKask-Curator-persona.md) — Curator persona specification
- [`PRINCIPLES.md`](../../docs/architecture/core/PRINCIPLES.md) §P5, P9, P12
