---
name: condenser-continuation
visibility: public
description: "Continuation skill for resuming hKask condenser implementation work after context reset. Restores session state, prioritizes remaining tasks, verifies build status, and composes a structured continuation plan. Works with any hKask-supported inference engine (DeepInfra, Together AI, fal.ai, OpenRouter, etc.)."
---

# Condenser Continuation Skill

Resume condenser implementation work after a context reset. This skill distills prior session state into an actionable continuation plan — what was done, what remains, what to do next, and whether the codebase is still healthy.

## Behavioral Dependencies

| Dependency | Type | Why |
|------------|------|-----|
| `hkask-mcp-condenser` (MCP server) | Prohibition | Step 3 build verification (`cargo check/build/clippy`) fails without it |
| `hkask-condenser` (crate) | Prohibition | Domain logic crate — build depends on `engine.rs`, `inference.rs`, `types.rs`, `algorithms.rs` |

## Informational Context

The skill describes the condenser's architecture (InferencePort trait, InferenceRouter, DeepInfra/Together/fal.ai/OpenRouter backends, CNS spans) for domain understanding. These are not behavioral dependencies — the skill functions as a continuation orchestrator regardless of which inference engine is configured.

## Registry Templates

This skill's runtime templates live in `registry/templates/condenser-continuation/`:

| Template | Type | Purpose |
|----------|------|--------|
| `condenser-continuation-restore.j2` | KnowAct | Distill session context into essential facts for continuation |
| `condenser-continuation-prioritize.j2` | KnowAct | Rank remaining tasks by priority and identify immediate next action |
| `condenser-continuation-verify.j2` | KnowAct | Produce a structured verification plan (commands, expected outcomes, success criteria) for the agent or runtime to execute |
| `condenser-continuation-compose.j2` | WordAct | Assemble the final structured continuation document |
| `condenser-convergence-check.j2` | KnowAct | Compute normalized convergence metric for the continuation cycle |

The SKILL.md (this file) teaches the Zed coding agent the condenser domain and methodology. The .j2 templates are executable process steps the hKask runtime invokes during `kask chat` sessions.

## When to Use

- Resuming condenser implementation work after a context window reset
- The user says "continue condenser work" or "pick up condenser"
- Starting a new session that needs to continue prior condenser integration work

## Domain: Condenser Implementation

The hKask condenser is a single MCP server (`hkask-mcp-condenser`) with 7 tools: `compress`, `classify`, `set_profile`, `stats`, `ping`, `persist`, `thread_summary`. No running hKask instance required — compiles standalone. Binary: `hkask-mcp-condenser`.

### Local CPU-Only Tools (6 tools)

`compress`, `classify`, `set_profile`, `stats`, `ping`, and `persist` run entirely on local CPU with no LLM dependency. Three compression algorithms (rtk_style, saliency_rank, flashrank) dispatch by context category.

### Thread Summary via Centralized Inference Router

The `condenser_thread_summary` tool uses the centralized hKask inference router (`InferencePort` trait, implemented by `InferenceRouter`). The router dispatches to DeepInfra, Together AI, fal.ai, or OpenRouter based on the model name's provider prefix (DI/, TG/, FA/, OR/). No standalone HTTP client or per-tool inference URL configuration — the router is built once at startup from standard hKask environment variables (`DI_API_KEY`, `TOGETHER_API_KEY`, `FA_API_KEY`, `OPENROUTER_API_KEY`).

**Graceful degradation:** If no inference backends are reachable, `thread_summary` returns an error. All other tools continue working.

**Key implementation detail:** For models with thinking/reasoning mode (e.g., qwen3, gemma4, deepseek-r1), `condenser_thread_summary` sets `disable_thinking: true` in `LLMParameters` before calling the centralized inference router. The router passes the flag through to the backend (e.g., as `enable_thinking: false` in OpenAI-compatible chat requests), preventing reasoning-mode models from spending all `max_tokens` on internal reasoning. If a backend ignores the flag, the tool degrades gracefully with an empty-summary error.

### MCP Server Configuration

The condenser MCP server is registered in the MCP runtime config (not in any editor-specific settings file). Condenser-specific credentials are:

- `INFERENCE_MODEL` — default `google/gemma-4-26B-A4B-it` (hKask classifier model), overridable per-request via the tool's `model` parameter.
- `HKASK_DB_PATH` + `HKASK_DB_PASSPHRASE` — required only by `condenser_persist`; without them the tool returns a permission-denied error while all other tools continue working.

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
2. **Inference-agnostic.** The condenser uses the centralized hKask inference router (`InferencePort`). No standalone HTTP client or inference URL configuration.
3. **Graceful degradation.** If the inference router has no reachable backends, the skill must still restore context and prioritize tasks — only `thread_summary` is unavailable.
4. **No sensitive data in outputs.** API keys, tokens, and PII must be redacted from continuation documents.
5. **Reference, don't duplicate.** Point to files by path, never reproduce their contents in continuation documents.
6. **Decisions carry rationale.** Every architectural decision in the continuation document must include *why* it was made.

## Key Files

| File | Purpose |
|------|--------|
| `mcp-servers/hkask-mcp-condenser/src/main.rs` | MCP server entry point, all tool implementations |
| `mcp-servers/hkask-mcp-condenser/Cargo.toml` | Dependencies including `hkask-inference` for the centralized inference router |
| `crates/hkask-condenser/src/engine.rs` | Pure domain logic — compression dispatch, profile management, stats |
| `crates/hkask-condenser/src/inference.rs` | Pure formatting functions — prompt building, text formatting, token estimation, output construction |
| `crates/hkask-condenser/src/types.rs` | Request/response types including `ThreadSummaryRequest`/`ThreadSummaryOutput` |
| `crates/hkask-condenser/src/algorithms.rs` | Compression and classification algorithms |

## Debug

- CNS spans: `cns.tool.condenser` for tool invocation governance
- `cns.inference` for inference governance when thread_summary is active
- Check `kask cns health` for current CNS state
- Run `hkask-mcp-condenser` standalone to test MCP handshake without hKask runtime

## Registry Manifest

**Type:** Skill | **Manifest:** `registry/manifests/condenser-continuation.yaml`

### PDCA Convergence
- **Threshold:** 0.05 (converged when metric ≤ this)
- **Improvement ratio:** 0.05 (min relative reduction per iteration)
- **Improvement gate:** threshold_only
- **Max iterations:** 3
- **Convergence meaning:** 0 = continuation packet is coherent, actionable, and free of critical blockers

### Energy Budgets
- **Gas (compute cycles):** cap 100000, 100 per iteration
- **rJoule (inference energy):** cap 3 rJ (manifest `rjoule.cap` — see `registry/manifests/condenser-continuation.yaml` for canonical value)
- **System constant:** 1 rJ = 250,000 gas cycles (`RJOULE_TO_GAS`)
