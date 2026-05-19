# hKask Architecture — Master Specification v0.21.0

**Date:** 2026-05-18  
**Status:** Pre-alpha MVP in progress  
**Line Budget:** ≤30,000 lines Rust (excluding ACP/MCP protocols, Okapi)

---

## Executive Summary

hKask (ℏKask — "Planck's Constant of Agent Systems") is a **minimal agent-native container platform** built on cybernetic first principles. It enables sovereign agents (bots and replicants) to communicate, compose capabilities, and learn through a unified template-driven architecture.

**Core Innovation:** Bot-mediated subsystems where each capability domain has an expert bot that communicates A2A via self-describing templates — eliminating manual code wiring.

**Design Philosophy:** Austere and efficient recombinatorial system built on ACP, A2A, and MCP protocols. The kernel (non-protocol code) is intentionally minimal.

---

## Part I: Architectural Foundations

### 1.1 Five Anchor Capabilities

| Anchor | Purpose | Implementation |
|--------|---------|----------------|
| **1. Agent Enablement** | Sovereign agents with WebID, UCAN, ACP | Bot/Replicant taxonomy, agent pods |
| **2. Essential Tools** | MCP servers + Okapi inference | 10 MCP servers (all hkask-) |
| **3. User Sovereignty** | OCAP, privacy, encryption | UCAN delegation, visibility gating, SQLCipher |
| **4. Cybernetic Nervous System** | νKask monitoring, variety counters | ν-events, algedonic alerts (REPLACES OKH + feedback) |
| **5. Composition Registries** | Templates, hLexicon, self-wiring | **Unified registry** with template_type discriminator |

### 1.2 Agent Taxonomy

| Type | Purpose | Interaction | Default Visibility | Examples |
|------|---------|-------------|-------------------|----------|
| **Bot** | Process/task execution | Machine-to-machine (A2A) | Public/Shared | Memory, Spandrel, Scholar, Okapi bots |
| **Replicant** | Human assistance | Human-to-agent (H2A) | Episodic=Private, Semantic=Public | Curator (default) |

**Both are agents** — same pod instantiation, WebID, UCAN, ACP. Distinction is **design intent**, not implementation.

**Key Principles:**
- No escalation primitive between bots and replicants
- Curator's role: loop human into ongoing agent discussion via kask chat
- Bots produce public artifacts; replicants produce private-by-default (episodic) or public-by-default (semantic/templates)
- Ownership confers modification rights (no CNS gating, no approval flow)

### 1.3 Bot-Mediated Subsystem Pattern

```
┌─────────────────────────────────────────────────────────────┐
│                    HUMAN USER                               │
│                          │                                  │
│                          ▼                                  │
│              ┌─────────────────────┐                       │
│              │   Curator           │                       │
│              │   (Replicant)       │                       │
│              └──────────┬──────────┘                       │
│                         │                                   │
│         Orchestrates via templates (self-wiring)            │
│                         │                                   │
│         ┌───────────────┼───────────────┐                  │
│         ▼               ▼               ▼                  │
│  ┌─────────────┐ ┌─────────────┐ ┌─────────────┐          │
│  │ Memory Bot  │ │ Spandrel    │ │ Okapi Bot   │          │
│  │             │ │ Bot         │ │             │          │
│  │ (expert)    │ │ (expert)    │ │ (expert)    │          │
│  └─────────────┘ └─────────────┘ └─────────────┘          │
│         │               │               │                  │
│         └───────────────┴───────────────┘                  │
│                         │                                   │
│              A2A Communication                              │
│              Template Registry                              │
│              (no manual wiring)                             │
└─────────────────────────────────────────────────────────────┘
```

**Key Principle:** Each subsystem has an expert bot curator. Curator bots communicate A2A via template-mediated coordination — replacing hand-wired code with self-describing templates.

---

## Part II: Technical Architecture

### 2.1 Crate Structure (21 crates total)

