# Handoff: Condenser Saliency Refactoring — 2026-06-27

## Session Context

Built the complete Matrix/Conduit communication pipeline for hKask: 7R7 listener → CNS bridge → CommunicationWatcher → CurationLoop → MetacognitionLoop → MCP response dispatch. Integrated Communication Accommodation Theory (CAT) — `convergence_bias` scalar on `AgentPersona` governs speak/silent decisions. Full workspace compiles, 65+ tests pass. 

The engagement gate currently uses `body.contains(agent_name)` for the speak/silent decision. During review, identified that saliency scoring — already implemented in `hkask-condenser`'s `SaliencyRankAlgorithm` — should inform this decision but is currently conflated with text compression in the condenser. The condenser needs to be refactored to expose saliency as a clean, callable concern.

## What Was Done

### Communication Pipeline (16 code files changed)

**CNS Bridge:**
- `crates/hkask-types/src/event.rs` — 4 canonical CNS namespaces (`cns.communication.message|thread|agent|listener`)
- `crates/hkask-storage/src/nu_event_store.rs` — 2 algedonic categories + 5 tests
- `crates/hkask-communication/src/listener.rs` — `NuEventSink` integration; persists `NuEvent` on message observation
- `crates/hkask-services-context/src/context_impl/matrix.rs` — `build_matrix()` accepts event sink, passes to listener
- `crates/hkask-services-context/src/context_impl.rs` — wires event sink; registers `communication_watcher` module

**Message Dispatch:**
- `crates/hkask-cns/src/types/loops/channels.rs` — `CommunicationEvent` struct + `CurationInput::Communication` variant
- `crates/hkask-agents/src/curator/curation_loop.rs` — handler pushes events to `CuratorContext.pending_communication`
- `crates/hkask-agents/src/curator/context.rs` — `pending_communication` storage + `drain_communication_events()` accessor
- `crates/hkask-services-context/src/context_impl/communication_watcher.rs` — polls NuEventStore every 30s, forwards to curation inbox

**CAT Integration:**
- `crates/hkask-agents/src/pod/types.rs` — `CommunicationPosture` struct (2 fields: `convergence_bias`, `invariant_traits`), added as `Option` to `AgentPersona`
- `crates/hkask-agents/src/curator_agent/cat.rs` — `evaluate()` pure function + 6 tests
- `crates/hkask-agents/src/curator_agent/metacognition.rs` — processes communication events, calls `executor.call_tool("communication/send_message", ...)`, posture fields (`curator_name`, `convergence_bias`)
- `crates/hkask-agents/src/curator_agent/mod.rs` — `with_communication_posture()` builder on `CuratorAgent`
- `crates/hkask-templates/src/executor.rs` — `call_tool()` public method on `ManifestExecutor`

**Templates:**
- `registry/templates/curator/metacognition-respond.j2` — CAT-aware response composition (Jinja2)
- `registry/templates/curator/metacognition-diagnose.j2` — updated with communication awareness
- `registry/templates/curator/dispatch_manifest.yaml` — registered `metacognition_respond` route

**Tests:**
- `crates/hkask-communication/tests/matrix_transport_tests.rs` — 12 tests (3 type, 9 Conduit-gated `#[ignore]`)

**Docs:**
- `crates/hkask-communication/README.md` — updated architecture, CNS bridge, CAT
- `mcp-servers/hkask-mcp-communication/README.md` — updated 12 tools, P4 gates
- `AGENTS.md` — communication in capability catalog
- `docs/architecture/ADRs/matrix-server-administration.md` — status, architecture diagram, CAT model

**Side fix:**
- `crates/hkask-services-backup/src/service.rs` — fixed pre-existing `Ok(raw)` type mismatch

### Test Results

- 53/53 hkask-agents unit tests pass
- 6/6 CAT gate tests pass
- 5/5 storage communication namespace tests pass
- 12/12 integration tests pass
- 25/25 hkask-communication tests pass

## What Remains

### HIGH — Condenser Refactoring: Separate Saliency from Compression

The condenser (`hkask-condenser`) has `SaliencyRankAlgorithm` which computes word-frequency-based saliency but it's embedded inside the compression pipeline. The `compress()` method takes input text, ranks lines, drops low-scoring lines, and returns compressed output with `reduction_pct`. There is no way to get a pure saliency score without running the full compression pipeline.

**What to build:**

A new module `crates/hkask-condenser/src/saliency.rs` with:

```rust
/// Score how salient a text prompt is against an agent's persona.
/// Returns 0.0 (no relevance) to 1.0 (highly relevant).
/// Uses the persona's charter description, capabilities, and invariant_traits.
pub fn score_against_persona(text: &str, persona: &AgentPersona) -> f64;

/// Score how salient a text prompt is against an agent's episodic memory.
/// Returns 0.0 (no memory triggers) to 1.0 (strong memory recall).
/// Queries the episodic store for semantically similar triples.
pub fn score_against_memory(text: &str, episodic: &EpisodicMemory) -> f64;
```

These functions reuse the word-frequency logic already in `SaliencyRankAlgorithm.compute_word_frequencies()` but expose it as a clean, callable interface rather than embedding it in the compression pipeline.

**MCP tool to expose:**

Add to `hkask-mcp-condenser`:

```
tool: condenser_score_saliency

Input:
  - text: string          (the prompt/message to score)
  - against: string       ("persona" or "memory")

Output:
  - score: number         (0.0–1.0)
  - against: string       (echoes input)
  - method: string        ("word_frequency" or "semantic_search")
```

