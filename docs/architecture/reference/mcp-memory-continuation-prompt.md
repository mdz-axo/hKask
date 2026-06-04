# Continuation Prompt: Implement `hkask-mcp-episodic` and `hkask-mcp-semantic`

Copy the prompt below into a fresh agent session to implement the two MCP servers.

---

## Prompt

You are implementing two new MCP servers for the hKask agent platform: `hkask-mcp-episodic` and `hkask-mcp-semantic`. These replace the phantom `hkask-mcp-memory` entry that never existed as a real crate.

The project is at `/home/mdz-axolotl/Clones/hKask`. Before writing any code, read `hKask/AGENTS.md` for project constraints (headless, no visual UI, no excess complexity). Then read the plan document at `hKask/docs/architecture/reference/mcp-memory-split-plan.md`.

### What already exists

- `bootstrap.rs` (line 252-269) already lists `"hkask-mcp-episodic"` and `"hkask-mcp-semantic"` instead of the old `"hkask-mcp-memory"`
- The gas estimator table in `crates/hkask-cns/src/table_gas_estimator.rs` does NOT yet have entries for these servers — you must add them
- The workspace `Cargo.toml` does NOT yet list these servers as members — you must add them
- `hkask-memory` crate already has `EpisodicMemory`, `SemanticMemory`, `ConsolidationBridge`
- `hkask-storage` crate has `Database`, `TripleStore`, `EmbeddingStore`, `Triple`
- `hkask-mcp` crate has all the scaffolding: `run_stdio_server`, `mcp_server_main!`, `CredentialRequirement`, `ServerContext`, `ToolSpanGuard`, `McpToolOutput`, `McpToolError`, `validate_identifier`

### Architecture decisions (already resolved)

1. **Two separate servers**, not one merged server. The sovereignty boundary is the reason: episodic is private/perspective-bound, semantic is shared/perspective-free.
2. **Consolidation is NOT exposed** via MCP. The bridge requires a `ConsolidationToken` issued by the Curation Loop. MCP servers cannot mint this token.
3. **Retraction is NOT exposed** in v1. `retract_triple()` is `pub(crate)` — membrane-sealed. Omit `episodic_retract` from v1 and add a doc comment noting it as a gap.
4. **Embedding search** expects pre-computed vectors. The MCP server does NOT include an embedding model. Callers provide vectors via `semantic_embed` and search via `semantic_search`.
5. **Separate database files** for each server. Use `HKASK_EPISODIC_DB` and `HKASK_SEMANTIC_DB` env vars respectively, with `HKASK_DB_PASSPHRASE` for both.

### Server 1: `hkask-mcp-episodic`

**Directory:** `mcp-servers/hkask-mcp-episodic/`

**Cargo.toml** — follow the pattern from `mcp-servers/hkask-mcp-ocap/Cargo.toml`:
```toml
[package]
name = "hkask-mcp-episodic"
version.workspace = true
edition.workspace = true
license.workspace = true
description = "Episodic memory store and recall MCP server"

[dependencies]
hkask-mcp = { path = "../../crates/hkask-mcp" }
hkask-types = { path = "../../crates/hkask-types" }
hkask-storage = { path = "../../crates/hkask-storage" }
hkask-memory = { path = "../../crates/hkask-memory" }
rmcp = { workspace = true, features = ["server", "macros", "transport-io"] }
tokio.workspace = true
serde.workspace = true
serde_json.workspace = true
schemars.workspace = true
tracing.workspace = true
anyhow.workspace = true
```

**Tools:**

| Tool | Description | Maps to |
|------|-------------|---------|
| `episodic_ping` | Liveness and storage info | Health check |
| `episodic_store` | Store an episodic triple | `EpisodicMemory::store()` |
| `episodic_recall` | Recall triples by entity | `EpisodicMemory::query_for_deduped()` |
| `episodic_budget` | Storage usage and budget | `EpisodicMemory::storage_usage()` + `storage_budget()` |

