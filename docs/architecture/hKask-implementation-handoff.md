# hKask Implementation — Agent Handoff Prompt

## Context

You are continuing implementation of **hKask** (ℏKask — "Planck's Constant of Agent Systems"), a minimal agent-native container platform in Rust.

**Architecture Status:** Pre-alpha MVP (v0.21.0)  
**Line Budget:** ≤30,000 lines Rust (excluding ACP/MCP protocols, Okapi)  
**Current LOC:** ~5,135 lines Rust (17% of budget)  
**Crate Structure:** 21 crates (11 core + 10 MCP servers)
**Tests:** 114 passing across workspace

---

## Implementation Progress (Updated 2026-05-19)

### ✅ Completed Phases

#### Phase 1-3: Security Foundation (~1,500 LOC)
- **hkask-keystore**: OS keychain integration, AES-256-GCM encryption
- **hkask-storage**: SQLite + SQLCipher with Argon2id key derivation
- **hkask-types**: OCAP capability-based access control with Ed25519 signatures
- Visibility gating (private/public/epistemic)

#### Phase 4: CNS + Templates (~3,600 LOC)
- **hkask-cns** (622 LOC): Cybernetic Nervous System
  - `spans.rs`: `cns.*` namespace spans (`cns.tool.*`, `cns.prompt.*`, `cns.agent_pod.*`)
  - `variety.rs`: Variety counters, deficit tracking
  - `algedonic.rs`: Algedonic alerts (variety deficit >100 → escalate)
  - 18 tests passing

- **hkask-templates** (~800 LOC): Unified registry + templating system
  - `registry.rs`: Template registry with `template_type` discriminator (Prompt/Process/Cognition)
  - `manifest.rs`: Manifest executor (~50 LOC fixed logic)
  - `renderer.rs`: minijinja-based Jinja2 rendering with filesystem loading
  - `cascade.rs`: Pre/core/post stage composition
  - `ports.rs`: Hexagonal architecture ports (ManifestExecutor, TemplateRenderer, RegistryIndex, etc.)
  - 23 tests passing

- **Template Files** (7 core templates in `registry/templates/`):
  - `prompt_selector.j2` — Template selection (Cognition)
  - `prompt_render.j2` — Prompt composition (Prompt)
  - `prompt_execute.j2` — Execution wrapper (Prompt)
  - `cognition_detect.j2` — Drift detection (Cognition)
  - `cognition_calibrate.j2` — Calibration planning (Cognition)
  - `process_recall.j2` — Memory recall workflow (Process)
  - `process_dispatch.j2` — Tool dispatch workflow (Process)

### 🔄 In Progress

#### Phase 4: Templates (Completing)
- Filesystem template loading: ✅ Complete
- Path resolution (env var + executable-relative): ✅ Complete
- Bootstrap manifest integration: ✅ Complete
- CNS event emission in manifest executor: ⏳ Pending

### ⏳ Remaining Phases

#### Phase 5: MCP Servers (Weeks 9-10 — ~10,000 LOC)
1. **hkask-mcp-inference** (1,400 LOC) — Okapi + Ollama
2. **hkask-mcp-embedding** (800 LOC) — Ollama embeddings
3. **hkask-mcp-ensemble** (700 LOC) — bot deliberation
4. **hkask-mcp-condenser** (2,200 LOC) — RTK-style + flashrank + reranker
5. **hkask-mcp-memory** (1,900 LOC) — atop storage MCP + Bayesian ops
6. **hkask-mcp-spandrel** (1,500 LOC) — graph exploration
7. **hkask-mcp-doc-knowledge** (1,200 LOC) — doc extraction
8. **hkask-mcp-web** (800 LOC) — web search
9. **hkask-mcp-scholar** (800 LOC) — academic search
10. **hkask-mcp-storage** (existing — storage operations)

#### Phase 6: Agents + Ensemble (~4,000 LOC)
- **hkask-agents**: Pods, ACP, bot/replicant, Curator
- **hkask-ensemble**: Multi-agent chat (NO swarms)
- **hkask-mcp**: MCP runtime, dispatch

#### Phase 7: CLI + API (~4,000 LOC)
- **hkask-cli**: CLI commands (`kask chat`, `kask bot manifest pull/push`)
- **hkask-api**: HTTP API, utoipa OpenAPI

#### Phase 8: Integration + Verification
- Seed templates (complete)
- Curator instantiation
- Success criterion test (16 items from master spec)
- LOC audit (≤30,000)

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

## Registry & Templating System Design

**Reference:** `~/Clones/hKask/docs/registry-templating-prompt-v2.md` — Complete design specification.

**Core Invariant:** Rust is the loom (fixed logic). YAML/Jinja2 is the thread (mutable content).

### Unified Registry (Not Three)

**Decision:** Single registry with `template_type` discriminator (`Prompt`, `Process`, `Cognition`).

**Rationale:**
- P1 (No trait without two consumers): No distinct consumers for separate registries
- C4 (Repetition is missing primitive): Three registries would be repetition, not genuine distinction
- Selection intelligence lives in Jinja2/LLM, not Rust branching

