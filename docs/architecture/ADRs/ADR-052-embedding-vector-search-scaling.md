---
title: "ADR-052: Embedding Vector Search Scaling Path — sqlite-vec to pgvector"
audience: [architects, developers, researchers]
last_updated: 2026-07-15
version: "0.31.0"
status: "Active"
domain: "Data"
mds_categories: [lifecycle, curation]
---

# ADR-052: Embedding Vector Search Scaling Path — sqlite-vec to pgvector

**Date:** 2026-07-15
**Status:** Active
**Decider:** User (mdz-axolotl)
**Supersedes:** None
**Related:** ADR-043 (Database Driver), ADR-047 (Storage Modularization), ADR-051 (SQLite + sqlite-vec Storage Optimization)

## Context

ADR-051 documented sqlite-vec's brute-force KNN limitation (no ANN index, [issue #25](https://github.com/asg017/sqlite-vec/issues/25))
and the 1024-dim performance cliff at ~250k vectors ([issue #186](https://github.com/asg017/sqlite-vec/issues/186)).
The curator's consolidation and re-embedding pipeline is expected to generate
250k+ h_mems and their embeddings within a month of active use. This ADR
addresses the scaling path: when and how the embedding vector search should
graduate from sqlite-vec to pgvector.

**Problem Statement:** sqlite-vec's brute-force KNN will become too slow for
real-time recall at the curator's expected growth rate. What is the right
scaling path, and when should the switch happen?

**Critical distinction:** h_mems (RDF triples) and embeddings have different
scaling profiles:
- **h_mems in SQLite:** NOT a scaling problem. SQLite handles billions of rows
  in regular B-tree-indexed tables. 250k triples is trivial — point lookups on
  `(entity, attribute)` indexes complete in microseconds. h_mems stay in
  SQLite/SQLCipher permanently for P1 sovereignty.
- **Embedding vectors in sqlite-vec:** THIS is the scaling wall. Brute-force
  KNN scans all vectors per query. At 250k × 1024-dim, query latency degrades
  to hundreds of milliseconds or seconds.

## Research Findings

### sqlite-vec Performance at Scale

**Source:** [sqlite-vec v0.1.0 release blog](https://alexgarcia.xyz/blog/2024/sqlite-vec-stable-release/index.html),
[issue #186](https://github.com/asg017/sqlite-vec/issues/186),
[DEV.to analysis](https://dev.to/zoricic/sqlite-as-a-vector-database-yes-really-4cm4)

Alex Garcia's own benchmarks (100k vectors, file-backed, various dimensions):

| Dimensions | Query time (100k vectors) | Assessment |
|------------|--------------------------|------------|
| 192 | ~15ms | Excellent |
| 384 | ~30ms | Good |
| 768 | ~50ms | Acceptable |
| 1024 | ~75ms | Borderline |
| 1536 | ~105ms | Slow |
| 3072 | ~214ms | Too slow |

At 100k × 1024-dim, sqlite-vec is ~75ms — "below the golden target of 100ms."
But issue #186 reports a user at **250k × 1024-dim** finding searches "very slow."
Linear extrapolation: 250k would be ~187ms per query. With SQLCipher encryption
overhead (every page read requires AES-256 decrypt), realistically 300-500ms.

The DEV.to analysis confirms: *"You don't need ANN until you're well past 100K
vectors."* At 250k+, brute-force is no longer viable for real-time recall.

### pgvector HNSW Performance at Scale

**Source:** [AWS pgvector benchmarks](https://aws.amazon.com/blogs/database/accelerate-hnsw-indexing-and-searching-with-pgvector-on-amazon-aurora-postgresql-compatible-edition-and-amazon-rds-for-postgresql/),
[ParadeDB tuning guide](https://www.paradedb.com/learn/postgresql/tuning-pgvector),
[Instaclustr benchmarks](https://www.instaclustr.com/education/vector-database/pgvector-performance-benchmark-results-and-5-ways-to-boost-performance/)

pgvector with HNSW index at 250k × 1024-dim:

| Metric | HNSW | Sequential scan (no index) |
|--------|------|---------------------------|
| Query latency | **1.5-3ms** | 650ms |
| Recall | 95-99% | 100% (exact) |
| Index build time | Minutes | N/A |
| Memory | High (graph must stay resident) | Low |

HNSW provides **~100-200× speedup** over brute-force at 250k vectors, with
95-99% recall. The tradeoff is approximate results and higher memory usage.

### Matryoshka Dimensionality Reduction (Interim Mitigation)

**Source:** [vec0 API reference](https://alexgarcia.xyz/sqlite-vec/api-reference.html),
ADR-050 (ontology-anchored embedding)

Many modern embedding models (including mxbai-embed-large, hKask's 1024-dim
default) support **Matryoshka representation learning** — vectors can be
truncated to fewer dimensions with graceful quality degradation. sqlite-vec
provides `vec_slice()` for this.

| Dim | sqlite-vec query (100k) | Recall impact |
|-----|------------------------|---------------|
| 1024 | ~75ms | Baseline |
| 512 | ~38ms | ~5-10% recall loss |
| 256 | ~19ms | ~15-25% recall loss |

Slicing 1024→512 roughly halves query time, buying time before the Postgres
migration. But it's a stopgap, not a solution — at 500k vectors even 512-dim
brute-force would be ~190ms.

### The Existing Architecture Already Supports the Switch

**Source:** `crates/hkask-storage/src/embeddings.rs`

The `VectorBackend` enum already branches:
```
enum VectorBackend {
    SqliteVec { pool, dim },
    PgVector { pool, handle, dim },
}
```

`EmbeddingStore::from_driver()` selects the backend based on
`driver.provider()` — `SqliteDriver` → `SqliteVec`, `PostgresDriver` →
`PgVector`. The `PostgresDriver` is already implemented
([`crates/hkask-storage/src/database/postgres.rs`](crates/hkask-storage/src/database/postgres.rs),
250 lines, `impl DatabaseDriver`). The search SQL for both paths is already
written and tested.

**The switch is a routing decision, not a rewrite.**

## Decision

### D1: h_mems stay in SQLite/SQLCipher permanently

h_mems (RDF triples) are regular indexed rows. SQLite handles millions+
trivially. They are the P1 sovereignty surface — downloadable, encrypted,
portable. No Postgres migration for h_mems. Full stop.

### D2: Embeddings graduate to pgvector when a pod exceeds the sqlite-vec practical limit

**Threshold: 100k vectors per pod.**

Below 100k × 1024-dim, sqlite-vec brute-force is <75ms — acceptable for
real-time recall. Above 100k, latency degrades nonlinearly with SQLCipher
overhead. The threshold is conservative (the DEV.to analysis puts the wall
at "well past 100k"; the sqlite-vec author's benchmarks show 100k as the
"golden target" boundary).

**Routing mechanism:** `EmbeddingStore::from_driver()` already selects the
backend based on `DbProvider`. When a pod is configured with a Postgres
connection (via `PostgresDriver`), embeddings automatically use pgvector with
HNSW indexes. When configured with SQLite (via `SqliteDriver`), embeddings
use sqlite-vec. This is a deployment/configuration decision, not a code change.

### D3: Matryoshka dimensionality reduction as an interim mitigation

For pods approaching but not yet exceeding the 100k threshold, reduce the
embedding dimension from 1024 to 512 via `vec_slice()` before storing in vec0.
This halves query time with ~5-10% recall loss. The `HKASK_EMBEDDING_DIM`
environment variable already controls the dimension — setting it to 512
configures the whole pipeline (schema, vec0, embedding model) for lower-dim
operation.

This is a stopgap for pods in the 50k-100k range. Pods above 100k should
switch to Postgres.

### D4: Postgres embeddings do NOT break sovereignty

The P1 sovereignty invariant applies to **h_mems** (the user's knowledge
triples), not to embedding vectors (which are derived from h_mems and can
be recomputed). Embeddings are a cache — they can be rebuilt from the h_mems
via the curator's re-embedding pipeline. Storing embeddings in Postgres
while h_mems stay in SQLCipher preserves sovereignty: the user's data is
portable; the derived index is a performance optimization.

The sovereignty archive (`BackupArchive`) exports h_mems, not embeddings.
A pod that migrates embeddings to Postgres can still export/import its h_mems
as an encrypted file. Embeddings are recomputed on the new backend.

### D5: Do NOT switch everything to Postgres

Postgres is not a replacement for SQLite/SQLCipher in hKask. It is a
complementary backend for the embedding vector search surface only. The
system remains local-first, file-portable, and sovereignty-preserving by
default. Postgres is an opt-in scaling layer for pods that outgrow sqlite-vec.

## Implementation Status

| Component | Status |
|-----------|--------|
| `VectorBackend::PgVector` variant | ✅ Implemented |
| `PostgresDriver` (`impl DatabaseDriver`) | ✅ Implemented (250 lines) |
| `EmbeddingStore::from_driver()` routing | ✅ Implemented (selects on `DbProvider`) |
| pgvector search SQL | ✅ Implemented (`embedding <-> ?1::vector` with `ORDER BY distance LIMIT`) |
| pgvector store/delete | ✅ Implemented |
| HNSW index creation | ❌ Not yet (pgvector default is exact/sequential scan) |
| Pod-level threshold routing | ❌ Not yet (currently static config, not dynamic) |
| Matryoshka `vec_slice` pipeline | ❌ Not yet (dim is configurable but no auto-slice) |

## Open Work

### W1: Add HNSW index creation to the pgvector schema

The pgvector path currently does exact nearest neighbor (sequential scan),
which has the same O(n) scaling as sqlite-vec brute-force. The HNSW index
must be created for the 100-200× speedup:

```sql
CREATE INDEX ON embeddings USING hnsw (embedding vector_cosine_ops)
  WITH (m = 16, ef_construction = 64);
```

This should be added to the Postgres schema initialization path.

### W2: Pod-level threshold routing (future)

Currently, the backend is selected statically at pod creation time based on
the configured `DatabaseDriver`. A more sophisticated routing mechanism would
monitor the embedding count and automatically migrate from sqlite-vec to
pgvector when a pod crosses the 100k threshold. This requires:
- A background migration job (copy embeddings from SQLite to Postgres)
- A runtime backend switch (hot-swap the `VectorBackend`)
- A fallback path (if Postgres is unavailable, fall back to sqlite-vec)

Defer until the static configuration path is proven in production.

### W3: Matryoshka pipeline integration

If the interim mitigation is needed before the Postgres migration, integrate
`vec_slice` into the embedding store path:
- Store full 1024-dim vectors in `embeddings.vector` (for exact retrieval)
- Store 512-dim sliced vectors in `vec_embeddings` (for fast KNN)
- This requires a separate dim for vec0 vs the stored vector

This is a medium-complexity change. Defer unless pods are in the 50k-100k
range and Postgres is not yet deployed.

## Consequences

### Positive
- **No rewrite needed:** the `VectorBackend` architecture already supports
  the switch — it's a configuration change, not a code change
- **Sovereignty preserved:** h_mems stay in SQLite/SQLCipher; embeddings are
  a recomputable cache
- **Clear threshold:** 100k vectors is well-grounded in the sqlite-vec
  author's own benchmarks
- **Interim mitigation available:** Matryoshma dim reduction buys time

### Negative
- **Operational complexity:** pods above 100k need a Postgres instance,
  which is infrastructure (connection string, auth, backups)
- **HNSW index not yet created:** the pgvector path currently uses exact
  scan, which doesn't solve the scaling problem until the HNSW index is added
- **Two-backend divergence:** the SQLite and Postgres paths have different
  SQL, different performance profiles, and different recall characteristics

### Risks
- **Threshold too conservative:** if the curator generates embeddings faster
  than expected, a pod could blow past 100k before Postgres is configured.
  Mitigation: monitor `EmbeddingStore::count()` and alert before the threshold.
- **Postgres not available in all deployment contexts:** local-only / air-gapped
  pods can't use Postgres. Mitigation: sqlite-vec remains the default; Postgres
  is opt-in.

## References

- [sqlite-vec v0.1.0 benchmarks](https://alexgarcia.xyz/blog/2024/sqlite-vec-stable-release/index.html) — 100k vectors, various dims
- [sqlite-vec#186](https://github.com/asg017/sqlite-vec/issues/186) — 250k × 1024-dim "very slow"
- [sqlite-vec#25](https://github.com/asg017/sqlite-vec/issues/25) — ANN index tracking
- [pgvector HNSW benchmarks (AWS)](https://aws.amazon.com/blogs/database/accelerate-hnsw-indexing-and-searching-with-pgvector-on-amazon-aurora-postgresql-compatible-edition-and-amazon-rds-for-postgresql/) — 1.5ms at scale
- [ParadeDB pgvector tuning](https://www.paradedb.com/learn/postgresql/tuning-pgvector) — HNSW vs IVFFlat, memory sizing
- [Instaclustr pgvector benchmarks](https://www.instaclustr.com/education/vector-database/pgvector-performance-benchmark-results-and-5-ways-to-boost-performance/) — scaling characteristics
- [DEV.to: SQLite as a vector database](https://dev.to/zoricic/sqlite-as-a-vector-database-yes-really-4cm4) — "you don't need ANN until well past 100k"
- ADR-050: Ontology-Anchored Embedding
- ADR-051: SQLite + sqlite-vec Storage and Recall Optimization