**Sovereignty enforcement:** The calling agent's `WebID` (from `ctx.webid`, resolved by `run_stdio_server` from `HKASK_WEBID`/`HKASK_AGENT_PERSONA`) is used as the `perspective` for all operations. An agent CANNOT read another agent's episodic memory.

**Request types (derive `Deserialize` + `JsonSchema`):**

```rust
StoreRequest {
    entity: String,
    attribute: String,
    value: serde_json::Value,  // JSON value for the triple
    confidence: Option<f64>,    // defaults to 1.0
}

RecallRequest {
    entity: String,
}

BudgetRequest {}  // no params
```

**Server struct:**

```rust
pub struct EpisodicServer {
    memory: EpisodicMemory,
    webid: WebID,
}
```

**Constructor:** Opens a SQLCipher database from `HKASK_EPISODIC_DB` + `HKASK_DB_PASSPHRASE`, constructs `TripleStore` → `EpisodicMemory`. Use `Database::open(path, passphrase)` from `hkask-storage`.

**`episodic_store` implementation:**
1. Build a `Triple::new(entity, attribute, value, owner_webid)` with `.with_perspective(self.webid)` and `.with_confidence(confidence.unwrap_or(1.0))` and `.with_visibility(Visibility::Private)`
2. Call `self.memory.store(triple)`
3. Return `McpToolOutput` with `stored: true, entity, attribute`

**`episodic_recall` implementation:**
1. Call `self.memory.query_for_deduped(entity, self.webid)`
2. Serialize triples to JSON array with `{entity, attribute, value, confidence, valid_from}` fields
3. Return `McpToolOutput` with `count, triples`

**`episodic_budget` implementation:**
1. Call `self.memory.storage_usage(&self.webid)` and `self.memory.storage_budget()`
2. Return `McpToolOutput` with `used, budget, remaining`

**`mcp_server_main!` invocation** — custom factory with credentials:

```rust
hkask_mcp::mcp_server_main!(
    "hkask-mcp-episodic",
    factory: |ctx: hkask_mcp::ServerContext| {
        let db_path = ctx.credentials.get("HKASK_EPISODIC_DB")
            .ok_or_else(|| anyhow::anyhow!("Missing HKASK_EPISODIC_DB"))?
            .clone();
        let passphrase = ctx.credentials.get("HKASK_DB_PASSPHRASE")
            .ok_or_else(|| anyhow::anyhow!("Missing HKASK_DB_PASSPHRASE"))?
            .clone();
        let db = hkask_storage::Database::open(&db_path, &passphrase)
            .map_err(|e| anyhow::anyhow!("Failed to open episodic database: {}", e))?;
        let conn = db.conn_arc();
        let triple_store = hkask_storage::TripleStore::new(conn);
        let memory = hkask_memory::EpisodicMemory::new(triple_store);
        Ok(EpisodicServer::new(memory, ctx.webid))
    },
    credentials: vec![
        hkask_mcp::CredentialRequirement::required("HKASK_EPISODIC_DB", "Path to episodic database file"),
        hkask_mcp::CredentialRequirement::required("HKASK_DB_PASSPHRASE", "SQLCipher encryption passphrase"),
    ]
);
```

### Server 2: `hkask-mcp-semantic`

**Directory:** `mcp-servers/hkask-mcp-semantic/`

**Cargo.toml** — same pattern:
```toml
[package]
name = "hkask-mcp-semantic"
version.workspace = true
edition.workspace = true
license.workspace = true
description = "Semantic memory store, recall, and similarity search MCP server"

[dependencies]
hkask-mcp = { path = "../../crates/hkask-mcp" }
hkask-types = { path = "../../crates/hkask-types" }
hkask-storage = { path = "../../crates/hkask-storage" }
hkask-memory = { path = "../../crates/hkask-memory" }
rmcp = { workspace = true, features = ["server", "macros", "transport-io"] }
tokio.workspace = true
serde.workspace = true
serde_json.workspace = true
schemars.workspace = true
tracing.workspace = true
anyhow.workspace = true
```

**Tools:**

