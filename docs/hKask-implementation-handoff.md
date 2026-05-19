# hKask Implementation — Agent Handoff Prompt

## Context

You are beginning implementation of **hKask** (ℏKask — "Planck's Constant of Agent Systems"), a minimal agent-native container platform in Rust.

**Architecture Status:** Complete (v2.2)  
**Line Budget:** ≤30,000 lines Rust (excluding ACP/MCP protocols, Okapi)  
**Crate Structure:** 21 crates (11 core + 10 MCP servers)

---

## Vision: The Quantum of Agent Systems

hKask is named after **Planck's constant** (ℏ) — the smallest possible unit of action from which all quantum phenomena emerge. Similarly, hKask is the minimal viable unit of an agent platform from which a full agent ecosystem can be reconstructed.

**Design Philosophy:** Austere and efficient recombinatorial system. The kernel (non-protocol code) is intentionally minimal. Complexity is offloaded to LLMs, tools, and templates — so the platform improves as those external systems improve.

**Core Insight:** Templates are the wiring surface. There is no wiring as a kernel concept. Templates A2A-talk between agents. The kernel does not orchestrate.

---

## Five Anchors (Non-Negotiable)

| # | Anchor | Purpose | Why It Matters |
|---|--------|---------|----------------|
| 1 | **Agent Enablement** | Sovereign agents (Bot/Replicant) in pods with WebID, ACP | Without agents, there is no agent platform. Bots and replicants are the atoms of hKask. |
| 2 | **Essential Tools** | 10 MCP servers + Okapi inference | Tools are capabilities exposed through MCP. No MCP = no capabilities. |
| 3 | **User Sovereignty** | OCAP, SQLCipher encryption, private/public gating | Without sovereignty, there is no trust. Users own their data, their agents, their choices. |
| 4 | **CNS (Cybernetic Nervous System)** | `cns.*` spans, variety counters, algedonic alerts | Without cybernetics, there is no learning. CNS observes; templates analyze; curators improve. |
| 5 | **Composition Registries** | 3 registries (Prompts, Processes, Cognition) with hLexicon | Without composition, there is no emergence. Templates combine to produce capabilities greater than their parts. |

**These five anchors are the foundation.** If any anchor is missing, the system is not hKask.

---

## Source Documents

All architecture documents are located at: `~/Clones/hKask/docs/architecture/`

**Active Specifications (5):**
1. `hKask-architecture-master.md` — **Sole authoritative spec** (v2.2)
2. `hKask-architecture-index.md` — Index + implementation checklist
3. `hKask-hLexicon.md` — Canonical vocabulary (≤75 terms)
4. `hKask-Curator-persona.md` — Curator persona specification
5. `AGENTS.md` — Agent operating guide

**Reference Documents (3):**
- `vKask-erd.md` — vKask entity relationships
- `vKask-cybernetic-constant.md` — νKask cybernetic foundation
- `MODEL_CATALOG.md` — Model catalog

---

## Key Architectural Decisions

| Decision | Value |
|----------|-------|
| **Cybernetic System** | CNS (Cybernetic Nervous System), `cns.*` namespace |
| **Storage Backend** | SQLite + SQLCipher + sqlite-vec |
| **Vector Contingency** | Qdrant embedded mode (if sqlite-vec fails) |
| **ACP SDK** | acp-runtime (beltxa/acp, Rust-native) |
| **MCP SDK** | rmcp (modelcontextprotocol/rust-sdk) |
| **Template Engine** | minijinja |
| **Encryption** | Interactive passphrase at startup |
| **Capability Delegation** | OCAP-only for h-bar (UCAN deferred) |
| **Condenser Algorithms** | All kask condenser except OpenCode-style + OpenHands-style |
| **Versioning** | Git-only (SHA-based, no SemVer) |
| **Memory Model** | Semantic vs Episodic (categorical, no promotion) |

---

## Crate Structure

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
    ├── Okapi (mdz-axo/Okapi)
    ├── ACP (acp-runtime)
    └── MCP (rmcp)
