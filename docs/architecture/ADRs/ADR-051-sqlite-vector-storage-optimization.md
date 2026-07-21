---
title: "ADR-051: SQLite + sqlite-vec Storage and Recall Optimization"
audience: [architects, developers, researchers]
last_updated: 2026-07-15
version: "0.31.0"
status: "Active"
domain: "Data"
mds_categories: [lifecycle, curation, trust]
---

# ADR-051: SQLite + sqlite-vec Storage and Recall Optimization

**Date:** 2026-07-15
**Status:** Active
**Decider:** User (mdz-axolotl)
**Supersedes:** None
**Supersedes:** ADR-050 (extends its embedding pipeline decisions into the storage layer)
**Related:** ADR-043 (Database Driver), ADR-047 (Storage Modularization), ADR-050 (Ontology-Anchored Embedding)

## Context

hKask uses per-pod SQLCipher-encrypted SQLite databases as the sovereignty-preserving
storage surface for h_mems (RDF-style triples), embedding vectors, and CNS observables.
The vector search layer uses sqlite-vec's `vec0` virtual table for KNN similarity search.
A series of CI instabilities and recall-quality investigations surfaced seven distinct
issues spanning native FFI lifecycle, SQLCipher configuration, vec0 identifier design,
distance metric selection, and WAL management. This ADR documents the research, the
decisions made and implemented, and the open questions for the system as it scales.

**Problem Statement:** How should hKask best utilize SQLite + sqlite-vec + SQLCipher for
per-pod storage of h_mem triples and embedding vectors to maximize recall quality,
storage efficiency, and CI reliability while preserving the P1 User Sovereignty invariant
(downloadable, passphrase-encrypted, file-portable databases)?

**Stakeholders:** UserPod agents (recall consumers), curator (consolidation + re-embedding),
users (sovereignty + portability), CI (reliability).

**Constraints:**
- P1 User Sovereignty: databases must remain file-portable, passphrase-encrypted, downloadable
- P3 Generative Space: recall must be semantically meaningful (cosine, not L2)
- Pre-release: no migration or backward compatibility required
- Rust-only: no Python dependencies; native FFI must be explicitly managed

---

## Research Findings

### 1. sqlite-vec Auto-Extension Teardown Segfault

