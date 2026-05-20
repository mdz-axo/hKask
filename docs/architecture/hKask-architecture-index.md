# hKask Architecture — Complete Specification Index

**Master Specification:** `hKask-architecture-master.md` — Consolidated authoritative spec (v0.21.0)

**Total Documents:** 8 (5 active + 3 reference)  
**Design Status:** Pre-alpha — MVP in progress  
**Line Budget:** ≤30,000 lines Rust (excluding ACP/MCP protocols, Okapi)

---

## Active Specifications

| # | Document | Purpose | Status |
|---|----------|---------|--------|
| 1 | hKask-architecture-master.md | Consolidated authoritative specification | **CURRENT — v0.21.0** |
| 2 | hKask-architecture-index.md | This index | **CURRENT** |
| 3 | hKask-hLexicon.md | Minimal composition vocabulary (≤75 terms) | **CURRENT** |
| 4 | hKask-Curator-persona.md | Curator replicant persona specification | **CURRENT** |
| 5 | hKask-erd.md | Entity relationship diagrams (Mermaid) | **CURRENT — v0.21.0** |

---

## Reference Documents

| # | Document | Purpose | Status |
|---|----------|---------|--------|
| 5 | vKask-erd.md | vKask entity relationships | Reference only |
| 6 | vKask-cybernetic-constant.md | νKask cybernetic foundation | Reference only |
| 7 | MODEL_CATALOG.md | Model catalog | Reference only |

---

## Deleted (Superseded)

The following documents were deleted during consolidation:

| Document | Reason |
|----------|--------|
| claude-architecture-hkask.md | Insights incorporated into master spec |
| hKask-storage-spec.md | Wrong backend (Redb vs SQLite + SQLCipher) |
| hKask-memory-spec.md | Wrong terminology (νKask vs CNS) |
| hKask-ensemble.md | Contains removed swarm/consensus code |
| hKask-implementation-roadmap.md | Wrong crate names, outdated |
| hKask-pre-implementation-decisions.md | Superseded by master spec Part III |
| hKask-data-ownership-privacy.md | Superseded by master spec §10 |
| hKask-okapi-integration.md | Superseded by master spec §12 |
| hKask-acp-mcp-integration.md | Superseded by master spec §13 |
| hKask-cli-api-surface.md | Superseded by master spec §11 |
| hKask-agent-taxonomy.md | Superseded by master spec §1.2 |
| hKask-erd.md | Contains removed components |

---

## Architecture Summary

### Five Anchor Capabilities

| Anchor | Implementation | Key Crates |
|--------|----------------|------------|
| **1. Agent Enablement** | Bots + Replicants in agent pods with WebID, ACP | `hkask-agents`, `hkask-ensemble` |
| **2. Essential Tools** | 10 MCP servers + Okapi | `hkask-mcp-*`, Okapi |
| **3. User Sovereignty** | OCAP, encrypted keystore, privacy guarantees | `hkask-keystore`, `hkask-agents` |
| **4. CNS (Cybernetic Nervous System)** | ν-events, variety counters, algedonic alerts | `hkask-cns` |
| **5. Composition Registries** | Templates with hLexicon, self-wiring via ACP/MCP | `hkask-templates` |

### Crate Structure (21 crates total)

```
hkask-workspace/
├── Core (11 crates — ≤30k lines total)
│   ├── hkask-types           # ~2,000 — ID types, ν-event, hLexicon, visibility
│   ├── hkask-storage         # ~4,000 — SQLite + SQLCipher, triples, vectors, blobs, Git CAS
│   ├── hkask-memory          # ~3,000 — Semantic/episodic pipelines (analytic distinction)
│   ├── hkask-cns             # ~2,000 — Cybernetic Nervous System, variety counters, algedonic alerts
│   ├── hkask-templates       # ~5,000 — Registry, hLexicon, cascade, resolver
│   ├── hkask-agents          # ~2,500 — Pods, ACP, bot/replicant, Curator, manifests
│   ├── hkask-ensemble        # ~1,500 — Multi-agent chat (NO swarms, NO consensus)
│   ├── hkask-keystore        # ~1,000 — OS keychain, AES-256-GCM
│   ├── hkask-mcp             # ~2,500 — MCP runtime, dispatch, security
│   ├── hkask-cli             # ~2,000 — CLI commands (bot manifest pull/push, chat)
│   └── hkask-api             # ~2,000 — HTTP API, utoipa OpenAPI
│
├── MCP Servers (10 crates — excluded from budget)
│   ├── hkask-mcp-inference       # Okapi-backed LLM inference
│   ├── hkask-mcp-storage         # Storage operations (triples, embeddings, blobs)
│   ├── hkask-mcp-memory          # Semantic/episodic memory operations
│   ├── hkask-mcp-embedding       # Embedding generation, similarity search
│   ├── hkask-mcp-condenser       # Template condensation, abstraction, summarization
│   ├── hkask-mcp-ensemble        # Multi-agent coordination, chat orchestration
│   ├── hkask-mcp-web             # Web search, scrape, extract
│   ├── hkask-mcp-scholar         # Academic research
│   ├── hkask-mcp-spandrel        # Graph analysis
│   └── hkask-mcp-doc-knowledge   # Document extraction
│
└── External (excluded from budget)
    ├── Okapi (mdz-axo/Okapi) # Inference orchestration
    ├── ACP Protocol          # Agent communication (acp-runtime)
    └── MCP Protocol          # Tool protocol (rmcp)
```

### Key Design Patterns

