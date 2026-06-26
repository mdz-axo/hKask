# hkask-condenser

Domain logic for context condensation — compression algorithms, ontology-aware
saliency weighting, and engine state management. Pure domain crate: no MCP,
no HTTP, no async.

## Architecture

| Module | Role |
|--------|------|
| `algorithms` | Three compression algorithms (`rtk_style`, `saliency_rank`, `flashrank`) with domain-aware scoring |
| `ontology_graph` | Cross-domain concept relationship index (FIBO, CogAT, GOLEM, ML-Schema, OMC, PKO, DC+BIBO) |
| `types` | `OntologyAnchor`, `Profile`, `ContextCategory`, `CompressedOutput`, health signals |
| `engine` | `CondenserEngine` — stateful compression dispatch, profile management, CNS observability |
| `inference` | Prompt formatting and token estimation for LLM thread summarization |

## Compression Profiles

| Profile | Retention | Max Lines | Action Threshold | Use Case |
|---------|-----------|-----------|-----------------|----------|
| `heavy` | 10% | 30 | 0.10 | Aggressive compression — minimal representation |
| `normal` | 20% | 80 | 0.25 | Default — balanced |
| `soft` | 60% | 200 | 0.50 | Moderate — preserves more context |
| `light` | 95% | — | 0.90 | Minimal compression — user sovereignty |

## Algorithms

### rtk_style
Head/tail ellipsis truncation. Keeps the first N% and last M% of lines with
a `...` separator. Default for ShellCommand, TestOutput, BuildOutput.
Uses ontology density factor to adjust head/tail ratio (FIBO gets more tail).

### saliency_rank
TF-IDF + structural bonus + ontology-aware scoring. Scores every line, keeps
the highest-scoring budget lines. Default for ConversationHistory, LogOutput.

Scoring formula:
```
score = TF-IDF_average + structural_bonus + domain_bonus + graph_adjacency_bonus
```

- **structural_bonus:** error=2.0, warning=1.0, heading=0.5, list=0.2
- **domain_bonus:** 0.3–0.5 for domain-specific keywords (e.g., FIBO numeric precision)
- **graph_adjacency_bonus:** 0.15 per related concept from the ontology graph, capped at 0.5

### flashrank
Greedy marginal-utility selection under token budget. Balances relevance,
novelty, and brevity. Default for FileContents, StructuredData, Unknown.

## Ontology Anchoring (P5.4/P8.1)

The condenser derives the ontology anchor from the `tool_name` — every MCP
server links against the same bridge crates, so no wire-protocol fields
are needed.

| Tool prefix | Ontology tier | Domain bridge |
|-------------|--------------|---------------|
| `company_*`, `stock_*`, `dcf_*`, `portfolio_*` | Domain supplement | FIBO |
| `memory_*`, `episodic_*`, `semantic_*` | Domain supplement | CogAT |
| `replica_*`, `author_*` | Domain supplement | GOLEM |
| `training_*`, `adapter_*`, `sweep_*` | Domain supplement | ML-Schema |
| `generate_*`, `video_*`, `image_*`, `gallery_*` | Domain supplement | OMC |
| `kanban_*`, `task_*`, `spec_*`, `research_*`, `skill_*` | Dual-axis (PKO) | — |
| `file_*`, `web_*`, `registry_*`, `wallet_*` | Dual-axis (DC+BIBO) | — |
| Everything else | Core (5W1H) | — |

The ontology graph encodes concept relationships (e.g., `fibo:Corporation` →
`HasProperty` → `fibo:MarketCapitalization`) and serves as a saliency
multiplier — lines containing graph-adjacent concepts get bonus scores.

## CNS Spans

| Span | Fields | When |
|------|--------|------|
| `cns.condenser` compress | `algorithm`, `category`, `tool_name`, `ontology_tier` | Every compression |
| `cns.condenser` compression_ratio | `reduction_pct`, `original_bytes`, `compressed_bytes`, `latency_ms` | Every compression |
| `cns.condenser` health | `total_compressions`, `health_signal_count` | Health check |
| `cns.condenser.degraded` | — | Systemic health violations (caller emits) |

## Consumers

- `hkask-mcp-condenser` — MCP server: thin wrapper exposing `/condenser/compress` etc.
- `hkask-services` — `ChatService::condense_history`: auto-condenses conversation windows
