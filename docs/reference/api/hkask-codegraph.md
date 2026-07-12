---
title: "hkask-codegraph — API Reference"
audience: [developers]
last_updated: 2026-07-07
version: "0.31.0"
status: "Active"
domain: "Core"
mds_categories: [domain]
last-verified-against: "e17e69e2"
---

# hkask-codegraph — API Reference

Native Rust code understanding engine. Provides semantic code graph construction from Rust source via tree-sitter, SQLite-backed symbol and edge storage with FTS5 keyword search, recursive CTE graph traversal, impact analysis with risk classification, dead code detection, complexity analysis, and token-budgeted context assembly for LLM prompts.

Design principles: native Rust with zero external binaries, integrates with hKask CNS/OCAP/condenser/MCP framework, recursive CTE traversal (SQL, not in-memory) for persistence and concurrency.

## Public Modules

| Module | Description |
|---|---|
| `error` | Error types: `CodeGraphError`, `IndexError` |
| `graph` | Graph operations. Sub-modules: `analysis` (impact/risk analysis), `context` (`AssembledContext`, `ContextBudget`, `assemble_context()`), `search` (`search()`), `traversal` (recursive CTE traversal) |
| `indexer` | Indexing pipeline. Sub-module: `pipeline` (`IndexPipeline`, `FileIndexResult`, `IndexStats`) |
| `types` | Core types: `Symbol`, `Edge`, `SymbolKind`, `EdgeKind`, `Visibility`, `Complexity`, `Direction` |

## Key Public Types

### `Symbol`

A symbol extracted from source code — a function, struct, trait, etc.

**Fields:**
| Field | Type | Description |
|---|---|---|
| `id` | `Option<i64>` | Database ID (assigned on insert) |
| `name` | `String` | Qualified name, e.g. `"hkask_mcp::runtime::McpRuntime::start"` |
| `kind` | `SymbolKind` | What kind of symbol |
| `file` | `String` | File path relative to workspace root |
| `start_line` | `usize` | Line range start (1-based, inclusive) |
| `end_line` | `usize` | Line range end (1-based, inclusive) |
| `signature` | `String` | First line of the definition |
| `visibility` | `Visibility` | Visibility: pub, pub(crate), or private (default: Private) |
| `doc_comment` | `Option<String>` | Doc comment, if any |
| `complexity` | `Complexity` | Cyclomatic + cognitive complexity (computed lazily, default: NotComputed) |

Derives: `Debug`, `Clone`, `Serialize`, `Deserialize`.

### `SymbolKind`

Enum of symbol kinds. Variants: `Function`, `Method`, `Struct`, `Enum`, `EnumVariant`, `Trait`, `Impl`, `Module`, `Const`, `Static`, `TypeAlias`, `Macro`, `Test`. Derives `Display` (renders as snake_case strings). Serde: `rename_all = "snake_case"`.

### `Visibility`

Symbol visibility enum. Idiomatic Rust: was `String`, now enum — invalid states unrepresentable (D1 fix).

**Variants:** `Public` (`pub`), `Crate` (`pub(crate)` or `pub(super)`), `Private` (default, no modifier). Derives `Display`. Serde: `rename_all = "snake_case"`.

### `Complexity`

Type-state enum for complexity metrics. Replaces `Option<usize>` — makes "has this been computed?" a type-level question (D2 fix).

**Variants:**
| Variant | Fields | Description |
|---|---|---|
| `NotComputed` | — | Not yet computed (default) |
| `Computed` | `cyclomatic: u32`, `cognitive: u32` | Successfully computed |
| `Unparseable` | — | Parse error prevented computation (e.g., macro-heavy code) |

Serde: internally tagged with `tag = "state"`, `rename_all = "snake_case"`.

### `Edge`

An edge between two symbols — a relationship in the code graph.

**Fields:**
| Field | Type | Description |
|---|---|---|
| `id` | `Option<i64>` | Database ID |
| `from_id` | `i64` | Source symbol ID (caller/importer/container) |
| `to_id` | `i64` | Target symbol ID (callee/importee/contained) |
| `kind` | `EdgeKind` | What kind of relationship |
| `file` | `String` | File where the relationship occurs |
| `line` | `usize` | Line where the relationship occurs |
| `target_name` | `String` | Target name for resolution (callee name, import path, etc.). Set by the extractor, used by the pipeline to resolve to_id. Default: empty string (skip_serializing_if empty) |

### `EdgeKind`

Enum of relationship kinds between symbols.

**Variants:**
| Variant | Description |
|---|---|
| `Calls` | A function/method call |
| `Imports` | A `use` import or direct path reference |
| `Implements` | An `impl Trait for Type` relationship |
| `Contains` | Parent-child containment (module contains function, struct contains method) |
| `References` | Ownership/passing reference (type in field, parameter, or return type) |
| `Inherits` | Trait inheritance (`trait Foo: Bar`) |

Derives `Display`. Serde: `rename_all = "snake_case"`.

### `Direction`

Direction for graph traversal.

**Variants:** `Forward` (follow edges from source to target — dependencies), `Reverse` (follow edges from target to source — callers, dependents). Serde: `rename_all = "snake_case"`.

### `AssembledContext`

Token-budgeted context assembly result, containing selected symbols and edges within a `ContextBudget`. Produced by `assemble_context()`.

### `ContextBudget`

Token budget constraints for context assembly.

### `IndexPipeline`

The indexing pipeline that extracts symbols and edges from a Rust source tree, resolves references, and populates the SQLite database.

### `IndexStats`

Aggregate statistics from an index run (symbol count, edge count, file count, timing).

### `FileIndexResult`

Per-file result from the indexing pipeline.

## Public Functions

### `assemble_context()`

```rust
pub fn assemble_context(
    // Token-budgeted context assembly
) -> AssembledContext
```

Constructs a token-budgeted context from the code graph for LLM prompt assembly.

### `search()`

```rust
pub fn search(/* query parameters */)
```

Performs FTS5 keyword search over indexed symbols.

## Error Types

### `CodeGraphError`

Top-level code graph error type.

### `IndexError`

Indexing pipeline error type.

### `Result<T>`

Type alias: `Result<T, CodeGraphError>`.

## Re-exports from Crate Root

`CodeGraphError`, `IndexError`, `Result`, `AssembledContext`, `ContextBudget`, `assemble_context()`, `search()`, `FileIndexResult`, `IndexPipeline`, `IndexStats`, `Complexity`, `Direction`, `Edge`, `EdgeKind`, `Symbol`, `SymbolKind`, `Visibility`. The `analysis` and `traversal` modules are available under `graph::`.