### Manifest vs Template

| Entity | Technology | Purpose | Mutability |
|--------|------------|---------|------------|
| **Manifest** | YAML | Process definition with steps (`select`, `populate`, `execute`) | Edited over time |
| **Template** | Jinja2 | Dynamic document form with fields | Rendered per invocation |

**Dispatch Pattern:**
1. `registry-dispatch-bot` executes `dispatch.yaml` manifest
2. Step 1 (`select`): Render `selector.j2` → fast local model → best-fit template
3. Step 2 (`populate`): Bind input into selected template's Jinja2 fields
4. Step 3 (`execute`): Submit rendered document to model/tool per contract

### CNS Integration

- `cns.prompt.select` — template selection event
- `cns.prompt.render` — Jinja2 render event
- `cns.prompt.outcome` — execution result with confidence

### Matroshka Limits

- Default depth: 7
- Configurable per template
- Enforced by Rust executor

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
- **Three separate registries** (unified registry with `template_type` discriminator)
- **Rust-based template selection** (selection intelligence in Jinja2/LLM)
- **Hard-coded process logic** (processes defined in YAML manifests)

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
   - Read `hKask-architecture-master.md` (sole authoritative spec, v0.21.0)
   - Read `AGENTS.md` (operating guide, Coco Chanel principles)
   - Read `hKask-hLexicon.md` (canonical vocabulary, ≤75 terms)
   - Read `hKask-Curator-persona.md` (persona specification)
   - Read `registry-templating-prompt-v2.md` (**Registry & templating system** — unified registry, manifest/template distinction, dispatch pattern)
   - Read `hKask-erd.md` (**Entity diagrams** — Mermaid ERDs, data flow, state machines)

2. **Create workspace skeleton:**
   ```bash
   cd ~/Clones/hKask
   git init
   cargo init --name hkask-workspace
   ```

3. **Set up virtual workspace:**
   - Create `Cargo.toml` with `[workspace]` and `[workspace.dependencies]`
   - Create empty crate stubs for all 11 Core crates
   - Set up CI: `cargo check`, `test`, `clippy -D warnings`, `fmt --check`, `tokei`

4. **Begin Phase 1:**
   - Start with `hkask-keystore` (smallest, no dependencies, 600 LOC)
   - Then `hkask-types` (foundation for all other crates, 2,000 LOC)
   - Then `hkask-storage` (SQLite + SQLCipher schema, 4,000 LOC)

5. **Phase 2 Priority:** `hkask-templates` (5,000 LOC) — registry, hLexicon, minijinja, manifest executor

6. **Remember:**
   - At 30,001 lines, the agent is fired and another tries again
   - No silent draws on reserve
   - No hallucinations
   - As simple as possible, but no simpler
   - Rust is the loom; YAML/Jinja2 is the thread

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

### The Loom and the Thread

**Rust is the loom.** YAML/Jinja2 is the thread. The loom doesn't change when you weave a different pattern.

| Layer | Technology | Budget | Mutability |
|-------|------------|--------|------------|
| **Hard (Kernel)** | Rust | ≤30,000 LOC | Fixed, stable |
| **Soft (Material)** | YAML, Jinja2, MD | Unlimited | Mutable, evolving |

**Rust owns:** Parsing YAML steps, rendering Jinja2 via minijinja, enforcing matroshka depth, validating hLexicon terms, routing MCP/LLM calls.

**Rust does NOT own:** Which templates exist, what they say, how selection logic is phrased, what steps a manifest contains, what a prompt looks like.

### Questions to Ask Before Adding Anything

1. **Is this one of the Five Anchors?** If not, what anchor does it serve?
2. **Does this have two consumers?** (P1) If not, is it truly needed?
3. **Is this a stub or placeholder?** (P6, C6) If yes, delete it.
4. **Is this unwired code?** (C2, C3) If yes, does it have a named owner and timeline?
5. **Can this be simpler?** Is there a concrete shape that serves the actual callsite?
6. **Does this belong in the kernel or a template?** Templates evolve; kernel endures.
7. **Is this Rust logic that should be YAML?** If the behavior can be expressed as a manifest step, do it in YAML.

### The hKask Commitment

- **≤30,000 lines** — Not one line more. At 30,001, the agent is fired and another tries again.
- **No silent draws on reserve** — Every change cited, every line accounted for.
- **No hallucinations** — All features traceable to architecture spec.
- **No speculation** — Code that is not needed today is debt, not investment.
- **No ceremony** — Direct, technical, concise. No preamble, no emoji, no questions.

### When in Doubt

1. Read the architecture spec (`hKask-architecture-master.md`)
2. Read the registry/templating design (`registry-templating-prompt-v2.md`)
3. Ask: "What is the simplest thing that could possibly work?"
4. Implement that. Test it. Ship it.
5. Iterate only when pressure demands it.

---

## Contact

