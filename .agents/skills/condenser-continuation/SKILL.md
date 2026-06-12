---
name: condenser-continuation
visibility: public
description: "Continuation skill for resuming hKask condenser implementation work after context reset. Restores session state, prioritizes remaining tasks, verifies build status, and composes a structured continuation plan. Works with any hKask-supported inference engine (Ollama, Fireworks, DeepInfra, etc.)."
---

# Condenser Continuation Skill

Resume condenser implementation work after a context reset. This skill distills prior session state into an actionable continuation plan — what was done, what remains, what to do next, and whether the codebase is still healthy.

## Registry Templates

This skill's runtime templates live in `registry/templates/condenser-continuation/`:

| Template | Type | Purpose |
|----------|------|--------|
| `condenser-continuation-restore.j2` | KnowAct | Distill session context into essential facts for continuation |
| `condenser-continuation-prioritize.j2` | KnowAct | Rank remaining tasks by priority and identify immediate next action |
| `condenser-continuation-verify.j2` | FlowDef | Run build/check/test commands and collect verification results |
| `condenser-continuation-compose.j2` | WordAct | Assemble the final structured continuation document |

The SKILL.md (this file) teaches the Zed coding agent the condenser domain and methodology. The .j2 templates are executable process steps the hKask runtime invokes during `kask chat` sessions.

## When to Use

- Resuming condenser implementation work after a context window reset
- The user says "continue condenser work", "pick up condenser", or "resume Option A/B"
- Starting a new session that needs to continue prior condenser integration work

## Domain: Condenser Implementation

The hKask condenser has two implementation options, both exposed as an MCP server (`hkask-mcp-condenser`):

### Option A: Standalone Condenser (MCP Server)

A self-contained MCP server with 7 tools: `compress`, `classify`, `set_profile`, `stats`, `ping`, `persist`, `thread_summary`. No running hKask instance required — compiles standalone. Binary: `hkask-mcp-condenser`.

### Option B: Thread Summary via Inference Engine

Adds a `condenser_thread_summary` tool that calls an inference engine's chat endpoint for LLM-powered summarization. Works with any hKask-supported inference backend:

| Engine | Endpoint | Config |
|--------|----------|--------|
| Ollama | `/api/chat` | `INFERENCE_URL`, `INFERENCE_MODEL` |
| Other | HTTP chat API | `INFERENCE_URL`, `INFERENCE_MODEL` + engine-specific vars |

**Key implementation detail:** For models with thinking/reasoning mode (e.g., qwen3), the chat request must include `"think": false` (or equivalent) to prevent the model from spending all output tokens on internal reasoning. This is engine-specific configuration, not a global setting.

**Graceful degradation:** Without an inference endpoint configured, `thread_summary` returns a clear error. All other Option A tools continue working.

### MCP Server Configuration

The condenser MCP server is registered in the MCP runtime config (not in any editor-specific settings file). Required env vars depend on which inference backend is in use.

## Procedure

### Step 1: Restore Context

Read the session context (prior handoff document, conversation history, or explicit user input) and distill it into essential facts:
- What implementation options are complete
- What remains unfinished
- What key files were changed
- What decisions were made and why

### Step 2: Prioritize Remaining Work

Rank remaining tasks as HIGH / MEDIUM / LOW. For each:
- What specifically needs to happen
- Where in the codebase the work should happen
- Any dependencies or blockers
- Whether it depends on a specific inference engine

### Step 3: Verify Current State

Run verification commands to confirm the codebase is still healthy:
- `cargo check -p hkask-mcp-condenser`
- `cargo build -p hkask-mcp-condenser --release`
- `cargo clippy -p hkask-mcp-condenser -- -D warnings`
- Test MCP handshake with the condenser binary

### Step 4: Compose Continuation Plan

Assemble a structured continuation document with:
- Restored context summary
- Prioritized task list
- Verification results
- Immediate next action
- Recommended skills for the session

## Constraints

1. **Headless only.** No visual UI, no Grafana, no dashboards. The condenser is CLI/MCP/API only.
2. **Inference-agnostic.** Templates must not hardcode any specific inference engine. Use the `inference` MCP server abstraction.
3. **Graceful degradation.** If no inference endpoint is available, the skill must still restore context and prioritize tasks — only `thread_summary` is unavailable.
4. **No sensitive data in outputs.** API keys, tokens, and PII must be redacted from continuation documents.
5. **Reference, don't duplicate.** Point to files by path, never reproduce their contents in continuation documents.
6. **Decisions carry rationale.** Every architectural decision in the continuation document must include *why* it was made.

## Key Files

| File | Purpose |
|------|--------|
| `mcp-servers/hkask-mcp-condenser/src/main.rs` | MCP server entry point, all tool implementations |
| `mcp-servers/hkask-mcp-condenser/src/engine.rs` | Pure domain logic — compression dispatch, profile management, stats |
| `mcp-servers/hkask-mcp-condenser/src/inference.rs` | Inference-backed summarization — message formatting, response validation |
| `mcp-servers/hkask-mcp-condenser/src/types.rs` | Request/response types including `ThreadSummaryRequest`/`ThreadSummaryOutput` |
| `mcp-servers/hkask-mcp-condenser/src/algorithms.rs` | Compression and classification algorithms |
| `mcp-servers/hkask-mcp-condenser/Cargo.toml` | Dependencies including `reqwest` for inference HTTP calls |

## Debug

- CNS spans: `cns.tool.condenser.*` for tool invocation governance
- `cns.inference.*` for inference governance when Option B is active
- Check `kask /status` for current agent, model, and pod state
- Run `hkask-mcp-condenser` standalone to test MCP handshake without hKask runtime