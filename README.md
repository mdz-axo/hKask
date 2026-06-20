<p align="center">
  <img src="assets/kask-logo.svg" alt="Kask Logo" width="120"/>
</p>
# ℏKask - A Minimal Viable Container for Agents

**Version:** v0.30.0 | **Status:** Phase 8 complete — skill system, QA, condenser, CI green, 39 CLI commands + 26 API route groups

---

## Logo & Brand

The Kask logo synthesizes four elements into a single mark:

| Element | Represents | Visual Form |
|---------|-----------|-------------|
| **The Kask (Container)** | Typed container, governed surface | Rectangular amphora with handles |
| **Calligraphy (Art)** | Human craft, temporal mark | Varied stroke width, pressure-sensitive |
| **Curator's Eye (Vision)** | Observation, governance, loyalty | Almond eye with iris, pupil, reflection |
| **Bitemporal Shadow (Perspective)** | Valid-time + transaction-time | Offset shadow, reduced opacity |

> *A simple container, drawn by hand, watching from within, remembering in two times.*

**Core principles:** Recognition <400ms | Scalable 16px–16ft | Monochrome-first | No gradients/effects

Full design principles → [`assets/LOGO-DESIGN-PRINCIPLES.md`](assets/LOGO-DESIGN-PRINCIPLES.md)

---

## Vision

hKask is the minimal viable unit of an agent platform from which a full agent ecosystem can be reconstructed.

**Design Philosophy:** Austere and efficient recombinatorial system. Rust is the loom (fixed logic). YAML/Jinja2 is the thread (mutable content).

---

## Five Anchors

| # | Anchor | Implementation |
|---|--------|----------------|
| 1 | **Agent Enablement** | Bots + Replicants in pods with WebID, A2A, Episodic and Semantic Memory and kask services |
| 2 | **Essential Tools** | 11 MCP servers + Inference Router (DeepInfra, Together AI, fal.ai, OpenRouter) |
| 3 | **User Sovereignty** | OCAP, SQLCipher, keystore, private/public gating |
| 4 | **CNS** | `cns.*` spans, variety counters, algedonic alerts |
| 5 | **Composition** | Unified registry with template_type discriminator, 45 composable skills, 232 Jinja2 templates |

---

## Skills & Composition

hKask's behavioral surface is defined by **45 skills** — composable agent instructions stored as YAML manifests with Jinja2 templates. Skills are not code. They are declarative, user-editable, and versioned in a unified registry.

| Layer | Format | Count | Purpose |
|-------|--------|-------|--------|
| **Skill manifests** | `manifest.yaml` | 72 registry crates | Skill metadata, contracts, constraints |
| **Templates** | `*.j2` (Jinja2) | 232 | Executable process steps (KnowAct, KnowCheck, etc.) |
| **Skills** | `.agents/skills/` | 45 | Categorized: coding, reasoning, kata, meta, specialized |

Skills execute through the `kask chat` runtime or via the QA pipeline (`kask qa triage`, `kask qa run-script`). The skill system includes discovery, bundling, translation, lifecycle management, and adversarial logic auditing.

---

## Crate Structure

### Foundation (10 crates)
| Crate | Purpose |
|-------|--------|
| `hkask-types` | ID types, nu-event, vocabulary, visibility, CNS spans |
| `hkask-storage` | SQLite + SQLCipher, triples, embeddings, blobs, Git CAS |
| `hkask-memory` | Semantic/episodic pipelines (consolidation: episodic → semantic) |
| `hkask-cns` | Cybernetic Nervous System |
| `hkask-templates` | Registry, vocabulary, cascade, resolver |
| `hkask-agents` | Pods, ACP, bot/replicant, Curator |
| `hkask-keystore` | OS keychain, AES-256-GCM |
| `hkask-mcp` | MCP runtime, dispatch, security |
| `hkask-cli` | CLI (39 subcommands + REPL) |
| `hkask-api` | HTTP API, utoipa OpenAPI (26 route groups) |

