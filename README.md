# hKask (ℏKask) — Planck's Constant of Agent Systems

**Version:** v0.21.0 (Pre-alpha MVP in progress)

---

## Vision

hKask is the minimal viable unit of an agent platform from which a full agent ecosystem can be reconstructed. Named after Planck's constant (ℏ) — the smallest possible unit of action.

**Design Philosophy:** Austere and efficient recombinatorial system. Rust is the loom (fixed logic). YAML/Jinja2 is the thread (mutable content).

---

## Five Anchors

| # | Anchor | Implementation |
|---|--------|----------------|
| 1 | **Agent Enablement** | Bots + Replicants in pods with WebID, ACP |
| 2 | **Essential Tools** | 16 MCP servers + Okapi |
| 3 | **User Sovereignty** | OCAP, SQLCipher, private/public gating |
| 4 | **CNS** | `cns.*` spans, variety counters, algedonic alerts |
| 5 | **Composition** | Unified registry with template_type discriminator |

---

## Crate Structure

### Core (11 crates)
- `hkask-types` — ID types, ν-event, hLexicon, visibility
- `hkask-storage` — SQLite + SQLCipher, triples, embeddings, blobs, Git CAS
- `hkask-memory` — Semantic/episodic pipelines
- `hkask-cns` — Cybernetic Nervous System
- `hkask-templates` — Registry, hLexicon, cascade, resolver
- `hkask-agents` — Pods, ACP, bot/replicant, Curator
- `hkask-ensemble` — Multi-agent chat
- `hkask-keystore` — OS keychain, AES-256-GCM
- `hkask-mcp` — MCP runtime, dispatch, security
- `hkask-cli` — CLI commands
- `hkask-api` — HTTP API, utoipa OpenAPI

### MCP Servers (16 crates)
- `hkask-mcp-inference` — Okapi-backed LLM inference
- `hkask-mcp-condenser` — General-purpose context reranking and condensation
- `hkask-mcp-web` — Web search, scrape
- `hkask-mcp-scholar` — Academic research
- `hkask-mcp-ocap` — Capability management
- `hkask-mcp-keystore` — Keystore operations
- `hkask-mcp-cns` — CNS operations
- `hkask-mcp-git` — Git CAS
- `hkask-mcp-registry` — Registry operations
- `hkask-mcp-gml` — GML allosteric engine
- `hkask-mcp-spec` — DDMVSS spec capture
- `hkask-mcp-github` — GitHub integration
- `hkask-mcp-fmp` — Financial data (FMP)
- `hkask-mcp-telnyx` — Communications (Telnyx)
- `hkask-mcp-fal` — Media generation (FAL)
- `hkask-mcp-rss-reader` — RSS feed reader

**Converted to templates** (not MCP servers):
- `spandrel` → `templates/spandrel/` (graph analysis)
- `doc-knowledge` → `templates/doc-knowledge/` (document extraction)

---

## Implementation Roadmap

### Phase 0: Workspace Skeleton ✓
- [x] Virtual workspace at root
- [x] `[workspace.dependencies]` with pinned versions
- [x] Empty crate stubs for all 21 crates
- [x] CI verification: `cargo check`, `test`, `clippy`, `fmt`

### Phase 1: Foundation (Weeks 1-3)
- [ ] `hkask-keystore` (600 LOC) — encrypted KV, interactive passphrase
- [ ] `hkask-types` (2,000 LOC) — ID types, ν-event, hLexicon enum
- [ ] `hkask-storage` (4,000 LOC) — SQLite + SQLCipher + sqlite-vec + BLAKE3 + gix
- [ ] `hkask-memory` (3,000 LOC) — semantic/episodic pipelines + Bayesian ops

### Phase 2: CNS + Templates (Weeks 4-6)
- [ ] `hkask-cns` (1,200 LOC) — outcome ingestion, `cns.*` span emission
- [ ] `hkask-templates` (5,000 LOC) — registry, hLexicon, minijinja, cascade
- [ ] `hkask-agents` (2,500 LOC) — pod lifecycle, ACP, bot/replicant, OCAP

### Phase 3: Surface (Weeks 7-8)
- [ ] `hkask-mcp` (2,500 LOC) — MCP runtime, dispatch, security
- [ ] `hkask-api` (1,800 LOC) — axum + utoipa, CLI/API parity
- [ ] `hkask-cli` (2,000 LOC) — clap-based CLI from OpenAPI

### Phase 4: MCP Servers (Weeks 9-10)
- [ ] All 10 MCP servers implemented

### Phase 5: Integration + Verification
- [ ] Seed templates (prompt/process/cognition)
- [ ] Curator instantiation
- [ ] Success criterion test (16 items from master spec)

---

## Success Criterion (16 Items)

hKask is "done" when a single user can:

1. Run `kask`, get prompted for passphrase, observe Curator pod start
2. Open `kask chat` and converse with Curator (episodic memory recorded)
3. Observe ≥3 subsystem-curator bots spawn at startup
4. Trigger ensemble session with ≥2 subsystem-curators deliberating
5. Invoke any operation through CLI or HTTP API with identical behavior
6. Invoke any tool from 16 MCP set; observe routing
7. Compose two tools via process template
8. Record episodic memory with confidence
9. Retrieve memory; observe `as-of` query returns historical state
10. Observe another agent cannot read private memory without OCAP delegation
11. Generate embedding via embedding MCP; stored in same SQLite transaction
12. `fork` public template via storage MCP; observe divergent branch
13. Merge two branches; observe structural success + conflict requiring ensemble
14. Attempt to clone private artifact; observe OCAP rejection
15. Observe okapi-curator reflect on inference outcomes, propose template revision
16. CNS records change, observes new outcomes

---

## Commands

```bash
# Verification
cargo check
cargo test
cargo clippy -- -D warnings
cargo fmt --check
```

---

## Documentation

- `docs/architecture/hKask-architecture-master.md` — Sole authoritative spec (v0.21.0)
- `docs/architecture/hKask-erd.md` — Entity relationship diagrams
- `docs/architecture/registry-templating-prompt-v2.md` — Registry & templating design
- `AGENTS.md` — Agent operating guide

---

## Hallucinations to Avoid

**Do NOT implement:**
- Bot reputation systems
- Bot swarms / consensus mechanisms
- Cross-machine sync
- Bot marketplace
- Curator customization
- SemVer versioning (Git-only)
- Separate feedback crate (CNS handles all)
- Promotion pipeline (episodic/semantic categorical)
- Escalation primitive
- Visibility type system (OCAP-enforced)
- OCT-H currency
- Fine-tuning (axolotl)
- OpenCode-style condenser
- OpenHands-style condenser
- UCAN for h-bar (OCAP-only)
- Three separate registries (unified with `template_type` discriminator)
- Rust-based template selection (selection intelligence in Jinja2/LLM)

---

## Design Philosophy

**As simple as possible, but no simpler.**

- **No silent draws on reserve** — Every change cited
- **No hallucinations** — All features traceable to spec
- **No speculation** — Code not needed today is debt
- **No ceremony** — Direct, technical, concise

**The Loom and the Thread:**

| Layer | Technology | Mutability |
|-------|------------|------------|
| **Hard (Kernel)** | Rust | Fixed, stable |
| **Soft (Material)** | YAML, Jinja2, MD | Mutable, evolving |

---

*ℏKask — Planck's Constant of Agent Systems — v0.21.0*  
*Rust is the loom. YAML/Jinja2 is the thread.*  
*MVP in progress.*
