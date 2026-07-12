---
title: "MCP Server Reference"
audience: [developers, operators]
last_updated: 2026-07-07
version: "0.31.0"
status: "Active"
domain: "Core"
mds_categories: [domain, composition]
last-verified-against: "3d1a876f"
---

# MCP Server Reference

All 15 MCP servers registered in `hkask-mcp::BUILTIN_SERVERS`. Each server follows the standard bootstrap pattern: `bootstrap_mcp_server` → P4 startup gates → `run_stdio_server`. Tools are dispatched through the OCAP membrane (`GovernedTool`).

## Server Listing

| Server | Purpose | Tools | Capability Tier |
|--------|---------|-------|----------------|
| `hkask-mcp-memory` | Unified episodic + semantic memory with cloud backup | 16 | Standard |
| `hkask-mcp-condenser` | Context condensation (thin wrapper over `hkask-condenser`) | 7 | Standard |
| `hkask-mcp-research` | Web search, extraction, and feed-based research | 17 | Standard |
| [`hkask-mcp-companies`](hkask-mcp-companies.md) | Company financial data, valuation, research, and portfolio tools | 41 | Standard |
| `hkask-mcp-communication` | Thin MCP wrapper over core communication crate | 9 | Communication |
| `hkask-mcp-curator` | System observability, escalation management, regulatory memory | 16 | Curator |
| `hkask-mcp-filesystem` | Filesystem and shell access (OCAP-governed, path allowlisting) | 12 | Restricted |
| `hkask-mcp-media` | Media generation (image, video, audio, 3D, workflows via fal.ai) | 37 | Standard |
| `hkask-mcp-docproc` | Unified document processing (format conversion, OCR, chunking, QA) | 9 | Standard |
| `hkask-mcp-training` | Model training (QA pairs, fine-tuning pipeline ingestion) | 8 | Standard |
| `hkask-mcp-replica` | Authorial style and corpus-pipeline operations | 14 | Standard |
| `hkask-mcp-scenarios` | Scenario planning and calibrated event-tree forecasting | 18 | Standard |
| `hkask-mcp-kata-kanban` | Kata-Kanban workflow coordination | 8 | Standard |
| `hkask-mcp-skill` | Skill invocation (exposes registered skills as callable tools) | 15 | Standard |
| `hkask-mcp-codegraph` | Code understanding (query, traverse, impact, context, embed) | 10 | Standard |

## Bootstrap Pattern

Every MCP server follows this pattern. `bootstrap_mcp_server` requires a non-empty host identity (`HKASK_MCP_HOST`, or `HKASK_CURATOR_REPLICANT` for Curator) and fails before daemon verification when it is absent.

```
bootstrap_mcp_server(name, target, host_env_var)
  → Require host identity
  → Register tools via mcp_server! macro
  → Build ToolContext with impl_tool_context!
  → verify_startup_gates() — P4 startup checks
  → run_stdio_server()
```

## Tool Dispatch Flow

1. Tool invocation arrives at server
2. `GovernedTool` membrane checks OCAP capability
3. Energy reserved from gas budget
4. CNS ν-event emitted (`cns.tool.reserved`)
5. Tool implementation executed
6. Energy settled (`cns.tool.completed`)
7. If OCAP denied → `cns.tool.denied` ν-event, error returned

## Capability Tiers

| Tier | Description | Example Servers |
|------|-------------|-----------------|
| `Standard` | General-purpose tools, available to all pods | Most servers |
| `Restricted` | Requires explicit OCAP delegation | `hkask-mcp-filesystem` |
| `Communication` | Matrix/chat integration | `hkask-mcp-communication` |
| `Curator` | System administration and escalation | `hkask-mcp-curator` |

## Registration

All servers are registered in `crates/hkask-mcp/src/lib.rs` in the `BUILTIN_SERVERS` constant (15 entries). See `docs/how-to/bootstrap-mcp-server.md` for adding new servers.