```
hkask-workspace/
├── Core (11 crates — ≤30k lines total)
│   ├── hkask-types           # ~2,000 — ID types, ν-event, hLexicon, visibility
│   ├── hkask-storage         # ~4,000 — SQLite + SQLCipher, triples, vectors, blobs, Git CAS
│   ├── hkask-memory          # ~3,000 — Semantic/episodic pipelines (analytic distinction)
│   ├── hkask-cns             # ~2,000 — Cybernetic Nervous System, variety counters, algedonic alerts
│   ├── hkask-templates       # ~5,000 — Registry, hLexicon, cascade, resolver
│   ├── hkask-agents          # ~2,500 — Pods, UCAN, bot/replicant, Curator, manifests
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
    ├── ACP Protocol          # Agent communication
    └── MCP Protocol          # Tool protocol
```

**Removed (Hallucinations):**
- `hkask-mcp-feedback` → CNS handles all feedback
- Swarm aggregation, consensus mechanisms → Not in minimal system
- Forecasting crate → Explicitly excluded by user
- Cross-machine sync → Local-only, Git handles backup
- Bot reputation system → Not a core requirement
- Bot marketplace → Not a core requirement
- Curator customization → Curator fixed; users create own replicants
- SemVer versioning → Git-only (SHA-based)
- UCAN → Deferred to multi-host (OCAP-only for h-bar)
- OpenCode-style condenser → Deleted per user directive
- OpenHands-style condenser → Deleted per user directive

**Consolidated:**
- Template MCP + Prompts MCP → `hkask-mcp-prompts` (single crate)
- All bitemporal crates → `hkask-storage` (single crate)

**Named:**
- Cybernetic Nervous System → **CNS** (replaces νKask/OKH terminology)
- Tracing namespace → `cns.*` (replaces `okh.*`)

### 2.2 MCP Servers (10 Total)

| MCP Server | Purpose |
|------------|---------|
| `hkask-mcp-inference` | Okapi-backed LLM inference |
| `hkask-mcp-storage` | Storage operations (triples, embeddings, blobs) |
| `hkask-mcp-memory` | Semantic/episodic memory operations |
| `hkask-mcp-embedding` | Embedding generation, similarity search |
| `hkask-mcp-condenser` | Template condensation, abstraction, summarization |
| `hkask-mcp-ensemble` | Multi-agent coordination, chat orchestration |
| `hkask-mcp-web` | Web search, scrape, extract |
| `hkask-mcp-scholar` | Academic research |
| `hkask-mcp-spandrel` | Graph analysis |
| `hkask-mcp-doc-knowledge` | Document extraction |

**Excluded:**
- Axolotl/fine-tuning → Not minimal
- Telnyx → Unused
- FMP/FAL → Unused
- Forecast → Use cascade skills
- RSS-reader → Use web

### 2.3 Storage Architecture

**Single Storage Crate:** `hkask-storage`

**Stores:**
1. **Bitemporal triples with confidence** (Bayesian)
   - Memory (episodic + semantic) decimated to triples
   - Confidence is first-class (Bayesian combination/retraction)
2. **Embedding vectors** (no fine-tuning vectors)
3. **Blobs** (PDFs, media, etc. — store and identify types)

