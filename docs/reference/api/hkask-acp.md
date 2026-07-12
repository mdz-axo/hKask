---
title: "hkask-acp — API Reference"
audience: [developers]
last_updated: 2026-07-07
version: "0.31.0"
status: "Active"
domain: "Core"
mds_categories: [domain]
last-verified-against: "3d1a876f"
---

# hkask-acp — API Reference

**Purpose:** Agent Communication Protocol (ACP) replicant. Presents coding agents in IDEs via the ACP protocol, reusing hKask's existing inference, memory, and MCP infrastructure.

## Public Modules

| Module | Purpose |
|--------|---------|
| `main_impl` | Core ACP agent implementation — session management, tool dispatch, prompt lifecycle |
| `cloud` | Cloud deployment support for ACP agents |

## Key Types

| Type | Description |
|------|-------------|
| `HkaskAcpAgent` | The ACP agent — implements the ACP protocol for IDE integration |
| `AcpError` | Error type for ACP operations |
| `SessionState` | Per-session state: conversation history, active tools, model configuration |

## Architecture

The ACP replicant reuses existing hKask infrastructure:
- **Inference** — delegates to `hkask-inference::InferenceRouter` for model calls
- **Memory** — uses `hkask-memory` for episodic recall and semantic search
- **MCP tools** — dispatches through the OCAP membrane (`GovernedTool`)
- **CNS** — emits spans for tool invocations and inference calls

## Prompt Turn Lifecycle

1. IDE sends a user prompt
2. `HkaskAcpAgent` constructs context (conversation history + relevant memory)
3. Inference is dispatched via the inference router
4. Tool calls in the response are executed through GovernedTool
5. Results are returned to the IDE
6. CNS spans are emitted for observability

## ACP Protocol vs MCP vs A2A

| Protocol | Purpose | hKask Implementation |
|----------|---------|---------------------|
| **ACP** | IDE agent presence — coding agents in editors | `hkask-acp` (this crate) |
| **MCP** | Tool protocol — model-tool communication | `hkask-mcp` (15 servers) |
| **A2A** | Agent-to-agent communication | `hkask-agents::a2a` |