The `"persona"` variant uses `score_against_persona` with the condenser server's persona (passed at construction). The `"memory"` variant uses `score_against_memory` with the episodic store available on the condenser server (already constructed).

**Existing code to reuse (do not rewrite):**

- `SaliencyRankAlgorithm.compute_word_frequencies()` in `algorithms.rs` — word frequency map builder
- The line-ranking logic in `SaliencyRankAlgorithm.compress()` — extracts which lines have the highest signal
- `EpisodicMemory` is already available in `hkask-mcp-condenser/src/lib.rs` as `self.memory`

**Design principles:**

1. The `saliency` module is pure — no MCP, no I/O, no async. Just text scoring.
2. The MCP tool is a thin wrapper in `hkask-mcp-condenser/src/lib.rs` that calls the saliency module.
3. The existing `SaliencyRankAlgorithm` internally calls the saliency module for its compression ranking (refactored, not deleted).
4. No new dependencies. Use what's already in `hkask-condenser` and `hkask-mcp-condenser`.

**Connection to the communication gate:**

After the condenser refactoring, update `crates/hkask-agents/src/curator_agent/metacognition.rs` in the communication events block to call the new MCP tool before `cat::evaluate()`:

```rust
// Score message saliency against persona
let persona_score = match executor.call_tool(
    "condenser/condenser_score_saliency",
    serde_json::json!({"text": body, "against": "persona"}),
).await {
    Ok(resp) => resp.get("score").and_then(|v| v.as_f64()).unwrap_or(0.5),
    Err(_) => 0.5, // graceful degradation if condenser unavailable
};

// Saliency pulls effective bias upward
let effective_bias = (bias + persona_score * (1.0 - bias)).min(1.0);
let decision = cat::evaluate(effective_bias, curator_name, event);
```

### MEDIUM — YAML Persona Posture Wiring

The `MetacognitionLoop` has `convergence_bias` wired but currently hardcoded to `0.5` at the `CuratorAgent::with_consolidation(...)` call site in `context_impl.rs` line ~996. If the curator persona YAML defines `communication_posture.convergence_bias`, it should be read and passed through. This is a small wiring change once the persona is accessible at construction time.

### TODO — 7R7 Receptor Specification

The 7R7 framework has 7 receptor bots registered on Conduit but only r7-1 (observer) is active. The other 6 (variety, algedonic, composer, consolidator, cybernetics, curator) need specification and implementation. This is a design session followed by implementation — separate from the condenser work.

### TODO — Per-Replicant Matrix Presence

Deferred to v2. Current design: humans log in as replicants. The replicant is not autonomous — it has a human user steering while it learns. Per-replicant Matrix listeners that operate independently are a future feature.

## Recommended Skills

| Skill | When to activate |
|-------|-----------------|
| `essentialist` | Before touching the condenser — audit existing code for what can be deleted vs. extracted. The `SaliencyRankAlgorithm` is ~200 lines. Apply G1 (deletion test): what behavior vanishes if each method is extracted? |
| `deep-module` | After extraction — ensure the new `saliency` module has depth (high benefit/cost ratio), interface minimalism (≤7 public functions), and passes the deletion test both ways. |
| `coding-guidelines` | During implementation — surface assumptions, enforce simplicity, surgical changes, goal-driven execution. The existing condenser has ~500 lines in algorithms.rs — touch only what's needed for extraction. |
| `idiomatic-rust` | For the `score_against_persona` and `score_against_memory` signatures — these are new public APIs. Type-driven design, ownership clarity, error handling. |

## Key Decisions

1. **`convergence_bias` IS the speak/silent decision.** Derived from Communication Accommodation Theory (Giles). A single scalar replaces trigger enums, rate limiters, and complex state machines. The condenser saliency score modulates it — pulling bias upward for salient content — but the decision remains a single dimension.

2. **The engagement gate is a pure function.** `cat::evaluate(bias, name, event)` has no I/O, no state, no dependencies beyond `CommunicationEvent`. The MCP calls (condenser scoring, send_message) happen in the metacognition loop, not in the gate. This keeps the gate testable and the MCP concerns at the orchestration layer.

3. **The condenser is an MCP server for reuse.** Rather than importing condenser types into the agents crate, tools are called via `executor.call_tool("condenser/...", ...)`. This avoids crate coupling and allows the condenser to evolve independently.

4. **Saliency scoring should be two independent functions.** `score_against_persona` and `score_against_memory` are separate concerns — persona scoring checks against charter/constraints, memory scoring checks against episodic recall. Both contribute to the speak decision but are computed independently.

5. **E2EE is deferred to v2.** SQLCipher/SQLite linking conflict between hkask-storage and matrix-sdk-sqlite. v1 uses TLS-only transport security with on-demand message polling.

6. **Per-replicant Matrix presence is v2+.** Current model: one human, one replicant, one Curator. The replicant has a human steering it. Independent replicant Matrix listeners come later.

## Verification Commands

```bash
# Before starting: verify current state
cargo check -p hkask-agents -p hkask-communication -p hkask-services-context
cargo test -p hkask-agents

# Condenser refactoring targets
cargo check -p hkask-condenser -p hkask-mcp-condenser
cargo test -p hkask-condenser

# After wiring into metacognition
cargo check -p hkask-agents
cargo test -p hkask-agents -- cat

# Full workspace (hkask-mcp has pre-existing errors in git_cas — unrelated)
cargo check -p hkask-agents -p hkask-services-context -p hkask-communication -p hkask-condenser -p hkask-mcp-condenser -p hkask-templates -p hkask-cns -p hkask-types -p hkask-storage
```