| Pattern | Description |
|---------|-------------|
| **Bot-Mediated Subsystems** | Each capability has expert bot; A2A + templates replace wiring |
| **Self-Wiring Templates** | Templates declare dependencies; registry resolves at runtime |
| **CNS Cybernetics** | All operations emit ν-events; variety tracking, algedonic alerts (`cns.*` namespace) |
| **Git Artifact Evolution** | Templates forkable; clone/branch/merge/pr semantics |
| **Privacy by Visibility** | Bots = public, Replicant episodic = private, OCAP-gated access |
| **Okapi Dual Interface** | A2A bot (machine) + MCP tool (human) with fallback chains |

---

## Implementation Readiness Checklist

### Pre-Implementation (Complete Before Coding)

- [ ] Review `hKask-architecture-master.md` (authoritative spec v0.21.0)
- [ ] Confirm Okapi integration plan (mdz-axo/Okapi)
- [ ] Set up Git repository structure
- [ ] Configure SQLite + SQLCipher for encryption
- [ ] Define OCAP capability schema (UCAN deferred)
- [ ] Create initial hLexicon term validation

### Phase 1: Foundation (Weeks 1-3)

- [ ] `hkask-types` — ID types, ν-event, hLexicon enum, visibility
- [ ] `hkask-storage` — SQLite schema, bitemporal triples, embeddings, blobs
- [ ] `hkask-memory` — Semantic/episodic pipelines (analytic distinction)

### Phase 2: CNS + Templates (Weeks 4-6)

- [ ] `hkask-cns` — Cybernetic Nervous System, variety counters, algedonic alerts
- [ ] `hkask-templates` — Registry, hLexicon validation, cascade, resolver
- [ ] `hkask-agents` — Pod lifecycle, bot/replicant, OCAP, Curator, manifests

### Phase 3: Surface (Weeks 7-8)

- [ ] `hkask-mcp` — MCP runtime, dispatch, security
- [ ] `hkask-cli` — CLI commands (bot manifest pull/push, chat)
- [ ] `hkask-api` — HTTP API, utoipa OpenAPI
- [ ] `hkask-keystore` — OS keychain, AES-256-GCM

### Phase 4: Integration (Weeks 9-10)

- [ ] `hkask-ensemble` — Multi-agent chat (NO swarms)
- [ ] MCP servers (10 crates: 6 Stack + 4 Arsenal)
- [ ] Okapi integration
- [ ] End-to-end integration tests

---

## Open Questions (All Resolved)

| Question | Decision |
|----------|----------|
| Cybernetic system naming | **CNS** (Cybernetic Nervous System) |
| Template versioning | Git-only (SHA-based) |
| Bot manifest structure | Affirmed (will evolve) |
| Running vs Invoked | Confirmed (lifecycle difference) |
| Manifest editing | CLI/API pull-edit-push workflow |
| Bot reputation | REMOVED (hallucinated) |
| Bot swarms | REMOVED (hallucinated) |
| Cross-machine sync | REMOVED (local-only) |
| Failure recovery | Fail fast (v1.0), checkpoint fallback (future) |
| Human oversight | Explicit request only (v1.0), multi-trigger (future) |
| sqlite-vec contingency | **Qdrant embedded** mode |
| Condenser algorithms | All except OpenCode-style and OpenHands-style |
| ACP SDK | **acp-runtime** (Rust-native) |
| Encryption passphrase | Interactive prompt at startup |
| OCAP vs UCAN | OCAP for h-bar, UCAN deferred to multi-host |

---

## Success Metrics

hKask implementation is successful when:

| Metric | Target | Measurement |
|--------|--------|-------------|
| **Line Budget** | ≤30,000 lines | `cloc` on Rust source (Stack only) |
| **Build Time** | <5 minutes | `cargo build --release` |
| **CLI Response** | <3s (human-facing) | `kask chat` latency |
| **Bot Response** | <100ms (machine-facing) | A2A message latency |
| **Memory Query** | <50ms (semantic), <100ms (episodic) | Recall latency |
| **Template Resolution** | <10ms overhead | Self-wiring overhead |
| **ν-Event Overhead** | <10% of operation time | Telemetry cost |
| **Test Coverage** | >80% | `cargo tarpaulin` |
| **Hallucinations** | 0 | All features traceable to requirements |

---

## Next Actions

1. **Create Implementation Repository**
   ```bash
   mkdir hKask && cd hKask
   git init
   cargo init --name hkask-workspace
   ```

2. **Set Up Crate Structure**
   ```bash
   mkdir -p stack/crates/{hkask-types,hkask-storage,hkask-memory,hkask-cns,hkask-templates,hkask-agents,hkask-ensemble,hkask-keystore,hkask-mcp,hkask-cli,hkask-api}
   mkdir -p mcp-servers/{hkask-mcp-inference,hkask-mcp-storage,hkask-mcp-memory,hkask-mcp-embedding,hkask-mcp-condenser,hkask-mcp-ensemble,hkask-mcp-web,hkask-mcp-scholar,hkask-mcp-spandrel,hkask-mcp-doc-knowledge}
   ```

3. **Begin Phase 1**
   - Start with `hkask-types` (foundation for all other crates)
   - Define ID types, ν-event structure, hLexicon enum
   - Set up SQLite + SQLCipher for `hkask-storage`

4. **Parallel Workstreams**
   - Okapi integration (separate repo, then integrate)
   - ACP/MCP protocol crates (excluded from budget, but required)
   - Arsenal MCP servers (can be developed in parallel)

---

## Document Version History

| Version | Date | Changes |
|---------|------|---------|
| 0.21.0 | 2026-05-18 | Pre-alpha MVP: unified registry, manifest/template distinction, CNS integration, ERD complete |

---

*hKask Architecture Index — 11 Core crates + 10 MCP servers, pre-alpha MVP (v0.21.0)*