| Tool | Description | Maps to |
|------|-------------|---------|
| `semantic_ping` | Liveness and storage info | Health check |
| `semantic_store` | Store a shared semantic triple | `SemanticMemory::store()` |
| `semantic_recall` | Recall triples by entity | `SemanticMemory::query_deduped()` |
| `semantic_embed` | Store an embedding vector | `SemanticMemory::store_embedding()` |
| `semantic_search` | KNN similarity search | `SemanticMemory::search_similar()` |
| `semantic_count` | Triple and embedding counts | `SemanticMemory::triple_count()` + `embedding_count()` |

**Request types:**

```rust
StoreRequest {
    entity: String,
    attribute: String,
    value: serde_json::Value,
    confidence: Option<f64>,  // defaults to 1.0
}

RecallRequest {
    entity: String,
}

EmbedRequest {
    entity_ref: String,
    vector: Vec<f32>,
    model: String,
}

SearchRequest {
    query_vector: Vec<f32>,
    limit: Option<usize>,  // defaults to 10
}

CountRequest {}  // no params
```

**Server struct:**

```rust
pub struct SemanticServer {
    memory: SemanticMemory,
    webid: WebID,
}
```

**Constructor:** Opens a SQLCipher database from `HKASK_SEMANTIC_DB` + `HKASK_DB_PASSPHRASE`, constructs `TripleStore` + `EmbeddingStore` → `SemanticMemory`. Use `Database::open(path, passphrase)` from `hkask-storage`.

**`semantic_store` implementation:**
1. Build `Triple::new(entity, attribute, value, owner_webid)` with `.with_visibility(Visibility::Shared)` and `.with_confidence(confidence.unwrap_or(1.0))`. Do NOT set perspective — semantic triples have `perspective: None`.
2. Call `self.memory.store(triple)`
3. Return `McpToolOutput` with `stored: true, entity, attribute`

**`semantic_recall` implementation:**
1. Call `self.memory.query_deduped(entity)` — returns all Shared triples for that entity, no perspective filtering
2. Serialize to JSON array
3. Return `McpToolOutput` with `count, triples`

**`semantic_embed` implementation:**
1. Call `self.memory.store_embedding(entity_ref, &vector, model)`
2. Return `McpToolOutput` with `stored: true, entity_ref, model, dimensions: vector.len()`

**`semantic_search` implementation:**
1. Call `self.memory.search_similar(&query_vector, limit.unwrap_or(10))`
2. Serialize `SimilarityResult` items to JSON array
3. Return `McpToolOutput` with `count, results`

**`semantic_count` implementation:**
1. Call `self.memory.triple_count()` and `self.memory.embedding_count()`
2. Return `McpToolOutput` with `triple_count, embedding_count`

**`mcp_server_main!` invocation:**

```rust
hkask_mcp::mcp_server_main!(
    "hkask-mcp-semantic",
    factory: |ctx: hkask_mcp::ServerContext| {
        let db_path = ctx.credentials.get("HKASK_SEMANTIC_DB")
            .ok_or_else(|| anyhow::anyhow!("Missing HKASK_SEMANTIC_DB"))?
            .clone();
        let passphrase = ctx.credentials.get("HKASK_DB_PASSPHRASE")
            .ok_or_else(|| anyhow::anyhow!("Missing HKASK_DB_PASSPHRASE"))?
            .clone();
        let db = hkask_storage::Database::open(&db_path, &passphrase)
            .map_err(|e| anyhow::anyhow!("Failed to open semantic database: {}", e))?;
        let conn = db.conn_arc();
        let triple_store = hkask_storage::TripleStore::new(Arc::clone(&conn));
        let embedding_store = hkask_storage::EmbeddingStore::new(conn);
        let memory = hkask_memory::SemanticMemory::new(triple_store, embedding_store);
        Ok(SemanticServer::new(memory, ctx.webid))
    },
    credentials: vec![
        hkask_mcp::CredentialRequirement::required("HKASK_SEMANTIC_DB", "Path to semantic database file"),
        hkask_mcp::CredentialRequirement::required("HKASK_DB_PASSPHRASE", "SQLCipher encryption passphrase"),
    ]
);
```

