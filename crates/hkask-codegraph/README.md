# hkask-codegraph

Native code understanding engine for hKask — tree-sitter based semantic code graph with SQLite storage, recursive CTE traversal, and token-budgeted context assembly for LLM prompts.

## Public Modules

| Module | Purpose |
|--------|---------|
| `types` | Core types: `Symbol`, `Edge`, `SymbolKind`, `EdgeKind`, `Visibility`, `Complexity`, `Direction` |
| `graph` | Graph operations: `search`, `traversal`, `context` assembly, `analysis`, `ranking` |
| `indexer` | Incremental indexing pipeline with SHA-256 change detection and tree-sitter parsing |
| `error` | `CodeGraphError` and `IndexError` enums via `thiserror` |

## Key Types

| Type | Description |
|------|-------------|
| `Symbol` | A source code symbol — function, struct, trait, etc. with name, kind, file, line range, signature, visibility, doc comments, complexity |
| `Edge` | A relationship between two symbols — calls, imports, implements, contains, references, inherits |
| `SymbolKind` | Enum: `Function`, `Method`, `Struct`, `Enum`, `EnumVariant`, `Trait`, `Impl`, `Module`, `Const`, `Static`, `TypeAlias`, `Macro`, `Test` |
| `EdgeKind` | Enum: `Calls`, `Imports`, `Implements`, `Contains`, `References`, `Inherits` |
| `Visibility` | Enum: `Public`, `Crate`, `Private` (type-safe, not `String`) |
| `Complexity` | Type-state enum: `NotComputed`, `Computed { cyclomatic, cognitive }`, `Unparseable` |
| `Direction` | Enum: `Forward` (dependencies) or `Reverse` (callers/dependents) |
| `ContextBudget` | Enum: `Minimal` (512 tokens), `Focused` (2048), `Standard` (4096), `Full` (8192) |
| `AssembledContext` | Context bundle with UUID, text, symbol list, and estimated token count |
| `SearchResult` | FTS5 search result with `Symbol` and BM25 rank |
| `TraversalNode` | Graph traversal node with `Symbol`, depth, and edge kind |
| `ImpactResult` | Impact analysis result with risk classification (`Low`/`Medium`/`High`/`Critical`) |
| `AnalysisReport` | Combined dead code + complexity findings |
| `IndexPipeline` | Incremental indexing pipeline — hash-checked, tree-sitter parsed |
| `IndexStats` | Index statistics: files, symbols, edges |
| `FileIndexResult` | Per-file indexing result with skip detection |
| `CodeGraphError` | Error enum: `Parse`, `Index`, `Database`, `Traversal`, `Serialization`, `Io`, `Internal` |
| `IndexError` | Index-specific errors: `FileNotAccessible`, `NotUtf8`, `BatchInsert` |

## Key Functions

| Function | Description |
|----------|-------------|
| `search(conn, query, limit) -> Vec<SearchResult>` | FTS5 keyword search with BM25 ranking; falls back to LIKE |
| `search_prefix(conn, prefix, limit) -> Vec<Symbol>` | Prefix search for autocomplete |
| `traverse(conn, symbol_id, direction, max_depth) -> Vec<TraversalNode>` | Recursive CTE graph traversal |
| `impact_analysis(conn, symbol_id, max_depth) -> Vec<ImpactResult>` | Blast radius analysis with risk classification |
| `find_symbol_id(conn, name) -> Option<i64>` | Look up symbol ID by name |
| `assemble_context(conn, query, budget) -> AssembledContext` | Token-budgeted context for LLM prompts |
| `find_dead_code(conn) -> Vec<DeadCodeFinding>` | Dead code detection |
| `find_high_complexity(conn, min_cyclomatic, min_cognitive) -> Vec<ComplexityFinding>` | Complexity analysis |
| `analyze(conn, min_cyclomatic, min_cognitive) -> AnalysisReport` | Combined analysis |
| `estimate_tokens(text) -> usize` | Token estimation (~4 chars/token) |

## Usage

```rust
use hkask_codegraph::graph::store::GraphStore;
use hkask_codegraph::graph::{search, traversal, context};
use hkask_codegraph::types::Direction;
use hkask_codegraph::{ContextBudget, assemble_context};

// Open an in-memory store
let store = GraphStore::open_in_memory()?;

// Index with the pipeline
use hkask_codegraph::indexer::pipeline::IndexPipeline;
let pipeline = IndexPipeline::new(store);
pipeline.index_directory(&std::path::Path::new("src"))?;

// Search
let results = search::search(pipeline.store().conn(), "GraphStore", 10)?;

// Traverse dependencies
let id = traversal::find_symbol_id(pipeline.store().conn(), "GraphStore")?.unwrap();
let deps = traversal::traverse(pipeline.store().conn(), id, Direction::Forward, 5)?;

// Assemble LLM context
let ctx = assemble_context(pipeline.store().conn(), "authentication", ContextBudget::Focused)?;
println!("{}", ctx.text);
```

## Dependencies

- `rusqlite` (bundled) — SQLite with FTS5 and recursive CTEs
- `sqlite-vec` — Vector search extension
- `tree-sitter` + `tree-sitter-rust` — Rust source parsing
- `serde` / `serde_json` — Serialization
- `blake3` — Content hashing for incremental indexing
- `walkdir` — Directory traversal
- `uuid` — Context ID generation
- `tracing` — Regulation event emission
- `thiserror` — Error types
