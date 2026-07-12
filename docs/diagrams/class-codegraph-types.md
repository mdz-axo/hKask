---
title: "CodeGraph Type System — Class Diagram"
audience: [developers, agents]
last_updated: 2026-07-04
version: "0.31.0"
status: "Active"
domain: "Infrastructure"
mds_categories: [domain, composition]
diagram_type: "class"
verified_against:
  - "crates/hkask-codegraph/src/types.rs"
  - "crates/hkask-codegraph/src/graph/store.rs"
  - "crates/hkask-codegraph/src/indexer/pipeline.rs"
  - "crates/hkask-codegraph/src/graph/context.rs"
  - "mcp-servers/hkask-mcp-codegraph/src/lib.rs"
diataxis: reference
---

# CodeGraph Type System

The `hkask-codegraph` crate provides a native Rust code understanding engine. It uses tree-sitter to parse Rust source into a semantic graph stored in SQLite, with FTS5 keyword search, recursive CTE graph traversal, impact analysis, dead code detection, and token-budgeted context assembly for LLM prompts.

This diagram shows the core type hierarchy and the relationships between the indexing pipeline, graph store, search engine, and MCP server layer.

```mermaid
classDiagram
    namespace CoreTypes {
        class Symbol {
            +Option~i64~ id
            +String name
            +SymbolKind kind
            +String file
            +usize start_line
            +usize end_line
            +String signature
            +Visibility visibility
            +Option~String~ doc_comment
            +Complexity complexity
        }
        class Edge {
            +Option~i64~ id
            +i64 from_id
            +i64 to_id
            +EdgeKind kind
            +String file
            +usize line
            +String target_name
        }
        class SearchResult {
            +Symbol symbol
            +f64 rank
        }
        class TraversalNode {
            +Symbol symbol
            +usize depth
            +String edge_kind
        }
        class AssembledContext {
            +Uuid context_id
            +String text
            +Vec~String~ symbols
            +usize estimated_tokens
        }
        class DeadCodeFinding {
            +String symbol_name
            +String kind
            +String file
            +usize line
        }
        class FileIndexResult {
            +String path
            +usize symbols
            +usize edges
            +u64 duration_ms
            +bool skipped
        }
        class IndexStats {
            +usize files
            +usize symbols
            +usize edges
        }
    }

    namespace Enums {
        class SymbolKind {
            <<enumeration>>
            Function
            Method
            Struct
            Enum
            EnumVariant
            Trait
            Impl
            Module
            Const
            Static
            TypeAlias
            Macro
            Test
        }
        class EdgeKind {
            <<enumeration>>
            Calls
            Imports
            Implements
            Contains
            References
            Inherits
        }
        class Visibility {
            <<enumeration>>
            Public
            Crate
            Private
        }
        class Complexity {
            <<enumeration>>
            NotComputed
            Computed
            Unparseable
        }
        class Direction {
            <<enumeration>>
            Forward
            Reverse
        }
        class ContextBudget {
            <<enumeration>>
            Minimal
            Focused
            Standard
            Full
        }
        class CodeGraphError {
            <<enumeration>>
            Parse
            Index
            Database
            Traversal
            Serialization
            Io
            Internal
        }
    }

    namespace Engine {
        class GraphStore {
            -Connection conn
            +open_in_memory() Result
            +open(path) Result
            +conn() Connection
            +upsert_file() i64
            +insert_symbols() Vec~i64~
            +insert_edges()
            +initialize_fts()
            +compute_pagerank()
        }
        class IndexPipeline {
            -GraphStore store
            -Instant last_full_index_at
            +new(store) Self
            +staleness_seconds() u64
            +index_directory(path) Vec~FileIndexResult~
            +index_file(path) FileIndexResult
            +stats() IndexStats
            +store() GraphStore
        }
        class search {
            +search(conn, query, limit) Vec~SearchResult~
        }
        class traversal {
            +traverse(conn, symbol_id, direction, max_depth) Vec~TraversalNode~
            +find_symbol_id(conn, name) Option~i64~
            +impact_analysis(conn, symbol_id, max_depth) ImpactResult
        }
        class analysis {
            +find_dead_code(conn) Vec~DeadCodeFinding~
            +find_high_complexity(conn, cyclo_threshold, cog_threshold) Vec~Finding~
        }
        class context {
            +assemble_context(conn, query, budget) AssembledContext
        }
    }

    namespace MCPServer {
        class CodeGraphServer {
            +WebID webid
            +String replicant
            +Option~DaemonClient~ daemon
            +CapabilityTier capability_tier
            -Arc~Mutex~IndexPipeline~~ pipeline
            -Option~EmbeddingRouter~ embed_router
            -Environment jinja
            +new(replicant, daemon, db_path) Result
            +codegraph_query()
            +codegraph_traverse()
            +codegraph_impact()
            +codegraph_analysis()
            +codegraph_context()
            +codegraph_structure()
            +codegraph_stats()
            +codegraph_reindex()
            +codegraph_feedback()
            +codegraph_index_embeddings()
        }
    }

    Symbol "1" --> "1" SymbolKind : kind
    Symbol "1" --> "1" Visibility : visibility
    Symbol "1" --> "1" Complexity : complexity
    Edge "1" --> "1" EdgeKind : kind
    Symbol "1" o-- "0..*" Edge : source
    Symbol "1" o-- "0..*" Edge : target

    GraphStore "1" --> "*" Symbol : stores
    GraphStore "1" --> "*" Edge : stores
    IndexPipeline "1" --> "1" GraphStore : owns
    IndexPipeline "1" --> "*" FileIndexResult : produces
    IndexPipeline "1" --> "1" IndexStats : produces

    search ..> GraphStore : reads
    search ..> SearchResult : produces
    traversal ..> GraphStore : reads
    traversal ..> TraversalNode : produces
    analysis ..> GraphStore : reads
    analysis ..> DeadCodeFinding : produces
    context ..> GraphStore : reads
    context ..> AssembledContext : produces
    context ..> ContextBudget : uses

    CodeGraphServer "1" --> "1" IndexPipeline : owns
    CodeGraphServer ..> search : delegates
    CodeGraphServer ..> traversal : delegates
    CodeGraphServer ..> analysis : delegates
    CodeGraphServer ..> context : delegates

    search ..> CodeGraphError : may raise
    traversal ..> CodeGraphError : may raise
    analysis ..> CodeGraphError : may raise
    context ..> CodeGraphError : may raise
    IndexPipeline ..> CodeGraphError : may raise
    GraphStore ..> CodeGraphError : may raise
```