### Infrastructure (7 crates)
| Crate | Purpose |
|-------|--------|
| `hkask-inference` | Inference router (provider dispatch, model selection) |
| `hkask-communication` | Matrix transport, agent registry, 7R7 listener |
| `hkask-improv` | Constructive interaction protocol (plussing, yes-and, yes-but, freestyling, riffing) |
| `hkask-condenser` | Context condensation engine (7 tools, 51 tests) |
| `hkask-acp` | Agent Communication Protocol |
| `hkask-adapter` | External provider adapters (Hugging Face, etc.) |
| `hkask-test-harness` | Test infrastructure (TestDb, TestWebId, mocks, strategies) |

### Services (17 crates)
| Crate | Purpose |
|-------|--------|
| `hkask-services` | Facade: all service ports via dependency inversion |
| `hkask-services-core` | Core service traits and port definitions |
| `hkask-services-backup` | Backup policy layer on Git CAS |
| `hkask-services-classify` | Content classification |
| `hkask-services-context` | Context window management |
| `hkask-services-daemon` | Background daemon services |
| `hkask-services-discover` | Content discovery and search |
| `hkask-services-embed` | Embedding generation and storage |
| `hkask-services-inference-svc` | Inference service orchestration |
| `hkask-services-kanban` | Kanban board coordination |
| `hkask-services-kata` | Toyota Kata coaching/improvement loops |
| `hkask-services-lifecycle` | Agent lifecycle management |
| `hkask-services-onboarding` | First-run and user onboarding |
| `hkask-services-skill` | Skill registry and discovery |
| `hkask-services-sovereignty` | Magna Carta enforcement |
| `hkask-services-verification` | Capability verification |
| `hkask-services-wallet` | Crypto wallet and chain port selection |

### Wallet & Identity (1 crate)
| Crate | Purpose |
|-------|--------|
| `hkask-wallet` | Multi-chain wallet (Solana, Hedera, Hinkal optional) |

### MCP Servers (11 crates)
- `hkask-mcp-condenser` — Context condensation (thin wrapper around hkask-condenser)
- `hkask-mcp-research` — Web search, extraction, and feed-based research
- `hkask-mcp-spec` — Specification authoring, curation, and validation
- `hkask-mcp-companies` — Company financial data (FMP + EODHD dual-provider)
- `hkask-mcp-communication` — Thin MCP wrapper over core communication crate
- `hkask-mcp-media` — Media generation (image, video, audio, 3D via fal.ai and other providers)
- `hkask-mcp-replica` — Authorial style embedding and prose composition
- `hkask-mcp-docproc` — Unified document processing (format conversion, OCR, chunking, QA generation)
- `hkask-mcp-memory` — Unified episodic + semantic memory with cloud backup
- `hkask-mcp-training` — Model training (QA pairs and training data for fine-tuning pipelines)
- `hkask-mcp-kanban` — Kanban board coordination

---

## Current Metrics

| Metric | Value |
|--------|-------|
| **Foundation LOC** | ~80,000 |
| **Infrastructure LOC** | ~16,000 |
| **Services LOC** | ~21,000 |
| **Wallet LOC** | ~6,800 |
| **Core Total (src/)** | ~124,000 |
| **MCP Server LOC (src/)** | ~34,500 |
| **Test Files** | 144 (with `#[cfg(test)]` modules) |
| **Core Crates** | 35 (10 foundation + 7 infra + 17 services + 1 wallet) |
| **MCP Servers** | 11 |
| **CLI Subcommands** | 39 |
| **API Route Groups** | 26 |
| **Build/Clippy/Fmt/Test** | All passing |
| **Skills** | 45 (72 registry crates, 232 Jinja2 templates) |
| **QA Pipeline** | Fuzz triage, mutation analysis, autonomous script runner |

---

## Implementation Roadmap

### Phase 0: Workspace Skeleton ✓
- [x] Virtual workspace at root
- [x] `[workspace.dependencies]` with pinned versions
- [x] Empty crate stubs for all 35 crates
- [x] CI verification: `cargo check`, `test`, `clippy`, `fmt`

### Phase 1: Security Foundation ✓
- [x] `hkask-keystore` — encrypted KV, interactive passphrase
- [x] `hkask-types` — ID types, nu-event, vocabulary enum
- [x] `hkask-storage` — SQLite + SQLCipher + sqlite-vec + BLAKE3 + gix
- [x] `hkask-memory` — semantic/episodic pipelines