**Schema:**
```sql
-- Bitemporal triples (semantic + episodic memory)
CREATE TABLE triples (
    id              UUID PRIMARY KEY,
    entity          TEXT NOT NULL,
    attribute       TEXT NOT NULL,
    value           JSONB NOT NULL,
    valid_from      TIMESTAMPTZ NOT NULL,
    valid_to        TIMESTAMPTZ,  -- NULL = still valid
    transaction_at  TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    confidence      FLOAT NOT NULL DEFAULT 1.0,
    perspective     TEXT,  -- NULL = semantic, SOME(agent_id) = episodic
    visibility      TEXT NOT NULL DEFAULT 'public',
    owner_webid     TEXT NOT NULL,
    INDEX idx_entity (entity),
    INDEX idx_valid (valid_from, valid_to),
    INDEX idx_perspective (perspective),
    INDEX idx_visibility (visibility)
);

-- Embeddings (for semantic search)
CREATE TABLE embeddings (
    id              UUID PRIMARY KEY,
    entity_ref      UUID REFERENCES triples(id),
    vector          BLOB NOT NULL,  -- Serialized f32 array
    dimensions      INT NOT NULL,
    model           TEXT NOT NULL,
    created_at      TIMESTAMPTZ DEFAULT NOW()
);

-- ν-events (cybernetic audit trail)
CREATE TABLE nu_events (
    id              UUID PRIMARY KEY,
    timestamp       TIMESTAMPTZ NOT NULL,
    observer_webid  TEXT NOT NULL,
    phase           TEXT NOT NULL,
    observation     JSONB NOT NULL,
    regulation      JSONB,
    outcome         JSONB,
    recursion_depth INT NOT NULL,
    parent_event    UUID,
    visibility      TEXT NOT NULL DEFAULT 'private',
    INDEX idx_timestamp (timestamp),
    INDEX idx_observer (observer_webid)
);
```

**Git Wrapping:**
- Artifacts wrapped in Git for versioning
- Public artifacts can be cloned/forked
- Git snapshots = backup (private GitHub repo)
- No SemVer — Git SHA is the version

**Encryption:**
- SQLite + SQLCipher (encrypted at rest)
- Encryption passphrase at startup (interactive prompt)

**Vector Index:** sqlite-vec (default), **Qdrant embedded** (fallback contingency)

### 2.4 Memory Architecture

**Analytic Distinction Only:**

| Aspect | Semantic | Episodic |
|--------|----------|----------|
| **Perspective** | Third-person (graph-anchored facts) | First-person (agent/user experience) |
| **Definition** | Social proof, shared knowledge | Personal experience by definition |
| **Transformation** | Template concern (framing) | Template concern (framing) |
| **Promotion** | **NO PROMOTION** (categorically different) | **NO PROMOTION** (categorically different) |

**Key Principles:**
- Episodic and semantic are categorically different — one does NOT promote to the other
- Same event can produce either via different template framings
- Transformation lives in template, not in promotion pipeline
- Episodic memory is incidental, not load-bearing for Curator function
- Curators run on charters and templates from first boot

### 2.5 Composition Registries (Unified with Template Type Discriminator)

**Decision:** Single unified registry with `template_type` discriminator (not three separate registries).

| Template Type | Purpose | Academic Grounding |
|---------------|---------|-------------------|
| **Prompt (WordAct)** | What to say — LLM/tool calls | Speech-act theory (Austin/Searle) |
| **Process (FlowDef)** | What to do — multi-step workflows | Process algebra / workflow patterns |
| **Cognition (KnowAct)** | How to think — knowing, learning, thinking | Cognitive science / epistemology |

**Unified Abstract Structure:**
- Same templating language (Jinja2 + LLM)
- Same storage and call mechanism
- Same registry shape
- Differences: `template_type` metadata and lexicon terms, NOT Rust code paths

**Key Invariant:** Rust is the loom (fixed logic). YAML/Jinja2 is the thread (mutable content). Selection intelligence lives in Jinja2/LLM, not Rust branching.

**Manifest vs Template:**
- **Manifest (YAML):** Immutable process definition with steps (`select`, `populate`, `execute`). Bot's charter.
- **Template (Jinja2):** Dynamic document form with fields. Bot's tool. Composes into varying documents based on input.

**Dispatch Pattern:**
1. `registry-dispatch-bot` executes `dispatch.yaml` manifest
2. Step 1 (`select`): Render `selector.j2` → call fast local model → choose best-fit template
3. Step 2 (`populate`): Bind input into selected template's Jinja2 fields
4. Step 3 (`execute`): Submit rendered document to model/tool per template contract

**CNS Integration:**
- `cns.prompt.select` — template selection event
- `cns.prompt.render` — Jinja2 render event
- `cns.prompt.outcome` — execution result with confidence

