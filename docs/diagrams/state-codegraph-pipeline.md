---
title: "Proposed CodeGraph IndexPipeline Lifecycle — State Diagram"
audience: [developers, agents]
last_updated: 2026-07-10
version: "0.31.0"
status: "Proposed"
domain: "Infrastructure"
mds_categories: [domain, lifecycle]
diagram_type: "state"
verified_against:
  - "crates/hkask-codegraph/src/indexer/pipeline.rs:38-273 (current implementation; proposed state model is not implemented)"
diataxis: reference
---

# Proposed CodeGraph IndexPipeline Lifecycle

> **Status: proposed, not current behavior.** `IndexPipeline` currently exposes `index_file`, `index_directory`, `finalize`, and `staleness_seconds`; it does not hold an explicit lifecycle-state enum, threshold-driven stale transition, snapshot lifecycle, or automatic re-index trigger. The implementation flow is documented in [CodeGraph Indexing Pipeline](flowchart-codegraph-pipeline.md).

This state model remains as a design reference if hKask later introduces an explicit lifecycle controller around the pipeline. It must not be used to describe current operational behavior.

```mermaid
stateDiagram-v2
    [*] --> Uninitialized : CodeGraphServer::new()
    Uninitialized --> Indexing : first ensure_indexed()

    state Indexing {
        [*] --> Walking : index_directory()
        Walking --> Hashing : per file
        Hashing --> SkipFile : hash matches stored
        Hashing --> Parsing : hash changed
        Parsing --> Extracting : tree-sitter CST → symbols + edges
        Extracting --> Inserting : persist symbols and resolved edges
        Inserting --> Walking : next file
        SkipFile --> Walking : next file
        Walking --> Ranking : all files done
        Ranking --> [*] : compute PageRank
        }

        Indexing --> Ready : finalize() — PageRank computed

    state Ready {
        [*] --> Serving : staleness_seconds < threshold
        Serving --> Serving : query/traverse/impact/context
        --
        note right of Serving : CNS span: cns.codegraph.index_health
    }

    Ready --> Stale : staleness_seconds > threshold OR file changed on disk

    state Stale {
        [*] --> AwaitingTrigger : CNS alert emitted
        AwaitingTrigger --> AwaitingTrigger : queries still served from last snapshot
    }

    Stale --> Indexing : codegraph_reindex() called
    Ready --> Indexing : codegraph_reindex() called (explicit)

    Ready --> [*] : CodeGraphServer dropped
    Stale --> [*] : CodeGraphServer dropped

    note left of Uninitialized : Store opened (file or in-memory)<br/>schema initialized (idempotent)
    note right of Indexing : Proposed controller behavior<br/>Current implementation indexes sequentially<br/>and uses BLAKE3 incremental skip
    note right of Ready : Proposed serving state
    note left of Stale : Proposed CNS-driven re-index trigger
```

### Implementation Gap

The following behavior is shown only as a future design target and requires a lifecycle controller to implement it:

- Explicit `Uninitialized`, `Indexing`, `Ready`, and `Stale` states.
- A staleness threshold and CNS-driven re-index trigger.
- A stable snapshot-serving contract while indexing.
- A documented controller boundary that owns state transitions and synchronization.

Current behavior is limited to call-driven indexing. `finalize()` resets the staleness clock, computes PageRank, and emits `cns.codegraph.index_health`; `staleness_seconds()` merely reports the elapsed time for an external caller.

### Related Documentation

- [`erd-codegraph-schema.md`](erd-codegraph-schema.md) — Database schema ERD
- [`flowchart-codegraph-pipeline.md`](flowchart-codegraph-pipeline.md) — Indexing pipeline detail
- [`sequence-codegraph-agent.md`](sequence-codegraph-agent.md) — Agent interaction workflow
- [`../architecture/hKask-architecture-master.md`](../architecture/hKask-architecture-master.md) — Architecture master (CNS feedback loop)
