# Plan: Split `hkask-mcp-memory` → `hkask-mcp-episodic` + `hkask-mcp-semantic`

**Status:** Planned  
**Origin:** P3/13f — Semantic Loop has no MCP server  
**Date:** 2026-06-03

## Problem

`hkask-mcp-memory` is a phantom entry in `bootstrap.rs` — no crate, no directory, no workspace member exists. Meanwhile, hKask has two fundamentally different memory systems (episodic and semantic) with no MCP access to either. The episodic/semantic split in `hkask-memory` and `hkask-storage` reflects a sovereignty boundary: episodic is private/perspective-bound, semantic is shared/perspective-free. MCP access should mirror this split.

## Decision

Delete the phantom `hkask-mcp-memory` and create two servers:

| Server | Memory Type | Sovereignty | Visibility |
|--------|-------------|-------------|------------|
| `hkask-mcp-episodic` | EpisodicMemory | Private (perspective-bound) | `Private` |
| `hkask-mcp-semantic` | SemanticMemory | Public (no perspective) | `Shared` |

## `hkask-mcp-episodic` — Tools

| Tool | Maps to | Notes |
|------|---------|-------|
| `episodic_store` | `EpisodicMemory::store()` | Requires `perspective` (caller's WebID from `HKASK_WEBID`) |
| `episodic_recall` | `EpisodicMemory::query_for_deduped()` | Filters by perspective, applies decay + temporal attention |
| `episodic_retract` | `EpisodicLoop::act()` path | Reduces confidence, doesn't delete |
| `episodic_budget` | `EpisodicMemory::storage_usage()` + `storage_budget()` | Read-only budget info |
| `episodic_ping` | Liveness | Standard MCP health check |

**Sovereignty enforcement:** All tools use the calling agent's `WebID` (resolved from `HKASK_WEBID` env var by `run_stdio_server`) as the `perspective`. An agent cannot read another agent's episodic memory.

## `hkask-mcp-semantic` — Tools

| Tool | Maps to | Notes |
|------|---------|-------|
| `semantic_store` | `SemanticMemory::store()` | `Visibility::Shared`, no perspective |
| `semantic_recall` | `SemanticMemory::query_deduped()` | Public — any agent can read |
| `semantic_search` | `SemanticMemory::search_similar()` | KNN similarity over embeddings |
| `semantic_embed` | `SemanticMemory::store_embedding()` | Index a triple's embedding |
| `semantic_count` | `SemanticMemory::triple_count()` | Read-only count |
| `semantic_ping` | Liveness | Standard MCP health check |

**Consolidation NOT exposed:** The Episodic → Semantic consolidation bridge requires a `ConsolidationToken` issued by the Curation Loop. MCP servers cannot mint this token. Consolidation stays internal (via `CurationLoop::act()`).

## Implementation Steps

### Step 1: Create `mcp-servers/hkask-mcp-episodic/`

```
mcp-servers/hkask-mcp-episodic/
├── Cargo.toml
└── src/
    └── main.rs
```

**Cargo.toml dependencies:**
- `hkask-mcp` (server scaffolding)
- `hkask-types` (WebID, Visibility, Triple)
- `hkask-storage` (Database, TripleStore)
- `hkask-memory` (EpisodicMemory)
- `rmcp`, `tokio`, `serde`, `serde_json`, `schemars`, `tracing`, `anyhow`

**Key design point:** The server opens its own SQLCipher database connection (via `HKASK_EPISODIC_DB` + `HKASK_DB_PASSPHRASE` env vars). It constructs `TripleStore` → `EpisodicMemory` internally. The calling agent's WebID is resolved from `HKASK_WEBID` by `run_stdio_server`.

### Step 2: Create `mcp-servers/hkask-mcp-semantic/`

```
mcp-servers/hkask-mcp-semantic/
├── Cargo.toml
└── src/
    └── main.rs
```

**Cargo.toml dependencies:**
- `hkask-mcp` (server scaffolding)
- `hkask-types` (WebID, Visibility, Triple)
- `hkask-storage` (Database, TripleStore, EmbeddingStore)
- `hkask-memory` (SemanticMemory)
- `rmcp`, `tokio`, `serde`, `serde_json`, `schemars`, `tracing`, `anyhow`

Same pattern: opens its own SQLCipher connection (via `HKASK_SEMANTIC_DB` + `HKASK_DB_PASSPHRASE`).

### Step 3: Add workspace members

In `Cargo.toml` workspace `members`, add:
```toml
"mcp-servers/hkask-mcp-episodic",
"mcp-servers/hkask-mcp-semantic",
```

### Step 4: Update bootstrap

Already done — `bootstrap.rs` now lists `hkask-mcp-episodic` and `hkask-mcp-semantic` instead of `hkask-mcp-memory`.

### Step 5: Update gas estimator tables

In `hkask-cns/src/table_gas_estimator.rs`:
- Remove `"hkask-mcp-memory"` if it exists (currently absent)
- Add `"hkask-mcp-episodic"` with cost 5 (internal memory read)
- Add `"hkask-mcp-semantic"` with cost 5 (internal memory read)

### Step 6: Credential requirements

**`hkask-mcp-episodic`:**
```rust
vec![
    CredentialRequirement::required("HKASK_EPISODIC_DB", "Path to episodic database file"),
    CredentialRequirement::required("HKASK_DB_PASSPHRASE", "SQLCipher encryption passphrase"),
]
```

**`hkask-mcp-semantic`:**
```rust
vec![
    CredentialRequirement::required("HKASK_SEMANTIC_DB", "Path to semantic database file"),
    CredentialRequirement::required("HKASK_DB_PASSPHRASE", "SQLCipher encryption passphrase"),
]
```

### Step 7: Add tests

Each server should have basic unit tests verifying:
- Store + recall round-trip
- Perspective isolation (episodic: agent A can't read agent B's data)
- Shared access (semantic: any agent can read shared data)
- Embedding store + search round-trip (semantic only)
- Budget reporting (episodic only)

## What NOT to do

- **Don't expose consolidation via MCP.** The bridge requires a `ConsolidationToken` from the Curation Loop. MCP servers don't have loop authority.
- **Don't expose retraction directly.** Episodic/semantic retraction is a Cybernetics membrane operation (loop-internal). The MCP `episodic_retract` tool should route through the loop's `act()` path, not call `retract_triple()` directly. If the loop isn't available in the MCP server context, omit this tool from v1 and note it as a gap.
- **Don't merge into one server.** The sovereignty boundary (private/perspective vs shared/no-perspective) is the reason for the split.

## Open questions for implementation

1. **Retraction tool feasibility:** `EpisodicMemory::retract_triple()` is `pub(crate)` — membrane-sealed. Should the MCP server have a `episodic_retract` tool that emits a CNS signal requesting retraction, or should retraction be loop-only?
2. **Embedding model:** `semantic_search` requires embedding vectors. Should the MCP server include an embedding model (e.g., via `hkask-mcp-inference`), or expect pre-computed vectors?
3. **Database sharing:** Should both servers share the same SQLCipher database file (different tables) or use separate files? Separate files is cleaner but requires two passphrases or two salt files.