```

---

## Implementation Roadmap

### Phase 0: Workspace Skeleton (Day 1)
- Create virtual workspace at root
- Set up `[workspace.dependencies]` with pinned versions
- Create empty crate stubs for all 15 crates
- Set up CI: `cargo check`, `test`, `clippy -D warnings`, `fmt --check`, `tokei`

### Phase 1: Foundation (Weeks 1-3)
1. **hkask-keystore** (600 LOC) — encrypted KV, interactive passphrase
2. **hkask-types** (2,000 LOC) — ID types, ν-event, hLexicon enum
3. **hkask-storage** (4,000 LOC) — SQLite + SQLCipher + sqlite-vec + BLAKE3 + gix
4. **hkask-memory** (3,000 LOC) — semantic/episodic pipelines + Bayesian ops

### Phase 2: CNS + Templates (Weeks 4-6)
5. **hkask-cns** (1,200 LOC) — outcome ingestion, `cns.*` span emission
6. **hkask-templates** (5,000 LOC) — registry, hLexicon, minijinja, cascade
7. **hkask-agents** (2,500 LOC) — pod lifecycle, ACP, bot/replicant, OCAP

### Phase 3: Surface (Weeks 7-8)
8. **hkask-mcp** (2,500 LOC) — MCP runtime, dispatch, security
9. **hkask-api** (1,800 LOC) — axum + utoipa, CLI/API parity
10. **hkask-cli** (2,000 LOC) — clap-based CLI from OpenAPI

### Phase 4: MCP Servers (Weeks 9-10)
11. **hkask-inference-mcp** (1,400 LOC) — Okapi + Ollama
12. **hkask-embedding-mcp** (800 LOC) — Ollama embeddings
13. **hkask-ensemble-mcp** (700 LOC) — bot deliberation
14. **hkask-condenser-mcp** (2,200 LOC) — RTK-style + flashrank + reranker + saliency_rank
15. **hkask-memory-mcp** (1,900 LOC) — atop storage MCP + Bayesian ops
16. **hkask-spandrel-mcp** (1,500 LOC) — graph exploration
17. **hkask-docknowledge-mcp** (1,200 LOC) — doc extraction
18. **hkask-web-mcp** (800 LOC) — web search
19. **hkask-scholar-mcp** (800 LOC) — academic search

### Phase 5: Integration + Verification
- Seed templates (prompt/process/cognition)
- Curator instantiation
- Success criterion test (16 items from master spec)
- LOC audit (≤30,000)

---

## Success Criterion (16 Items)

hKask is "done" when a single user can:

1. Run `kask`, get prompted for passphrase, observe Curator pod start
2. Open `kask chat` and converse with Curator (episodic memory recorded)
3. Observe ≥3 subsystem-curator bots spawn at startup
4. Trigger ensemble session with ≥2 subsystem-curators deliberating
5. Invoke any operation through CLI or HTTP API with identical behavior
5. Invoke any tool from 10 MCP set; observe routing
6. Compose two tools via process template (capabilities resolved through MCP)
7. Record episodic memory (kind, role, owner attributed) with confidence
8. Retrieve memory; observe `as-of` query returns historical state
9. Observe another agent cannot read private memory without OCAP delegation
10. Generate embedding via embedding MCP; stored in same SQLite transaction
11. `fork` public template via storage MCP; observe divergent branch via `log` + `diff`
12. Merge two branches; observe structural success + conflict requiring ensemble
13. Attempt to clone private artifact; observe OCAP rejection
14. Observe okapi-curator reflect on inference outcomes, propose template revision
15. CNS records change, observes new outcomes
16. Observe Bayesian combination (§16.5a) when two agents corroborate triple
17. All locally, encrypted at rest (SQLCipher-verified), ≤30,000 LOC

---

## Hallucinations to Avoid

**Do NOT implement:**
- Bot reputation systems
- Bot swarms / consensus mechanisms
- Cross-machine sync
- Bot marketplace
- Curator customization (Curator is fixed)
- SemVer versioning (Git-only)
- Separate feedback crate (CNS handles all)
- Promotion pipeline (episodic/semantic categorical)
- Escalation primitive (Curator loops human in)
- Visibility type system (OCAP-enforced)
- OCT-H currency
- Fine-tuning (axolotl)
- OpenCode-style condenser (deleted)
- OpenHands-style condenser (deleted)
- UCAN for h-bar (OCAP-only, UCAN deferred)

---

## Key Dependencies (Pinned)

```toml
[workspace.dependencies]
tokio = "1.51"  # LTS
axum = "0.8"
utoipa = "5.5"
utoipa-axum = "0.2"
rmcp = "1"  # out of LOC budget
acp-runtime = "0.1"  # out of LOC budget
rusqlite = { version = "0.39", features = ["bundled-sqlcipher-vendored-openssl"] }
sqlite-vec = "0.1"
blake3 = "1"
gix = "0.81"
minijinja = "2"
serde = "1"
serde_json = "1"
thiserror = "2"
anyhow = "1"
tracing = "0.1"
clap = "4"
uuid = "1"
chrono = "0.4"
```

---

## Next Actions (Your First Session)

1. **Review architecture documents:**
   - Read `hKask-architecture-master.md` (sole authoritative spec, v2.2)
   - Read `AGENTS.md` (operating guide, Coco Chanel principles)
   - Read `hKask-hLexicon.md` (canonical vocabulary, ≤75 terms)
   - Read `hKask-Curator-persona.md` (persona specification)

2. **Create workspace skeleton:**
   ```bash
   cd ~/Clones/hKask
   git init
   cargo init --name hkask-workspace
   ```

3. **Set up virtual workspace:**
   - Create `Cargo.toml` with `[workspace]` and `[workspace.dependencies]`
   - Create empty crate stubs for all 11 Stack crates
   - Set up CI: `cargo check`, `test`, `clippy -D warnings`, `fmt --check`, `tokei`

4. **Begin Phase 1:**
   - Start with `hkask-keystore` (smallest, no dependencies, 600 LOC)
   - Then `hkask-types` (foundation for all other crates, 2,000 LOC)
   - Then `hkask-storage` (SQLite + SQLCipher schema, 4,000 LOC)

5. **Remember:**
   - At 30,001 lines, the agent is fired and another tries again
   - No silent draws on reserve
   - No hallucinations
   - As simple as possible, but no simpler

---

## Communication Protocol

- **ACP:** `acp-runtime` (beltxa/acp, Rust-native, v0.1.2)
- **MCP:** `rmcp` (modelcontextprotocol/rust-sdk, v1.6)
- **Okapi:** `mdz-axo/Okapi` (user's inference orchestration layer)

---

## Design Philosophy

**Austere and efficient recombinatorial system** built on ACP, A2A, and MCP protocols. The kernel (non-protocol code) is intentionally minimal.

**Core Innovation:** Bot-mediated subsystems where each capability domain has an expert bot that communicates A2A via self-describing templates — eliminating manual code wiring.

**Templates are the wiring surface.** "There is no wiring as a kernel concept." Templates A2A-talk between agents. The kernel does not orchestrate.

---

## Design Philosophy: As Simple As Possible, But No Simpler

**hKask is a study in restraint.** Every feature, every crate, every line of code must justify its existence.

### Questions to Ask Before Adding Anything

1. **Is this one of the Five Anchors?** If not, what anchor does it serve?
2. **Does this have two consumers?** (P1) If not, is it truly needed?
3. **Is this a stub or placeholder?** (P6, C6) If yes, delete it.
4. **Is this unwired code?** (C2, C3) If yes, does it have a named owner and timeline?
5. **Can this be simpler?** Is there a concrete shape that serves the actual callsite?
6. **Does this belong in the kernel or a template?** Templates evolve; kernel endures.

### The hKask Commitment

- **≤30,000 lines** — Not one line more. At 30,001, the agent is fired and another tries again.
- **No silent draws on reserve** — Every change cited, every line accounted for.
- **No hallucinations** — All features traceable to architecture spec.
- **No speculation** — Code that is not needed today is debt, not investment.
- **No ceremony** — Direct, technical, concise. No preamble, no emoji, no questions.

### When in Doubt

1. Read the architecture spec (`hKask-architecture-master.md`)
2. Ask: "What is the simplest thing that could possibly work?"
3. Implement that. Test it. Ship it.
4. Iterate only when pressure demands it.

---

## Contact

If you encounter ambiguity or need clarification:
1. Check `hKask-architecture-master.md` first (sole authoritative spec)
2. Check `AGENTS.md` (operating guide)
3. Ask the user if the spec is unclear

**Do not hallucinate features.** All features must be traceable to the architecture spec.

---

*ℏKask — Planck's Constant of Agent Systems — v2.2*
*As simple as possible, but no simpler.*
*Begin implementation.*