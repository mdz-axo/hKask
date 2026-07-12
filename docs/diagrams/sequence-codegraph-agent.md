---
title: "CodeGraph Agent Workflow — Sequence Diagram"
audience: [developers, agents]
last_updated: 2026-07-10
version: "0.31.0"
status: "Active"
domain: "Infrastructure"
mds_categories: [composition, domain]
diagram_type: "sequence"
verified_against:
  - "mcp-servers/hkask-mcp-codegraph/src/lib.rs:243-632"
  - "crates/hkask-codegraph/src/graph/search.rs"
  - "crates/hkask-codegraph/src/graph/traversal.rs"
  - "crates/hkask-codegraph/src/indexer/pipeline.rs"
diataxis: reference
---

# CodeGraph Agent Workflow

An agent uses the codegraph MCP server to understand the codebase, trace dependencies, assess change impact, and assemble context for LLM prompts. This sequence shows the three most common interaction patterns: search, impact analysis, and context assembly with feedback.

All codegraph tools follow the same OCAP-gated MCP dispatch pattern established by `sequence-mcp-tool-dispatch.md`. This diagram focuses on the codegraph-specific logic after the security gateway.

```mermaid
sequenceDiagram
    participant Agent as Agent (via MCP)
    participant Server as CodeGraphServer
    participant Pipeline as IndexPipeline (Mutex)
    participant Store as GraphStore (SQLite)
    participant FTS as symbols_fts (FTS5)
    participant CNS as CNS (tracing)

    Note over Agent,CNS: ── Pattern 1: Search & Traverse ──

    Agent->>+Server: codegraph_query(query="McpRuntime", limit=10)
    Server->>Server: ensure_indexed()
    Server->>+Pipeline: lock()
    Pipeline-->>-Server: &IndexPipeline
    Server->>+Store: conn()
    Store-->>-Server: &Connection
    Server->>+FTS: MATCH query, ORDER BY rank, LIMIT 10
    FTS-->>-Server: Vec~SearchResult~ (BM25 ranked)
    Server-->>-Agent: [{symbol, rank}, ...]

    Agent->>+Server: codegraph_traverse(symbol="McpRuntime", direction="reverse", max_depth=3)
    Server->>+Store: find_symbol_id("McpRuntime")
    Store-->>-Server: Some(id=142)
    Server->>+Store: RECURSIVE CTE: edges WHERE to_id=142, depth≤3
    Store-->>-Server: [{deep_callee(d=2), calls}, ...]
    Server-->>-Agent: [TraversalNode{callee: fn deep_callee(), depth: 2, edge: calls}]

    Note over Agent,CNS: ── Pattern 2: Impact Analysis ──

    Agent->>+Server: codegraph_impact(symbol="McpRuntime", max_depth=5)
    Server->>+Store: traverse(symbol_id, Reverse, 5)
    Store-->>-Server: Vec~TraversalNode~ (all dependents)
    Server->>Server: classify_risk per symbol
    Note right of Server: Critical: public traits<br/>High: public types<br/>Medium: impls, crate-visible<br/>Low: private/test
    Server-->>-Agent: {symbol, total_affected: 12, affected: [{risk: critical}, ...]}

    Note over Agent,CNS: ── Pattern 3: Context Assembly + Feedback ──

    Agent->>+Server: codegraph_context(query="authentication", budget="focused")
    Server->>+FTS: search("authentication", max=20, BM25)
    FTS-->>-Server: SearchResults
    Server->>+Store: rank by PageRank, apply budget (2048 tokens, 20 symbols)
    Store-->>-Server: AssembledContext{text, symbols, estimated_tokens}
    Server-->>-Agent: {context_id: uuid, text: "...", symbols: [...], estimated_tokens: 1800}

    Note over Agent: Agent uses context in LLM prompt,<br/>observes which symbols were actually referenced

    Agent->>+Server: codegraph_feedback(context_id, symbols_provided, symbols_used)
    Server->>+CNS: tracing::info!("cns.codegraph.context_efficiency", ratio=0.65)
    CNS-->>-Server: span emitted
    Server-->>-Agent: {recorded: true, ratio: 0.65}

    Note over Agent,CNS: ── Pattern 4: Re-index ──

    Agent->>+Server: codegraph_reindex()
    Server->>+Pipeline: index_directory(workspace_root)
    loop For each .rs file
        Pipeline->>Pipeline: BLAKE3 hash, compare to stored
        alt hash changed
            Pipeline->>Pipeline: tree-sitter parse → extract symbols/edges
            Pipeline->>+Store: batch insert symbols, resolve edges
            Store-->>-Pipeline: name→id mapping
            Pipeline->>+CNS: cns.codegraph.file_indexed
            CNS-->>-Pipeline: span emitted
        else hash matches
            Pipeline->>Pipeline: skip (returned in results as skipped:true)
        end
    end
    Pipeline->>Store: compute PageRank

    Pipeline->>+CNS: cns.codegraph.index_health (staleness_seconds=0)
    CNS-->>-Pipeline: span emitted
    Pipeline-->>-Server: IndexStats {files, symbols, edges}
    Server-->>-Agent: {files_indexed, symbols_added, total_symbols, total_edges}
```

### Tool Summary

| Tool | What it does | Key query |
|------|-------------|-----------|
| `codegraph_query` | FTS5 keyword search with BM25 ranking | `SELECT ... FROM symbols_fts WHERE MATCH ? ORDER BY rank` |
| `codegraph_traverse` | Recursive CTE: forward (deps) or reverse (callers) | `WITH RECURSIVE trav AS (SELECT e.to_id ... UNION SELECT e.to_id ...)` |
| `codegraph_impact` | Reverse traversal + risk classification | Same CTE + `classify_risk(symbol)` per result |
| `codegraph_analysis` | Dead code detection or complexity hotspots | `symbols WHERE id NOT IN (SELECT to_id FROM edges)` |
| `codegraph_context` | Token-budgeted context assembly for LLM prompts | FTS5 search → PageRank sort → budget cap |
| `codegraph_feedback` | Signal-to-noise tracking (G12 feedback loop) | CNS span: `cns.codegraph.context_efficiency` |

### CNS Spans Emitted

| Span | Trigger | Target |
|------|---------|--------|
| `cns.codegraph.file_indexed` | Per-file index complete | G7: file-level observability |
| `cns.codegraph.index_health` | After full re-index | X6: staleness reset to 0 |
| `cns.codegraph.context_efficiency` | After `codegraph_feedback` | G12: ratio of used/provided symbols |
| `cns.codegraph.embeddings` | After `codegraph_index_embeddings` batch | G13: embedding batch complete |

### Related Documentation

- [`erd-codegraph-schema.md`](erd-codegraph-schema.md) — Database schema ERD
- [`class-codegraph-types.md`](class-codegraph-types.md) — Type system class diagram
- [`sequence-mcp-tool-dispatch.md`](sequence-mcp-tool-dispatch.md) — MCP tool dispatch with OCAP enforcement
- [`../architecture/hKask-architecture-master.md`](../architecture/hKask-architecture-master.md) — Architecture master
