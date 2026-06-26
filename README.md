<p align="center">
  <img src="assets/kask-logo.svg" alt="Kask Logo" width="120"/>
</p>
# ℏKask - A Minimal Viable Container for Agents

**Version:** v0.30.0 | 

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
| 2 | **Essential Tools** | 14 MCP servers + Inference Router (DeepInfra, Together AI, fal.ai, OpenRouter) |
| 3 | **User Sovereignty** | OCAP, SQLCipher, keystore, private/public gating |
| 4 | **CNS** | `cns.*` spans, variety counters, algedonic alerts |
| 5 | **Composition** | Unified registry with template_type discriminator, 47 composable skills, 299 Jinja2 templates |

---

## Skills & Composition

hKask's behavioral surface is defined by **47 skills** — composable agent instructions stored as YAML manifests with Jinja2 templates. Skills are not code. They are declarative, user-editable, and versioned in a unified registry.

| Layer | Format | Count | Purpose |
|-------|--------|-------|--------|
| **Skill manifests** | `manifest.yaml` | 46 registry crates | Skill metadata, contracts, constraints |
| **Templates** | `*.j2` (Jinja2) | 239 | Executable process steps (KnowAct, KnowCheck, etc.) |
| **Skills** | `.agents/skills/` | 46 | Categorized: coding, reasoning, kata, meta, specialized |

Skills execute through the `kask chat` runtime or via the QA pipeline (`kask qa triage`, `kask qa run-script`). The skill system includes discovery, bundling, translation, lifecycle management, and adversarial logic auditing.

---

## Crate Structure

### Foundation (12 crates)
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
| `hkask-capability` | OCAP delegation tokens |
| `hkask-ports` | Hexagonal port traits |

### Infrastructure (8 crates)
| Crate | Purpose |
|-------|--------|
| `hkask-inference` | Inference router (provider dispatch, model selection) |
| `hkask-communication` | Matrix transport, agent registry, 7R7 listener |
| `hkask-improv` | Constructive interaction protocol (plussing, yes-and, yes-but, freestyling, riffing) |
| `hkask-condenser` | Context condensation engine (7 tools, 51 tests) |
| `hkask-acp` | Agent Communication Protocol |
| `hkask-adapter` | External provider adapters (Hugging Face, etc.) |
| `hkask-test-harness` | Test infrastructure (TestDb, TestWebId, mocks, strategies) |
| `hkask-mcp-cloud-gateway` | Cloud MCP gateway for remote tool dispatch |

### Services (11 crates)
| Crate | Purpose |
|-------|--------|
| `hkask-services` | Facade: all service ports via dependency inversion |
| `hkask-services-core` | Core service traits and port definitions |
| `hkask-services-context` | AgentService context, CNS runtime, cybernetic loops |
| `hkask-services-runtime` | Runtime orchestration (lifecycle, daemon, events) |
| `hkask-services-backup` | Backup policy layer on Git CAS |
| `hkask-services-corpus` | Document corpus management and indexing |
| `hkask-services-kanban` | Kanban board coordination |
| `hkask-services-kata` | Toyota Kata coaching/improvement loops |
| `hkask-services-onboarding` | First-run and user onboarding |
| `hkask-services-skill` | Skill registry and discovery |
| `hkask-services-wallet` | Crypto wallet and chain port selection |

### Wallet, Identity & Ledger (3 crates)
| Crate | Purpose |
|-------|--------|
| `hkask-wallet` | Multi-chain wallet (Hedera, Hinkal optional) |
| `hkask-wallet-types` | Wallet value types and data structures |
| `hkask-ledger` | Triple-entry accounting ledger |

### MCP Servers (12 crates)
- `hkask-mcp-condenser` — Context condensation (thin wrapper around hkask-condenser)
- `hkask-mcp-research` — Web search, extraction, and feed-based research
- `hkask-mcp-skill` — Skill registry and discovery MCP interface
- `hkask-mcp-curator` — Curator daemon tools (algedonic log, escalations, memory recall, semantic search)
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
| **Foundation LOC** | ~82,600 |
| **Infrastructure LOC** | ~18,100 |
| **Services LOC** | ~27,500 |
| **Wallet & Identity LOC** | ~5,300 |
| **Core Total (src/)** | ~133,500 |
| **MCP Server LOC (src/)** | ~35,700 |
| **Total LOC** | ~191,700 |
| **Core Crates** | 34 (12 foundation + 8 infra + 11 services + 3 wallet/identity) |
| **MCP Servers** | 13 |
| **Tests** | 1,460 (workspace) |
| **CLI Subcommands** | 39 |
| **API Route Groups** | 26 |
| **Build/Clippy/Fmt/Test** | All passing |
| **Skills** | 46 (75 registry crates, 297 Jinja2 templates) |
| **QA Pipeline** | Fuzz triage, mutation analysis, autonomous script runner |

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
- `docs/architecture/reference/hKask-Curator-persona.md` — Curator persona specification
- `docs/status/PROJECT_STATUS.md` — Project status (single source of truth)
- `assets/LOGO-DESIGN-PRINCIPLES.md` — Logo design principles
- `AGENTS.md` — Agent operating guide

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
*CI green. 34 crates. 13 MCP servers. 47 skills.*
