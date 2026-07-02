<p align="center">
  <img src="assets/kask-logo.svg" alt="Kask Logo" width="120"/>
</p>
# ℏKask - A Minimal Viable Container for Replicants

**Version:** v0.31.0 |

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
| 5 | **Composition** | Templates + manifests compose into iterative PDCA Skill loops (39 skills, 273 templates, 64 manifests) |

---

## Skills & Composition

hKask distinguishes two layers that other systems conflate:

- **Templates** (`.j2` Jinja2 files, 273 total) — One-shot prompt executions. These are what Claude, ChatGPT, and most agent platforms call "skills." In hKask, they are raw material: a template runs once, returns output, and exits.
- **Skills** (39 total) — Iterative PDCA (Plan-Do-Check-Act) loops that compose multiple templates into autonomous search, learning, and implementation cycles. A Skill has a FlowDef manifest with `convergence.threshold > 0`, a `gas.cap`, and a `loop` action. It runs until it converges on a quality threshold, exhausts its energy budget, or escalates to the Curator.

Where other systems give you a prompt, hKask gives you a process.

| Layer | Format | Count | Behavior |
|-------|--------|-------|----------|
| **Templates** | `*.j2` (Jinja2) | 273 | One-shot: execute → return output |
| **Skill manifests** | `manifest.yaml` | 64 | FlowDef: contracts, convergence criteria, gas budget |
| **Skills** | `.agents/skills/` | 39 | PDCA loops: compose templates → iterate → converge \| max_out \| escalate |

Skills execute through the `kask chat` runtime or via the QA pipeline (`kask qa triage`, `kask qa run-script`). The skill system includes discovery, bundling, translation, lifecycle management, and adversarial logic auditing. A Bundle composes multiple Skills but is not itself a PDCA loop.

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

### Infrastructure (9 crates)
| Crate | Purpose |
|-------|--------|
| `hkask-inference` | Inference router (provider dispatch, model selection) |
| `hkask-communication` | Matrix transport, agent registry, 7R7 listener |
| `hkask-improv` | Constructive interaction protocol (plussing, yes-and, yes-but, freestyling, riffing) |
| `hkask-condenser` | Context condensation engine (7 tools, 51 tests) |
| `hkask-acp` | Agent Communication Protocol |
| `hkask-adapter` | External provider adapters (Hugging Face, etc.) |
| `hkask-fal` | Fal.ai API client library (workflow DAG execution, model dispatch) |
| `hkask-test-harness` | Test infrastructure (TestDb, TestWebId, mocks, strategies) |
| `hkask-mcp-cloud-gateway` | Cloud MCP gateway for remote tool dispatch |

### Services (11 crates)
| Crate | Purpose |
|-------|--------|
| `hkask-services` | Facade: all service ports via dependency inversion |
| `hkask-services-core` | Core service traits and port definitions |
| `hkask-services-context` | AgentService context, CNS runtime, cybernetic loops |
| `hkask-services-runtime` | Runtime orchestration (lifecycle, daemon, events) |
| `hkask-services-chat` | Chat session management and history |
| `hkask-services-compose` | Skill composition and bundle orchestration |
| `hkask-services-curator` | Curator daemon metacognition and escalation |
| `hkask-services-corpus` | Document corpus management and indexing |
| `hkask-services-kata-kanban` | Toyota Kata coaching/improvement + Kanban board coordination |
| `hkask-services-onboarding` | First-run and user onboarding |
| `hkask-services-skill` | Skill registry and discovery |
| `hkask-services-wallet` | Crypto wallet and chain port selection |

### Wallet, Identity & Ledger (3 crates)
| Crate | Purpose |
|-------|--------|
| `hkask-wallet` | Multi-chain wallet (Hedera, Hinkal optional) |
| `hkask-wallet-types` | Wallet value types and data structures |
| `hkask-ledger` | Triple-entry accounting ledger |

### Ontology & Interface (4 crates)
| Crate | Purpose |
|-------|--------|
| `hkask-bridge-dublincore` | Dublin Core ontology bridge (metadata interoperability) |
| `hkask-bridge-pko` | Process Knowledge Ontology bridge (workflow semantics) |
| `hkask-federation` | Cross-instance agent federation protocol |
| `hkask-tui` | Terminal UI (ratatui-based interactive console) |

### MCP Servers (14 crates)
- `hkask-mcp-fal` — Fal workflow execution (Strategy D PDCA loops)
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
- `hkask-mcp-kata-kanban` — Kata-Kanban workflow coordination
- `hkask-mcp-filesystem` — Secure filesystem operations with path allowlisting

---

## Current Metrics

| Metric | Value |
|--------|-------|
| **Foundation LOC** | ~86,100 |
| **Infrastructure LOC** | ~19,200 |
| **Services LOC** | ~23,600 |
| **Wallet & Identity LOC** | ~5,500 |
| **Ontology & Interface LOC** | ~13,100 |
| **Core Total (src/)** | ~147,500 |
| **MCP Server LOC (src/)** | ~41,300 |
| **Total LOC** | ~188,800 |
| **Core Crates** | 39 (12 foundation + 9 infra + 11 services + 3 wallet/identity/ledger + 4 ontology/interface) |
| **MCP Servers** | 14 |
| **Tests** | ~860 (864 test binary targets) |
| **CLI Subcommands** | 33 |
| **API Route Groups** | 26 |
| **Build/Clippy/Fmt/Test/UnusedDeps** | All passing |
| **Skills** | 39 (64 registry manifests, 273 Jinja2 templates) |
| **QA Pipeline** | Fuzz triage, mutation analysis, autonomous script runner |

---

## Commands

```bash
# Verification
cargo check --workspace
cargo test --workspace
cargo clippy --workspace --all-targets -- -D warnings
cargo fmt --check

# Dependency hygiene (nightly)
RUSTFLAGS="-D unused_crate_dependencies" cargo +nightly check --workspace
```

---

## Documentation

- `AGENTS.md` — Agent operating guide (capability catalog, tooling policy, prohibitions)
- `.github/workflows/ci.yml` — CI pipeline (fmt → clippy + unused-deps + build → test + doc → invariants)
- `.github/workflows/audit.yml` — Weekly dependency audit (cargo-deny + cargo-audit)
- `assets/LOGO-DESIGN-PRINCIPLES.md` — Logo design principles
- `OPEN_QUESTIONS.md` — Open design questions and tradeoffs

> **Note:** Architecture documentation (`docs/architecture/`) is under reconstruction. The authoritative source for principles and invariants is the CI pipeline (`ci.yml` invariants job) and crate-level doc comments.

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

*ℏKask - A Minimal Viable Container for Replicants — v0.31.0*
*Rust is the loom. YAML/Jinja2 is the thread.*
*CI green. 39 crates. 14 MCP servers. 39 PDCA skill loops. 273 templates.*