### Changes to existing files

1. **`Cargo.toml` (workspace root)** — add to `workspace.members` array:
   ```toml
   "mcp-servers/hkask-mcp-episodic",
   "mcp-servers/hkask-mcp-semantic",
   ```

2. **`crates/hkask-cns/src/table_gas_estimator.rs`** — add to `default_gas_table()`:
   ```rust
   table.insert("hkask-mcp-episodic", 5);
   table.insert("hkask-mcp-semantic", 5);
   ```
   Also update the test `default_table_has_all_servers` to assert these entries.

3. **`crates/hkask-cns/src/composite_gas_estimator.rs`** — check if there are any tests referencing `"hkask-mcp-memory"` and update them to use the new server names.

### Follow these patterns exactly

Every tool method MUST:
1. Start with `let span = ToolSpanGuard::new("tool_name", &self.webid);`
2. Validate inputs with `validate_identifier()` where appropriate (entity, attribute — max 256 chars)
3. Return `span.ok(McpToolOutput::new(json!({...})).to_json_string())` on success
4. Return `span.error(McpErrorKind::Xxx, McpToolError::xxx("message").to_json_string())` on error

Look at `mcp-servers/hkask-mcp-ocap/src/main.rs` as the canonical reference for the server pattern.

### Key API details

**`EpisodicMemory`** (`crates/hkask-memory/src/episodic.rs`):
- `new(triple_store: TripleStore) -> Self`
- `store(&self, triple: Triple) -> Result<(), EpisodicMemoryError>` — rejects `Visibility::Shared`, requires `perspective`
- `query_for_deduped(&self, entity: &str, perspective: WebID) -> Result<Vec<Triple>, EpisodicMemoryError>` — filters by perspective, applies Bayesian decay + temporal attention + dedup
- `storage_usage(&self, perspective: &WebID) -> Result<usize, EpisodicMemoryError>`
- `storage_budget(&self) -> usize`

**`SemanticMemory`** (`crates/hkask-memory/src/semantic.rs`):
- `new(triple_store: TripleStore, embedding_store: EmbeddingStore) -> Self`
- `store(&self, triple: Triple) -> Result<(), SemanticMemoryError>` — requires `Visibility::Shared`, rejects `perspective`
- `query_deduped(&self, entity: &str) -> Result<Vec<Triple>, SemanticMemoryError>` — returns all Shared triples for entity
- `store_embedding(&self, entity_ref: &str, vector: &[f32], model: &str) -> Result<String, SemanticMemoryError>`
- `search_similar(&self, query_vector: &[f32], limit: usize) -> Result<Vec<SimilarityResult>, SemanticMemoryError>`
- `triple_count(&self) -> Result<usize, SemanticMemoryError>`
- `embedding_count(&self) -> Result<usize, SemanticMemoryError>`

**`Triple`** (`crates/hkask-storage/src/triples.rs`):
- `Triple::new(entity, attribute, value: Value, owner_webid: WebID) -> Self`
- `.with_confidence(f64) -> Self`
- `.with_perspective(WebID) -> Self`
- `.with_visibility(Visibility) -> Self`

**`Database`** (`crates/hkask-storage/src/database.rs`):
- `Database::open(path: &str, passphrase: &str) -> Result<Self, DatabaseError>`
- `Database::in_memory() -> Result<Self, DatabaseError>`
- `.conn_arc() -> Arc<Mutex<Connection>>`

**`Visibility`** enum: `Private`, `Shared` (from `hkask-types`)

### Verification

After all implementation:
1. `cargo check -p hkask-mcp-episodic`
2. `cargo check -p hkask-mcp-semantic`
3. `cargo check -p hkask-cns` (gas table changes)
4. `cargo test -p hkask-mcp-episodic`
5. `cargo test -p hkask-mcp-semantic`
6. `cargo test -p hkask-cns`

All must pass with no warnings from our changes.