**Matroshka Limits:**
- Root template has matroshka number 0
- Templates called by root have matroshka number 1
- Hard limit: matroshka number ≤ 7 (configurable per template)

### 2.6 CNS (Cybernetic Nervous System)

**Replaces:** OKH + Feedback Loop (deprecated terminology)

**Components:**
- ν-events (cybernetic audit trail)
- Variety counters (Ashby's Law)
- Algedonic alerts (variety deficit → escalation)

**Span Namespace:**
- `cns.*` — replaces `okh.*` namespace
- `cns.connector.*` — external I/O (LLM dispatch, OCR, embeddings)
- `cns.pipeline.*` — multi-stage processing flows
- `cns.tool.*` — tool governance and invocation signals
- `cns.prompt.*` — prompt feedback loop (rendered, validated, outcome)
- `cns.agent_pod.*` — agent lifecycle (populated, registered, activated, delegation)

**Algedonic Alert:** Variety deficit >100 → escalate to Curator/human

**Implementation:**
- `hkask-cns` crate (≤1,200 LOC) — thin outcome ingestion, span emission
- `cns-curator` bot — owns cognition templates for drift detection, calibration, regression diagnosis

### 2.7 Bot Manifests

**Structure:**
```yaml
---
bot:
  name: memory-bot
  type: Bot
  binding_contract: true
  editor: curator-or-human-admin

capabilities:
  - tool:memory:remember
  - tool:memory:recall

rights:
  - read: public_semantic_memory
  - write: own_episodic_memory

responsibilities:
  - respond_to: memory_tool_calls
  - emit: ν_events

process_template:
  cascade:
    core:
      - template: memory/recall
```

**Editing Workflow:**
1. `kask bot manifest pull <bot-name>` — Download current manifest
2. Edit YAML locally
3. `kask bot manifest push <bot-name> manifest.yaml` — Validate and upload

**Validation:**
- YAML syntax
- Required fields present
- Capability references exist
- Rights/responsibilities well-formed

**Running vs Invoked:**
| Aspect | Running (Bot) | Invoked (Skill) |
|--------|---------------|-----------------|
| Lifecycle | Always active, listening | On-demand render |
| Trigger | ACP/MCP call arrives | Template invoked |
| State | Persistent listener | Ephemeral execution |
| Same Substrate? | Yes (template registry) | Yes |

---

## Part III: Implementation Constraints

### 3.1 Hallucinations Removed

| Feature | Status |
|---------|--------|
| Bot Reputation System | REMOVED |
| Bot Swarms | REMOVED |
| Cross-Machine Sync | REMOVED |
| Bot Marketplace | REMOVED |
| Curator Customization | REMOVED |
| SemVer Template Versioning | REMOVED (Git-only) |
| Separate Feedback Crate | REMOVED (CNS handles all) |
| Promotion Pipeline | REMOVED (episodic/semantic categorical) |
| Escalation Primitive | REMOVED (Curator loops human in) |
| Visibility Type System | REMOVED (OCAP-enforced) |
| OCT-H Currency | REMOVED (not in hKask) |
| Fine-tuning (axolotl) | REMOVED |
| OpenCode-style Condenser | REMOVED (user directive) |
| OpenHands-style Condenser | REMOVED (user directive) |
| UCAN (h-bar) | DEFERRED (OCAP-only, multi-host later) |

### 3.2 Design Decisions

| Decision | Status |
|----------|--------|
| Git-only versioning | CONFIRMED |
| Bayesian confidence | CONFIRMED (combine, subtract, join, decay) |
| No promotion (episodic→semantic) | CONFIRMED |
| Curator fixed (one persona) | CONFIRMED |
| Ownership confers modification rights | CONFIRMED |
| OCAP for visibility (not typed enum) | CONFIRMED |
| CNS naming (replaces νKask/OKH) | CONFIRMED |
| 10 MCP servers (6 Stack + 4 Arsenal) | CONFIRMED |
| Bot manifest binding contract | CONFIRMED |
| CLI/API pull-edit-push workflow | CONFIRMED |
| sqlite-vec + Qdrant contingency | CONFIRMED |
| acp-runtime (Rust-native ACP) | CONFIRMED |
| Interactive passphrase at startup | CONFIRMED |
| All condenser algorithms except OpenCode/OpenHands | CONFIRMED |

### 3.3 Deferred to v1.1+

- Checkpoint fallback (failure recovery) — v1.0: fail fast
- Multi-trigger escalation — v1.0: explicit request only (`@human`, `/escalate`)
- Embedding model version awareness — Embedding MCP responsibility
- Curator retirement — Not in minimal system

---

## Part IV: Implementation Roadmap

### Phase 1: Foundation (Weeks 1-3)
- `hkask-types` — ID types, ν-event, hLexicon
- `hkask-storage` — SQLite + SQLCipher, triples, embeddings
- `hkask-memory` — Semantic/episodic pipelines

### Phase 2: Core (Weeks 4-6)
- `hkask-cns` — Cybernetic Nervous System, variety counters
- `hkask-templates` — Registry, hLexicon, cascade
- `hkask-agents` — Pods, UCAN, bot/replicant, manifests

### Phase 3: Surface (Weeks 7-8)
- `hkask-mcp` — MCP runtime, dispatch
- `hkask-cli` — CLI commands
- `hkask-api` — HTTP API, utoipa

### Phase 4: Integration (Weeks 9-10)
- `hkask-ensemble` — Multi-agent chat
- `hkask-keystore` — OS keychain
- MCP servers (10 crates)
- Okapi integration

---

## Part V: Success Criteria

1. **Agent Enablement:** Bot and replicant pods instantiate with WebID, UCAN, ACP
2. **Tool Invocation:** 10 MCP servers callable via template-mediated patterns
3. **User Sovereignty:** OCAP delegation, SQLCipher encryption, private/public gating
4. **CNS Monitoring:** ν-events emitted, variety counters tracked, algedonic alerts trigger (`cns.*` namespace)
5. **Template Composition:** Prompt/Process/Cognition templates render via unified substrate
6. **Line Budget:** ≤30,000 lines Rust (excluding protocols, MCPs, Okapi)
7. **No Hallucinations:** All features traceable to user requirements
8. **Bayesian Confidence:** Combination, subtraction, join operations correct
9. **Encryption:** Interactive passphrase at startup, SQLCipher-verified

---

## Document Lineage

**Current Version:** v0.21.0 (Pre-alpha MVP)

**Lineage:**
- **v0.21.0:** Pre-alpha MVP baseline — unified registry (template_type discriminator), manifest/template distinction, CNS integration, ERD complete, loom/thread metaphor

**Source Documents:**
- `registry-templating-prompt-v2.md` — Registry & templating system design (unified registry, dispatch pattern, CNS integration)
- `claude-architecture-hkask.md` — Insights incorporated into this spec (revision 2)
- `claude-says-hkask-corrections.md` — Claude's correction analysis
- `hKask-architecture-corrections-v1.1.md` — Hallucination removals
- `hKask-architecture-corrections-v1.2.md` — MCP server count, bot manifest decisions

**Current Document Set:** 8 total (5 active + 3 reference)
- Active: `hKask-architecture-master.md`, `hKask-architecture-index.md`, `hKask-hLexicon.md`, `hKask-Curator-persona.md`, `hKask-erd.md`, `hKask-implementation-handoff.md`, `registry-templating-prompt-v2.md`
- Reference: `vKask-erd.md`, `vKask-cybernetic-constant.md`, `MODEL_CATALOG.md`

---

*ℏKask — Planck's Constant of Agent Systems — v0.21.0*
*Unified registry. Manifest/template distinction. Rust = loom, YAML/Jinja2 = thread.*
*MVP in progress.*