### Diagram Notes

- **Symbol** and **Edge** are the core data types. Every Rust function, struct, trait, impl, module, etc. becomes a Symbol. Relationships (calls, imports, containment, trait implementations, inheritance) become Edges.
- **GraphStore** wraps a SQLite connection with `code_files`, `symbols`, `edges` tables plus `symbols_fts` (FTS5) and `symbols_vec` (sqlite-vec 0.1 for embeddings).
- **IndexPipeline** coordinates the full indexing flow: walk files → SHA-256 hash → parse with tree-sitter → extract symbols/edges → batch insert → resolve edge targets.
- **CodeGraphServer** is the thin MCP wrapper exposing 10 tools: `codegraph_query`, `codegraph_traverse`, `codegraph_impact`, `codegraph_analysis`, `codegraph_context`, `codegraph_structure`, `codegraph_stats`, `codegraph_reindex`, `codegraph_feedback`, `codegraph_index_embeddings`.
- **ContextBudget** controls token limits for LLM prompt assembly: Minimal (512), Focused (2048), Standard (4096), Full (8192).
- **CodeGraphError** uses `thiserror` with variants for Parse, Index, Database, Traversal, Serialization, Io, and Internal.

### Related Documentation

- [`class-service-layer.md`](class-service-layer.md) — Service layer class diagram (hexagonal ports)
- [`sequence-mcp-tool-dispatch.md`](sequence-mcp-tool-dispatch.md) — MCP tool dispatch sequence
- [`../architecture/hKask-architecture-master.md`](../architecture/hKask-architecture-master.md) — Architecture master (four patterns, crate-to-loop mapping)
- [`hkask-codegraph`](../../crates/hkask-codegraph/) — Implementation crate (original design plan absorbed)