**Source:** [sqlite-vec#169](https://github.com/asg017/sqlite-vec/issues/169),
[sqlite-vec#206](https://github.com/asg017/sqlite-vec/issues/206),
[sqlite-loadable docs](https://docs.rs/sqlite-loadable/latest/sqlite_loadable/)

The `sqlite3_auto_extension` API registers a process-global hook that loads sqlite-vec
on every new SQLite connection. Apple deprecates it outright ("Process-global auto
extensions are not supported on Apple platforms"). The sqlite-vec author himself reports
unreliable segfaults from this path. The hook is never paired with
`sqlite3_reset_auto_extension`, leaving an orphaned function pointer at process exit.

**Finding:** Per-connection loading via `sqlite3_vec_init(conn.handle(), null, null)`
scopes the extension's lifetime to each connection. The extension's native state tears down
with the connection, not orphaned at process exit. This is the documented canonical pattern
for static-linked extensions.

**Implemented:** `init_sqlite_vec_on(&Connection)` replaces the global `Once` +
`sqlite3_auto_extension` in both `hkask-storage-core` and `hkask-codegraph`.

### 2. SQLCipher Reopen Configuration Inconsistency

**Source:** [SQLCipher docs](https://www.zetetic.net/sqlcipher/design/),
[SO cipher_plaintext_header_size](https://stackoverflow.com/questions/26041507/)

`PRAGMA cipher_plaintext_header_size = 32` tells SQLCipher where to find the salt in the
database file. It must be set on **every** connection to a database created with it — not
just on first creation. Omitting it on reopen makes the codec misparse page 1, producing
`hmac check failed` errors and potential native instability.

**Finding:** The original code gated this PRAGMA on `is_new` (first creation only). On
reopen, the pragma was omitted. This was a latent config inconsistency — the raw hex key
mode happened to tolerate the omission for decryption, but the codec was in a suboptimal
state.

**Implemented:** The pragma now runs unconditionally on every connection in `file_pool`'s
`with_init` closure, before `PRAGMA key`. The dead `is_new` field was removed.

### 3. vec0 Identifier Design — TEXT Metadata Column vs Integer Rowid

**Source:** [vec0 docs](https://alexgarcia.xyz/sqlite-vec/features/vec0.html),
[Simon Willison TIL](https://til.simonwillison.net/sqlite/sqlite-vec),
[sqlite-vec#274](https://github.com/asg017/sqlite-vec/issues/274)

vec0's primary key is always an **implicit integer `rowid`**. TEXT columns in the vec0
constructor are **metadata columns**, not primary keys. The docs warn: metadata columns
are "slightly inefficient with long strings (> 12 characters)." Issue #274 documented
spurious `SQLITE_DONE` errors on DELETE with text metadata columns.

The original schema declared `vec0(id TEXT PRIMARY KEY, embedding float[$DIM])` where `id`
was a 36-character UUID. This meant:
- Every KNN scan paid a string-comparison overhead on a metadata column
- The join `v.id = e.id` was a TEXT-to-TEXT scan, not an integer B-tree lookup
- The DELETE path triggered the inefficient >12-char metadata scan

**Finding:** The canonical pattern (from the docs' own examples and Simon Willison) is:
vec0 uses its implicit integer rowid; the UUID lives in the `embeddings` metadata table;
join via `rowid`. This gives integer B-tree lookups (SQLite's fastest path) and eliminates
the metadata-column overhead.

**Implemented:** vec0 constructor changed to `vec0(embedding float[$DIM] distance_metric=cosine)`.
`store()` inserts into `embeddings` first, gets `last_insert_rowid()`, then inserts into
`vec_embeddings (rowid, embedding)`. `search_sql` joins `v.rowid = e.rowid`, selects `e.id`.
`delete()` resolves UUID → rowid via subquery, deletes by integer key.

### 4. Distance Metric — L2 (Default) vs Cosine

**Source:** [vec0 KNN docs](https://alexgarcia.xyz/sqlite-vec/features/knn.html),
[Grokipedia comparison](https://grokipedia.com/page/Comparison_of_sqlite-vec_and_pgvector)

vec0 defaults to **L2 (Euclidean)** distance. The `EmbeddingPort` trait contract says
"Search for similar embeddings by **cosine similarity**." The `cosine_distance()` function
in `hkask-services-compose` computes true cosine distance. The embedding pipeline does
**not normalize** vectors before storage.

For unnormalized vectors, L2 and cosine produce **different rankings**: L2 accounts for
magnitude, cosine ignores it. A high-magnitude vector that's directionally similar can rank
lower under L2 than a low-magnitude vector that's directionally less similar. **The KNN
results were ranked by the wrong metric.**

**Implemented:** `distance_metric=cosine` added to all vec0 declarations (`vec_embeddings`
1024-dim, `symbols_vec` 384-dim in codegraph). This makes the vec0 KNN ranking match the
port trait contract and the `cosine_distance` function used elsewhere.

**Alternative considered:** normalize vectors at ingestion so L2 and cosine converge. Rejected
because explicit `distance_metric=cosine` is more robust (survives provider changes that
don't normalize) and is a one-line schema change vs. a pipeline change.

### 5. SQLCipher Page Size and Vector Storage

**Source:** [Zetetic SQLCipher performance](https://www.zetetic.net/sqlcipher/performance/)

Each 1024-dim `float32` vector = 4096 bytes. SQLCipher's default `cipher_page_size` is 4096
bytes — one vector per encrypted page. The `embeddings` table stores the vector BLOB
redundantly alongside vec0's internal copy (8KB per embedding, all encrypted).

**Decision (documented, not implemented):** The vector redundancy earns its keep. vec0
requires the vector for KNN MATCH; `embeddings.vector` provides uniform retrieval via the
backend-agnostic `DatabaseDriver` query path (`get`/`get_all_by_prefix` work without
branching on SqliteVec vs PgVector). Deduplicating would require backend-conditional
retrieval — more complexity for ~4KB/embedding savings. If per-pod storage becomes a concern
at scale, the escape hatch is a vec0 auxiliary column (`+vector BLOB`) to eliminate the
`embeddings.vector` copy.

### 6. WAL Checkpoint Starvation

**Source:** [SQLite WAL docs](https://sqlite.org/wal.html),
[checkpoint starvation](https://tenthousandmeters.com/blog/sqlite-concurrent-writes-and-database-is-locked-errors/)

With WAL mode and long-lived readers, checkpoints cannot complete if there is always an
active reader, causing the WAL file to grow without bound. With SQLCipher, a bloated WAL
means more per-page crypto on every read (the WAL is encrypted too).

**Implemented:**
- `PRAGMA wal_autocheckpoint = 256` — checkpoints every 256 pages (~1MB WAL) vs the default
  1000 pages. More aggressive auto-checkpointing under the r2d2 acquire-release pattern.
- `Database::checkpoint()` — a public method running `PRAGMA wal_checkpoint(PASSIVE);
  PRAGMA optimize;` for periodic maintenance from the curator loop.
- Pool size reduced from 64 to 8 — fewer concurrent read-transaction holders, less
  starvation risk. Per-pod serves one agent; 8 is generous, overridable via
  `HKASK_DB_POOL_SIZE`.

### 7. mmap and Cache Size Tuning

**Source:** [Zetetic SQLCipher performance](https://www.zetetic.net/sqlcipher/performance/),
[SQLite PRAGMA docs](https://sqlite.org/pragma.html)

**Implemented:**
- `PRAGMA mmap_size = 268435456` (256MB) — memory-maps the encrypted DB file for faster
  reads. Safe with `cipher_plaintext_header_size` set (which enables mmap optimization).
- `PRAGMA cache_size = -65536` (64MB) — keeps more decrypted pages in SQLite's page cache,
  reducing redundant crypto operations on re-reads.

### 8. Pre-Pool Passphrase Verification (SIGSEGV Mitigation)

**Source:** CI run analysis (run 29435074587, 29465386120)

The `hkask-storage` test binary intermittently segfaulted (SIGSEGV, signal 11) during
process teardown after all 71 tests passed. The crash followed the archive tests, which
include a wrong-passphrase test that creates a SQLCipher pool with a bad key. When the pool
drops that connection, the codec cleanup on a corrupted-state connection can crash.

**Implemented:** `file_pool()` now opens a standalone `rusqlite::Connection` probe that
verifies the passphrase (via `SELECT count(*) FROM sqlite_master`) **before** creating the
r2d2 pool. A wrong key returns `PassphraseMismatch` without ever creating a pool — so no
corrupted codec contexts are dropped during teardown.

**Status:** Hypothesis-based fix. The crash could not be reproduced locally (0/20 runs).
CI validation is the falsifier: if the crash persists, the next suspect is r2d2 pool-drop
ordering (test with `max_size(1)`).

---

## Decisions

### D1: Per-connection sqlite-vec loading (not global auto-extension)
**Status:** Implemented (commit `196dafa`)

Replace `sqlite3_auto_extension` with per-connection `sqlite3_vec_init`. Scopes extension
lifetime to each connection. Apple-safe. Eliminates deprecated API usage.

### D2: `cipher_plaintext_header_size = 32` on every connection
**Status:** Implemented (commit `196dafa`)

Set unconditionally in `file_pool`'s `with_init`, before `PRAGMA key`. Removes the `is_new`
conditional. Prevents codec misparse on reopen.

### D3: vec0 keyed on implicit integer rowid
**Status:** Implemented (commit `be04672`)

vec0 constructor drops `id TEXT PRIMARY KEY`. UUID lives only in `embeddings(id TEXT PRIMARY
KEY)`. Join via `v.rowid = e.rowid`. Delete via rowid subquery. Eliminates >12-char metadata
inefficiency and #274-class DELETE risk.

### D4: `distance_metric=cosine` on all vec0 tables
**Status:** Implemented (commit `ae83c8e`)

Matches the `EmbeddingPort` contract ("cosine similarity") and the `cosine_distance()`
function. Corrects a recall-quality defect: unnormalized embeddings were ranked by L2
(magnitude + direction) instead of cosine (direction only).

### D5: SQLite PRAGMA tuning (mmap, cache, autocheckpoint, pool size)
**Status:** Implemented (commit `ae83c8e`)

- `mmap_size = 256MB`, `cache_size = 64MB` — read performance
- `wal_autocheckpoint = 256` — checkpoint starvation prevention
- Pool size 64 → 8 — per-pod right-sizing
- `Database::checkpoint()` — explicit maintenance method

### D6: Pre-pool passphrase verification
**Status:** Implemented (uncommitted, pending CI validation)

Standalone connection probe verifies the key before pool creation. Prevents corrupted
codec contexts from entering the pool. Falsifier: CI crash persistence.

### D7: Vector storage redundancy is intentional
**Status:** Documented decision (no code change)

The 4KB/vector redundancy (vector in both `embeddings` and `vec_embeddings`) preserves the
uniform `DatabaseDriver` retrieval abstraction. Dedup would require backend-conditional
retrieval (SqliteVec vs PgVector). Escape hatch: vec0 `+vector` auxiliary column.

---

## Open Questions for the System as It Scales

### Q1: Treating h_mems as Graphs (RDF Triple → Graph Querying)

The `hmems` table stores RDF-style triples: `(entity, attribute, value)` with temporal
metadata (`valid_from`, `valid_to`, `recalled_at`) and provenance (`perspective`,
`confidence`, `owner_webid`). Current query patterns are exact-match SQL:
`WHERE entity = ?`, `WHERE attribute = ?`, `WHERE entity = ? AND attribute = ?`.

**To treat h_mems as a graph, consider:**

1. **Graph traversal queries** — multi-hop reasoning ("what does entity X know about
   entity Y through attribute Z?") requires recursive CTEs or a graph query layer. SQLite
   supports `WITH RECURSIVE` CTEs, which can express bounded-depth traversal without a
   separate graph database. For unbounded traversal or SPARQL-like pattern matching, a
   dedicated layer would be needed.

2. **Entity/relation indexes** — the current indexes (`idx_hmems_entity`,
   `idx_hmems_attribute`, `idx_hmems_entity_attribute`) support point lookups but not
   graph-join patterns (e.g., "find all entities connected to X through any attribute").
   A composite index on `(entity, attribute, value)` or a separate edge table would
   accelerate graph queries.

3. **FTS5 on triple text** — an FTS5 virtual table on `entity || ' ' || attribute || ' ' ||
   value` would enable keyword-grounded recall over triples, complementing the vector KNN
   path. This is the hybrid search pattern already implemented in `hkask-codegraph`'s
   `graph/search.rs` (FTS5 + BM25 + LIKE fallback).

4. **SQLCipher encryption of shadow tables** — FTS5 and vec0 shadow tables live in the same
   SQLite file, so they are encrypted by SQLCipher automatically. No additional encryption
   configuration is needed for graph/FTS5 structures stored alongside h_mems.

### Q2: TransE Embeddings of h_mem Triples

**Should we create TransE embeddings of the memory triples too?**

TransE (Bordes et al., 2013) models relations as translations in vector space:
`head + relation ≈ tail`. The scoring function is `||h + r - t||`, minimized for valid
triples. This is fundamentally different from text embeddings (which encode semantic
content of a string):

| Dimension | Text Embeddings (current) | TransE / KGE |
|-----------|---------------------------|--------------|
| What it encodes | Semantic content of entity/value text | Structural relationships between entities |
| Training | Pre-trained on corpus (provider) | Trained on the pod's own triples |
| Query type | "find textually similar memories" | "find structurally related entities" / link prediction |
| Update cost | API call per embedding | Local training pass over triples |
| Cold start | Works immediately (pre-trained) | Requires enough triples to train (100s+) |
| Recall type | Content similarity | Graph completion / link prediction |

**Recommendation: Do NOT replace text embeddings with TransE. Do consider ADDING TransE
as a complementary signal.**

Rationale:
- Text embeddings answer "what memories are semantically similar to this query?" — this is
  the primary recall path for RAG and curator consolidation.
- TransE answers "what entities are structurally related?" and "what links are missing?"
  (link prediction) — this is graph completion, a different task.
- The two are **complementary**, not competitive. A fused recall score
  `α · cosine(text_embedding) + β · transE_score(triple)` would combine content similarity
  with structural plausibility.
- TransE requires **local training** on the pod's own triples (not a pre-trained model).
  This is feasible (TransE is a simple margin-based loss, trainable in seconds on
  thousands of triples) but adds a training step to the curator's consolidation loop.
- **Cold start problem:** a new pod with <100 triples has insufficient data to train TransE.
  Text embeddings work from the first triple. TransE should be gated on a minimum triple
  count.
- **TransE limitations:** the basic TransE scoring function (`h + r ≈ t`) cannot model
  1-to-many, many-to-1, or many-to-many relations. TransH, TransR, or RotatE address this
  but add complexity. For hKask's entity-attribute-value model (which is inherently
  many-to-many), TransR or RotatE would be more appropriate than vanilla TransE.

**If pursued, the implementation path would be:**
1. Store TransE entity/relation vectors in a second vec0 table (`vec_triples` with
   `entity_vec float[D]`, `relation_vec float[D]`) — or in regular BLOB columns since KNN
   is not the primary query (triple scoring is).
2. Train during curator consolidation: iterate over the pod's triples, minimize the
   margin-based loss (`max(0, margin + d(h+r, t) - d(h+r, t')`).
3. Use for link prediction: given `(entity, attribute, ?)`, score candidate values by
   `||entity_vec + attribute_vec - value_vec||`.
4. Fuse with text embedding KNN for hybrid recall.

**Defer until:** the pod has enough triples (500+) to justify training, and a concrete
graph-completion use case emerges (e.g., the curator asking "what attributes might entity
X have that we haven't observed?").

### Q3: sqlite-vec Scale Limits

sqlite-vec v0.1.x is **brute-force KNN** — no ANN index ([issue #25](https://github.com/asg017/sqlite-vec/issues/25)).
At 1024 dimensions, [issue #186](https://github.com/asg017/sqlite-vec/issues/186) reports
searches becoming "very slow" at ~250k vectors. For per-pod recall (thousands of vectors),
brute-force is ~ms. For pods approaching 100k+ vectors, consider:
- Routing large pods to the `PostgresDriver` + `pgvector` path (with HNSW/IVFFlat indexes)
  — the `VectorBackend` enum already supports this
- sqlite-vec's planned ANN support (DiskANN/IVF, pre-v1.0)
- Quantization (UINT8, 5× faster, recall >0.95) — available in the SQLite Cloud fork, not
  upstream v0.1.x

### Q4: vec0 Shadow Table Bloat Under Re-Embedding

The curator's re-embedding pipeline (consolidation, prefix-purge, re-ingest) performs
frequent INSERT/DELETE cycles on vec0. vec0 shadow tables (like FTS5) are not fully
reclaimed by `VACUUM`. After many cycles, dead pages accumulate. Consider periodic
`PRAGMA incremental_vacuum` or a vec0 table rebuild (DROP + CREATE + re-INSERT) during
maintenance checkpoints.

### Q5: Hybrid FTS5 + Vector Search for RAG

The codegraph crate demonstrates the pattern: FTS5 keyword search (BM25 ranking) +
vec0 vector KNN (cosine distance), with a LIKE fallback. For the embedding store's RAG
path, hybrid search would:
1. Add an FTS5 virtual table on a text representation of the triple or chunk
   (`entity || ' ' || attribute || ' ' || value`)
2. Run FTS5 (keyword grounding) + vec0 KNN (semantic similarity) in parallel
3. Fuse scores via normalization (e.g., reciprocal rank fusion: `1/(k+rank_fts) + 1/(k+rank_knn)`)

This requires a text column to index. Currently `entity_ref` is a short ID, not searchable
text. The FTS5 index should be on the triple's natural-language rendering, not the raw
column values. Defer until the recall layer has a content field to index.

### Q6: SQLCipher Concurrency Under WAL

SQLCipher is a SQLite fork with a re-implemented WAL. Historical issues ([sqlcipher#67](https://github.com/sqlcipher/sqlcipher/issues/67))
show that concurrent read/write with `NOMUTEX` + WAL doesn't always behave like vanilla
SQLite. The `delete()` path already works around this correctly (single-writer-per-
transaction on one connection). All write paths should maintain this discipline: never
acquire a second pool connection within a transaction.

---

## Consequences

### Positive
- **Recall quality:** cosine metric + integer rowid join = correct ranking + fast lookups
- **CI reliability:** per-connection extension loading + pre-pool key verification =
  no orphaned native state at teardown
- **Read performance:** mmap + 64MB cache + aggressive autocheckpoint = fewer decrypt ops
- **Resource efficiency:** pool size 8 (not 64) = less native teardown surface, less
  checkpoint starvation
- **Sovereignty preserved:** all changes are within the SQLite/SQLCipher file-portable
  model. No external database required. Postgres remains optional for large-corpus pods.

### Negative
- **Pre-pool probe overhead:** every `file_pool()` call now opens an extra connection for
  key verification (~1ms). Acceptable for a cached pool (called once per Database lifetime).
- **SIGSEGV fix unconfirmed:** the pre-pool verification is a hypothesis-based fix. CI
  validation is the falsifier. If the crash persists, pool-drop ordering is the next suspect.
- **Vector storage redundancy:** 4KB/embedding stored twice. Documented as intentional;
  escape hatch documented if it becomes a concern.

### Neutral
- **No migration needed:** pre-release status allows clean schema changes. Existing pod
  databases (test-only) are regenerated on next open.

---

## References

- [sqlite-vec v0.1.0 stable release](https://alexgarcia.xyz/blog/2024/sqlite-vec-stable-release/index.html) — Alex Garcia
- [vec0 virtual table docs](https://alexgarcia.xyz/sqlite-vec/features/vec0.html) — metadata columns, partition keys, auxiliary columns
- [vec0 KNN docs](https://alexgarcia.xyz/sqlite-vec/features/knn.html) — distance_metric configuration
- [sqlite-vec#169](https://github.com/asg017/sqlite-vec/issues/169) — auto-extension segfaults
- [sqlite-vec#25](https://github.com/asg017/sqlite-vec/issues/25) — ANN index tracking
- [sqlite-vec#186](https://github.com/asg017/sqlite-vec/issues/186) — 1024-dim performance
- [sqlite-vec#274](https://github.com/asg017/sqlite-vec/issues/274) — DELETE text metadata bug
- [SQLCipher performance optimization](https://www.zetetic.net/sqlcipher/performance/) — page size, mmap, KDF
- [SQLite WAL documentation](https://sqlite.org/wal.html) — checkpoint starvation
- [TransE (Bordes et al., 2013)](https://papers.nips.cc/paper/2013/hash/1cecc7a77928ca8133fa24680a88d2f9-Abstract.html) — translational distance models
- [Knowledge Graph Embedding survey](https://arxiv.org/pdf/2410.14733) — TransE/TransH/TransR/RotatE comparison
- [Simon Willison: sqlite-vec TIL](https://til.simonwillison.net/sqlite/sqlite-vec) — integer rowid pattern
- ADR-050: Ontology-Anchored Embedding (tag → embed pipeline)
- ADR-047: Storage Crate Modularization
- ADR-043: Database Driver