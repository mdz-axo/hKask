# hkask-condenser

Context condensation domain logic for hKask.

Compresses conversation history and tool outputs to fit within token budgets.

## Features

- **Compression algorithms** — `rtk_style` (head/tail), `saliency_rank` (TF-IDF), `flashrank` (marginal utility)
- **Profiles** — heavy, normal, soft, light
- **Classification** — maps tool names to context categories
- **Thread summarization** — LLM-powered via centralized inference router
- **Health signals** — SLA violation detection

## Integration

The condenser MCP server (`hkask-mcp-condenser`) is a thin wrapper around this crate.

**LOC:** ~1,450 | **Tests:** 35+
