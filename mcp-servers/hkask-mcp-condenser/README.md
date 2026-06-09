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
| `condenser_thread_summary` | LLM-powered conversation summarization via Okapi | `OKAPI_URL` |

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
| `OKAPI_URL` | Okapi inference engine URL (e.g. `http://127.0.0.1:11435`) | Thread summary unavailable |
| `OKAPI_MODEL` | Model for summarization | `qwen3:8b` |
| `OKAPI_API_KEY` | API key if Okapi authentication is enabled | — |

Without `HKASK_DB_PATH`, `condenser_persist` returns a permission-denied error. Without `OKAPI_URL`, `condenser_thread_summary` returns a permission-denied error. All other tools work without configuration (graceful degradation).

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

## Running

```bash
# As part of kask (auto-started with other MCP servers)
kask chat

# Standalone stdio MCP server
hkask-mcp-condenser

# With persistence
HKASK_DB_PATH=/path/to/db HKASK_DB_PASSPHRASE=secret hkask-mcp-condenser

# With Okapi thread summarization
OKAPI_URL=http://127.0.0.1:11435 hkask-mcp-condenser
```