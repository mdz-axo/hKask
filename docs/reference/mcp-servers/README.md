# MCP Server Registry

**Diataxis type:** Reference
**Status:** Current (v0.31.0)

Built-in MCP servers shipped with hKask. Each server is a thin surface over domain crates, following the standard bootstrap path (`hkask_mcp::bootstrap_mcp_server` → `hkask_mcp::run_server`).

## Server Catalog

| Server | Crate | Domain | Tools | Math Engine |
|--------|-------|--------|-------|-------------|
| Scenarios | `mcp-servers/hkask-mcp-scenarios` | Event-tree forecasting (Tetlock/Schwartz/Chermack) | 18 | `hkask-forecast` |
| Companies | `mcp-servers/hkask-mcp-companies` | FIBO-anchored financial forecasting | — | `hkask-forecast` |
| CodeGraph | `mcp-servers/hkask-mcp-codegraph` | Code understanding (query, traverse, impact) | 11 | `hkask-codegraph` |
| Curator | `mcp-servers/hkask-mcp-curator` | Curator agent metacognition | — | — |
| Memory | `mcp-servers/hkask-mcp-memory` | Episodic and semantic memory | — | — |
| DocProc | `mcp-servers/hkask-mcp-docproc` | Document processing and QA generation | — | — |
| Filesystem | `mcp-servers/hkask-mcp-filesystem` | File access and operations | — | — |
| Kata Kanban | `mcp-servers/hkask-mcp-kata-kanban` | Toyota Kata task boards | 14 | `hkask-services-kata-kanban` |
| Media | `mcp-servers/hkask-mcp-media` | Fal.ai media generation | — | — |
| Replica | `mcp-servers/hkask-mcp-replica` | Replicant lifecycle | — | — |
| Research | `mcp-servers/hkask-mcp-research` | Research paper search and reading | — | — |
| Skill | `mcp-servers/hkask-mcp-skill` | Skill registry access | — | — |
| Training | `mcp-servers/hkask-mcp-training` | LoRA training pipeline | — | — |
| Communication | `mcp-servers/hkask-mcp-communication` | Federation messaging | — | — |
| Condenser | `mcp-servers/hkask-mcp-condenser` | Context condensation | — | — |

## Common Patterns

All servers follow these patterns:

1. **Bootstrap:** `hkask_mcp::bootstrap_mcp_server(name, target, host_env_var)` → returns `MCPBootstrap { replicant, daemon_client }`
2. **Struct:** `hkask_mcp::mcp_server!` macro generates the struct with `webid`, `replicant`, `daemon` fields plus domain fields
3. **Tool dispatch:** `execute_tool_semantic(self, tool_name, ontology, async { ... })` wraps each tool with CNS span + daemon outcome recording
4. **Tool router:** `#[tool_handler(router = Self::...router())]` on the `ServerHandler` impl
5. **Error type:** `McpToolError` for tool-level errors, domain `Error` enums (via `thiserror`) for computation errors

## Cross-links

- [Scenarios Adversarial Review](../../status/scenarios-adversarial-review.md) — code smell inventory for the scenarios server
- [Scenario Forecasting Pipeline Diagram](../../diagrams/flowchart-scenario-forecasting-pipeline.md) — scenarios tool flow
- [Superforecasting: Layered Model](../../explanation/superforecasting-layers.md) — three-layer architecture
- [Architecture Patterns](../../explanation/architecture-patterns.md) — MCP dispatch sequence