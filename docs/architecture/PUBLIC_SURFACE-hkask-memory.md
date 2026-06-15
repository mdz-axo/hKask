---
title: "Public Surface Justification — hkask-memory"
audience: [architects, developers]
last_updated: 2026-06-15
version: "0.27.0"
status: "Active"
domain: "Technology"
mds_categories: [composition]
---

# Public Surface Justification — hkask-memory

**Crate:** `hkask-memory`  
**Public items in lib.rs:** 14  
**Deep-module threshold:** ≤7 public functions (Ousterhout)

## Why This Surface Is Large

`hkask-memory` is the **memory pipeline crate** — semantic and episodic memory encoding, consolidation, and retrieval. Its surface is large because it implements dual-stream memory:

1. **Episodic memory** — Experience recording, narrative generation, session log management.
2. **Semantic memory** — Triple extraction, embedding, KNN retrieval, knowledge graph construction.
3. **Consolidation** — Episodic → Semantic bridge with perspective stripping.
4. **MemoryPort** — Trait for pluggable memory backends.

## Mitigations

- **Dual-stream separation:** Episodic and semantic pipelines are separate modules.
- **Trait-based backend:** `MemoryPort` enables testability and future backend swaps.

## Deletion Test

Delete `hkask-memory` and experience recording, triple extraction, embedding pipelines, and consolidation reappear scattered across agent pods and MCP servers. The crate earns its existence.
