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
| Filesystem | `mcp-servers/hkask-mcp-filesystem` | File access and shell operations (OCAP-sandboxed) | 7 | — |
| Kata Kanban | `mcp-servers/hkask-mcp-kata-kanban` | Toyota Kata task boards | 14 | `hkask-services-kata-kanban` |
| Media | `mcp-servers/hkask-mcp-media` | Fal.ai media generation | — | — |
| Replica | `mcp-servers/hkask-mcp-replica` | Replicant lifecycle | — | — |
| Research | `mcp-servers/hkask-mcp-research` | Web search, extraction, browsing, RSS feeds | 17 | `hkask-services-research` |
| [Skill](skill-server.md) | `mcp-servers/hkask-mcp-skill` | Skill registry access (list, execute) | 3 | — |
| Training | `mcp-servers/hkask-mcp-training` | LoRA training pipeline | — | — |
| Communication | `mcp-servers/hkask-mcp-communication` | Federation messaging | — | — |
| Condenser | `mcp-servers/hkask-mcp-condenser` | Context condensation | — | — |

## Common Patterns

All servers follow these patterns:
:

1. **Bootstrap:** `hkask_mcp::bootstrap_mcp_server(name, target, host_env_var)` → returns `MCPBootstrap { replicant, daemon_client }`
2. **Struct:** `hkask_mcp::mcp_server!` macro generates the struct with `webid`, `replicant`, `daemon` fields plus domain fields
3. **Tool dispatch:** `execute_tool_semantic(self, tool_name, ontology, async { ... })` wraps each tool with CNS span + daemon outcome recording
4. **Tool router:** `#[tool_handler(router = Self::...router())]` on the `ServerHandler` impl
5. **Error type:** `McpToolError` for tool-level errors, domain `Error` enums (via `thiserror`) for computation errors
6. **Governance:** OCAP is enforced at the dispatcher `GovernedTool` membrane (`DelegationToken` per call), not at the server. The server is the transport pipe; `shell_exec`-style tools are reachable only by agents holding the relevant capability token. See [dispatch.rs](../../crates/hkask-mcp/src/dispatch.rs).

## Testing standard

Every MCP server MUST include **tool-behavior contract tests** that invoke tools through their public `Parameters<T>` seam (e.g. `server.fs_read(Parameters(FsReadRequest { ... }))`), covering at minimum: the happy path, invalid input, boundary/edge cases, and error-specificity. Helper-seam-only tests (testing `sandbox_path`/services/infrastructure in isolation) are necessary but **not sufficient** — a helper-seam-only suite cannot catch tool-contract bugs (slice-index panics on bad input, canonicalize-on-non-existent, silent no-ops, error-swallowing), as the [filesystem review](filesystem.md) demonstrated with three shipped logic bugs that had zero `unwrap()` calls. The [`filesystem_contract.rs`](../../../mcp-servers/hkask-mcp-filesystem/tests/filesystem_contract.rs) and [`kanban_contract.rs`](../../../mcp-servers/hkask-mcp-kata-kanban/tests/kanban_contract.rs) suites are the exemplar patterns. See the [fleet test-seam audit](../../status/mcp-fleet-test-seam-audit-2026-07-17.md) for the current coverage gap across all 15 servers.

## Cross-links

- [Skill MCP Server](skill-server.md) — Skill server architecture reference (3 tools, diagram)
- [Research MCP Adversarial Review](../../status/research-mcp-adversarial-review-2026-07-17.md) — code smell inventory for the research server
- [Filesystem Server Reference](filesystem.md) — sandbox model, 7 tools, CNS spans, current behavior and known limitations (DIAG-RF-003)
- [Scenarios Adversarial Review](../../status/scenarios-adversarial-review.md) — code smell inventory for the scenarios server
- [Companies MCP Code Review](../../status/companies-mcp-code-review-2026-07-15.md) — adversarial code review of the companies server
- [Scenario Forecasting Pipeline Diagram](../../diagrams/flowchart-scenario-forecasting-pipeline.md) — scenarios tool flow
- [Superforecasting: Layered Model](../../explanation/superforecasting-layers.md) — three-layer architecture
- [Architecture Patterns](../../explanation/architecture-patterns.md) — MCP dispatch sequence