# hkask-mcp-condenser

Context condensation MCP server — compress tool outputs, manage profiles, classify categories, summarize thread history.

Part of hKask's Episodic loop (L2). The condenser operates on the active conversation window, compressing tool outputs to fit within token budgets.

## Tools (7)

| Tool | Description | Requires Config |
|------|-------------|-----------------|
| `condenser_ping` | Liveness check, profile info, algorithm listing | — |
| `condenser_compress` | Compress tool output using context-aware algorithms | — |
| `condenser_classify` | Classify tool name to context category | — |
| `condenser_set_profile` | Set compression profile (heavy/normal/soft/light) | — |
| `condenser_stats` | Cumulative compression statistics | — |
| `condenser_persist` | Persist compressed output to episodic memory | `HKASK_DB_PATH` + `HKASK_DB_PASSPHRASE` |
| `condenser_thread_summary` | LLM-powered conversation summarization via centralized inference router. Disables model thinking/reasoning mode to ensure output tokens are used for the summary, not internal reasoning. Returns `original_tokens_approx` and `summary_tokens_approx` for context-window budgeting. | — |

## Compression Profiles

| Profile | Retention | Max Lines | Use Case |
|---------|-----------|-----------|----------|
| heavy | 10% | 30 | Aggressive compression |
| normal | 20% | 80 | Default balance |
| soft | 60% | 200 | Light compression |
| light | 95% | ∞ | Near-passthrough |

## Algorithms

| Algorithm | Categories | Strategy |
|-----------|-----------|----------|
| `rtk_style` | ShellCommand, TestOutput, BuildOutput | Head/tail preservation with ellipsis |
| `saliency_rank` | ConversationHistory, LogOutput, Unknown | TF-IDF scoring + structural bonus for errors/warnings |
| `flashrank` | FileContents, StructuredData | Greedy marginal-utility selection (relevance + novelty + brevity) |

## Configuration

Environment variables (all optional):

| Variable | Description | Default |
|----------|-------------|---------|
| `HKASK_DB_PATH` | SQLite database path for episodic persistence | In-memory (no persistence) |
| `HKASK_DB_PASSPHRASE` | Database encryption passphrase | Required if `HKASK_DB_PATH` is set |
| `INFERENCE_MODEL` | Model for thread summarization | `google/gemma-4-26B-A4B-it` (hKask classifier model; supports OM/, FW/, DI/ prefixes) |

Without `HKASK_DB_PATH`, `condenser_persist` returns a permission-denied error. All other tools work without configuration (graceful degradation). Thread summarization uses the centralized hKask inference router (configured via standard `OM_BASE_URL`, `FW_API_KEY`, `DI_API_KEY` environment variables).

## Context Categories

| Category | Matched Substrings | Algorithm |
|----------|-------------------|-----------|
| `shell_command` | git, docker, cargo, npm, shell, exec, run, bash | rtk_style |
| `test_output` | test, pytest, spec | rtk_style |
| `build_output` | build, compile, make | rtk_style |
| `file_contents` | file, read, cat | flashrank |
| `conversation_history` | chat, conversation, message | saliency_rank |
| `structured_data` | json, api, query | flashrank |
| `log_output` | log, journal, trace | saliency_rank |
| `unknown` | (fallback) | saliency_rank |

More-specific categories are checked first — `test` matches before `run`, so `pytest_run` classifies as `test_output`.

## Token Estimation

`approx_token_count` uses the standard ~4 characters per token heuristic (same rule of thumb used by OpenAI's tiktoken and Anthropic's Claude). This provides fast, dependency-free estimates for context-window planning.

`ThreadSummaryOutput` includes both `original_tokens_approx` (before summarization) and `summary_tokens_approx` (after), enabling callers to budget context windows. The `ChatService` auto-condense trigger uses this same heuristic at 87.5% of the model's context window.

## Thinking Mode

For models with reasoning/thinking mode (e.g., qwen3, gemma4, deepseek-r1), `condenser_thread_summary` and `ChatService::condense_history` set `disable_thinking: true` in `LLMParameters`. This maps to `enable_thinking: false` in the OpenAI-compatible chat request, instructing the model to skip internal reasoning and produce output directly. Without this, reasoning-mode models can consume all `max_tokens` on internal thought, producing empty visible output.

The `enable_thinking` field is only serialized when `false` — backends that don't support it are unaffected.

**Known behavior:** Some backends may not honor `enable_thinking: false` for reasoning-mode models (qwen3.5, gemma4, deepseek-r1). The condenser gracefully degrades: when a thinking model returns an empty summary, the tool responds with `"Inference engine returned an empty summary"`. **Workaround:** use a non-thinking model for summarization. The default condenser model is `google/gemma-4-26B-A4B-it` (hKask's classifier model), which produces clean structured summaries without thinking interference.

## Running

```bash
# As part of kask (auto-started with other MCP servers)
kask chat

# Standalone stdio MCP server
hkask-mcp-condenser

# With persistence
HKASK_DB_PATH=/path/to/db HKASK_DB_PASSPHRASE=secret hkask-mcp-condenser
```