---
title: "CodeGraph Indexing Pipeline — Flowchart"
audience: [developers, agents]
last_updated: 2026-07-10
version: "0.31.0"
status: "Active"
domain: "Infrastructure"
mds_categories: [domain, composition]
diagram_type: "flowchart"
verified_against:
  - "crates/hkask-codegraph/src/indexer/pipeline.rs"
  - "crates/hkask-codegraph/src/indexer/extractor.rs"
  - "crates/hkask-codegraph/src/indexer/parser.rs"
  - "crates/hkask-codegraph/src/graph/store.rs"
  - "crates/hkask-codegraph/src/graph/schema.rs"
diataxis: reference
---

# CodeGraph Indexing Pipeline

The `IndexPipeline` coordinates the end-to-end flow from source files on disk to a searchable code graph in SQLite. It uses incremental BLAKE3 content hashes to skip unchanged files, parses Rust source with tree-sitter, extracts symbols and edges, resolves references by name, and computes PageRank when finalized.

Key design invariants:
- **Incremental skip**: Per-file BLAKE3 hash-on-read before tree-sitter work
- **Sequential directory indexing**: Files are indexed one at a time; each successful file writes symbols and resolvable edges to SQLite
- **Staleness tracking**: `last_full_index_at` is reset by `finalize()` and exposed by `staleness_seconds()`

```mermaid
flowchart TD
    Start([Start: index_directory or reindex]) --> WalkDir[Walk workspace, find .rs files]
    WalkDir --> NextFile{Next file?}
    NextFile -->|No| ComputePR[Compute PageRank across all nodes]
    NextFile -->|Yes| ReadFile[Read file bytes]
    ReadFile --> Hash[BLAKE3 content hash]
    Hash --> HashCheck{Hash matches stored?}
    HashCheck -->|Yes: skip| NextFile
    HashCheck -->|No: changed| Parse[tree-sitter parse Rust CST]
    Parse --> ParseErr{Parse OK?}
    ParseErr -->|Error| LogErr[Log parse error, skip file]
    LogErr --> NextFile
    ParseErr -->|OK| Extract[Extract symbols and edges from CST]
    Extract --> UpsertFile[Upsert file record with hash]
    UpsertFile --> InsertSyms[Batch insert symbols]
    InsertSyms --> ResolveEdges[Resolve edge targets by qualified name]
    ResolveEdges --> InsertEdges[Batch insert edges]
    InsertEdges --> NextFile
    ComputePR --> Health[Emit index health event]
    Health --> End([End: return index results])

    subgraph PerFile["Per-file sequential processing"]
        ReadFile
        Hash
        HashCheck
        Parse
        ParseErr
        Extract
    end

    subgraph SqliteWrites["SQLite writes"]
        UpsertFile
        InsertSyms
        ResolveEdges
        InsertEdges
    end


```

### Pipeline Stages

| Stage | Component | What Happens |
|-------|-----------|-------------|
| **Walk** | `walkdir` | Discover all `.rs` files in workspace |
| **Hash** | `blake3` | BLAKE3 content hash; compare to `code_files.content_hash` |
| **Parse** | `tree-sitter-rust` | Build Concrete Syntax Tree from source bytes |
| **Extract** | `extractor.rs` | Walk CST, produce `Vec<Symbol>` + `Vec<Edge>` with qualified names |
| **Insert** | `store.rs` | Batch insert symbols (ID assigned), resolve edge targets by name lookup |
| **Rank** | `pagerank` | Compute PageRank across all nodes for importance weighting |
| **Health** | tracing | Emit `cns.codegraph.index_health` after `finalize()` |

### CNS Spans

The pipeline emits tracing events for cybernetic observability:
- `cns.codegraph.file_indexed` — symbols, edges, and elapsed time for a changed file
- `cns.codegraph.index_health` — total files, symbols, edges, and zero staleness after `finalize()`

`staleness_seconds()` exposes elapsed time since `finalize()` for a caller that needs to monitor index freshness.

### Related Documentation

- [`class-codegraph-types.md`](class-codegraph-types.md) — Type system class diagram
- [`sequence-mcp-tool-dispatch.md`](sequence-mcp-tool-dispatch.md) — MCP tool dispatch sequence (applies to codegraph tools)
- [`../architecture/hKask-architecture-master.md`](../architecture/hKask-architecture-master.md) — Architecture master (crate-to-loop mapping)
- [`hkask-codegraph`](../../crates/hkask-codegraph/) — Implementation crate (original design plan absorbed)