### Phase 2: Bot System & A2A ✓
- [x] `hkask-agents` — pod lifecycle, ACP, bot/replicant, OCAP
- [x] `hkask-keystore` — OS keychain, AES-256-GCM

### Phase 3: Templates & Registry ✓
- [x] `hkask-templates` — registry, vocabulary, minijinja, cascade

### Phase 4: Security Hardening & Testing ✓
- [x] Comprehensive security hardening (ADR-022)
- [x] Test coverage across core crates

### Phase 5: CNS & Improv Integration ✓
- [x] `hkask-cns` — outcome ingestion, `cns.*` span emission, variety counters
- [x] `hkask-improv` — multi-agent interaction protocol (plussing, yes-and, yes-but, freestyling, riffing)

### Phase 6: CLI/API Commands ✓
- [x] `hkask-mcp` — MCP runtime, dispatch, security
- [x] `hkask-api` — axum + utoipa, 26 route groups
- [x] `hkask-cli` — 39 subcommand groups + REPL with `/model` slash command

### Phase 7: Documentation Refresh ✓
- [x] DDMVSS-aligned architecture documentation (9/9 categories)
- [x] 56 active documents curated, stale archives pruned

### Phase 8: Skill System ✓
- [x] 45 skills across the corpus (coding, reasoning, kata, meta, specialized)
- [x] 72 registry crates (manifest.yaml + Jinja2 .j2 templates)
- [x] 232 Jinja2 templates — composable, declarative, user-editable
- [x] Skill discovery, bundler, translator, manager, logic auditor
- [x] QA system — fuzz triage, mutation analysis, autonomous script runner

### In Progress
- [x] Context condensation in condenser MCP server (7 tools, 51 tests)
- [ ] Service-layer refactor (strangler fig: CLI/API/MCP → shared services)
- [ ] Seed templates (prompt/process/cognition)

### Upcoming
- [ ] Curator instantiation
- [ ] Success criterion test (17 items from master spec)

---

## Success Criterion (17 Items)

hKask is "done" when a single user can:

1. Run `kask`, get prompted for passphrase, observe Curator pod start
2. Open `kask chat` and converse with Curator (episodic memory recorded)
3. Use `/model qwen` to fuzzy search models; `/model qwen3:8b` to switch the LLM
4. Observe ≥3 subsystem-curator bots spawn at startup
5. Trigger improv session with ≥2 subsystem-curators deliberating
6. Invoke any operation through CLI or HTTP API with identical behavior
7. Invoke any tool from 11 MCP set; observe routing
8. Compose two tools via process template
9. Record episodic memory with confidence
10. Retrieve memory; observe `as-of` query returns historical state
11. Observe another agent cannot read private memory without OCAP delegation
12. Generate embedding via embedding MCP; stored in same SQLite transaction
13. `fork` public template via storage MCP; observe divergent branch
14. Merge two branches; observe structural success + conflict requiring improv resolution
15. Attempt to clone private artifact; observe OCAP rejection
16. Observe curator reflect on inference outcomes, propose template revision
17. CNS records change, observes new outcomes

---

## Commands

```bash
# Verification
cargo check --workspace
cargo test --workspace
cargo clippy --workspace --all-targets -- -D warnings
cargo fmt --check
```

---

## Documentation

- `docs/architecture/hKask-architecture-master.md` — Architecture index
- `docs/architecture/core/PRINCIPLES.md` — Magna Carta principles (P1–P12)
- `docs/architecture/reference/hKask-erd.md` — Entity relationship diagrams
- `docs/architecture/interface-and-composition.md` — Registry & templating design
- `docs/status/PROJECT_STATUS.md` — Project status (single source of truth)
- `assets/LOGO-DESIGN-PRINCIPLES.md` — Logo design principles
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
- UCAN for hKask (OCAP-only)
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

*ℏKask - A Minimal Viable Container for Agents — v0.30.0*
*Rust is the loom. YAML/Jinja2 is the thread.*
*CI green. 39 commands. 35 crates. 11 MCP servers. 45 skills.*