If you encounter ambiguity or need clarification:
1. Check `hKask-architecture-master.md` first (sole authoritative spec)
2. Check `AGENTS.md` (operating guide)
3. Ask the user if the spec is unclear

**Do not hallucinate features.** All features must be traceable to the architecture spec.

---

## Next Agent Continuation — Immediate Tasks

### Current State Summary

**hkask-templates crate:** ✅ Complete (23 tests, ~800 LOC)
- Unified registry with `template_type` discriminator
- Filesystem template loading (env var + executable-relative path)
- 7 core Jinja2 templates in `registry/templates/`
- minijinja renderer with contract parsing
- Cascade composition (pre/core/post stages)
- Manifest executor with depth limiting

**hkask-cns crate:** ✅ Complete (18 tests, ~622 LOC)
- Span tracking (`cns.tool.*`, `cns.prompt.*`, `cns.agent_pod.*`)
- Variety counters with deficit tracking
- Algedonic alerts (threshold: 100)

**Workspace:** ✅ 114 tests passing, ~5,135 LOC Rust (17% of 30k budget)

### Immediate Next Steps

#### 1. Integrate CNS Port in Manifest Executor (Priority: High)

The `ManifestExecutorImpl` has a `CnsPort` generic but CNS events are not yet emitted during execution. Add CNS span emission at each manifest step:

**File:** `crates/hkask-templates/src/manifest.rs`

**Changes needed:**
- Emit `cns.prompt.select` after Select action (include selected template ID, confidence)
- Emit `cns.prompt.populate` after Populate action (include binding count)
- Emit `cns.prompt.execute` after Execute action (include outcome, confidence)
- Emit `cns.prompt.outcome` at end of manifest execution (final result, total steps, duration)

**Example:**
```rust
// In execute_step(), after Select:
self.cns.emit(
    "cns.prompt.select",
    Value::Object(serde_json::json!({
        "selected_template": selected_id,
        "confidence": confidence,
        "rationale": rationale
    }).as_object().unwrap().clone()),
    confidence,
);
```

#### 2. Create YAML Manifest Files (Priority: Medium)

The bootstrap manifest is hardcoded in Rust. Create external YAML manifest files that can be loaded at runtime:

**Directory:** `registry/manifests/`

**Files to create:**
- `dispatch.yaml` — Bootstrap dispatch manifest (matches current hardcoded version)
- `memory_recall.yaml` — Memory recall workflow
- `tool_dispatch.yaml` — Tool dispatch workflow

**YAML schema:**
```yaml
manifest:
  id: registry/dispatch
  name: Registry Dispatch
  description: Bootstrap process for all registry resolution
  steps:
    - ordinal: 1
      action: select
      description: Select best-fit template
      template_ref: prompt/selector
      model_tier: fast_local
      mcp: hkask-mcp-inference
      renderer: minijinja
```

**Rust changes:**
- Add `serde_yaml` dependency to `hkask-templates`
- Add `ProcessManifest::load_from_yaml(path: &Path)` constructor
- Update `Registry::bootstrap_manifest()` to load from YAML file

#### 3. Add Template Loading Tests (Priority: Medium)

Add integration tests that verify end-to-end template loading and rendering:

**File:** `crates/hkask-testing/integration-tests/template_loading.rs`

**Test scenarios:**
- Load all 7 bootstrap templates from filesystem
- Render each template with valid bindings
- Verify contract parsing extracts correct input/output fields
- Verify lexicon term extraction
- Verify template_type detection

#### 4. Prepare for MCP Server Implementation (Priority: Low)

Create stub MCP server crates with proper structure:

**Crates to scaffold:**
- `mcp-servers/hkask-mcp-inference/`
- `mcp-servers/hkask-mcp-embedding/`
- `mcp-servers/hkask-mcp-ensemble/`
- (etc. for all 10 MCP servers)

**Each crate needs:**
- `Cargo.toml` with `rmcp` dependency
- `src/lib.rs` with MCP server skeleton
- `src/tools.rs` with tool definitions
- Basic README

**Note:** Don't implement tools yet — just create the crate structure. Implementation comes in Phase 5.

---

## Questions for User

Before proceeding, clarify:

1. **CNS Integration:** Should CNS events be emitted synchronously (blocking) or asynchronously (spawned task)? Current `CnsPort` trait is synchronous.

2. **YAML Manifests:** Should the hardcoded bootstrap manifest in `Registry::bootstrap_manifest()` be replaced entirely with YAML loading, or should YAML be an alternative option?

3. **Template Path Fallback:** Current fallback is `./registry/templates/` relative to CWD. Should this also check `~/.config/kask/templates/` for user-customized templates?

4. **MCP Server Priority:** Which MCP server should be implemented first? Recommendation: `hkask-mcp-inference` (required for template selector to work).

---

*ℏKask — Planck's Constant of Agent Systems — v0.21.0*
*As simple as possible, but no simpler.*
*Rust is the loom. YAML/Jinja2 is the thread.*
*MVP in progress.*