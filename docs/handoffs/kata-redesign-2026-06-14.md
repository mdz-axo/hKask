# Handoff — Kata System Redesign for Recursive Self-Improvement

**Session date:** 2026-06-14
**Project:** hKask v0.27.0
**Handoff from:** Kata engine implementation, model_tier expunged, ports deleted, memory bridge added
**Handoff to:** Redesign kata system as composable recursive self-improvement tools grounded in Toyota Kata methodology

---

## 1. Session Context

This session built a kata execution engine (`crates/hkask-services/src/kata.rs`, ~600 lines) with CLI commands (`kask kata list/show/start`), CNS span emission, OCAP consent hooks, state save/resume, classifier model routing, and experience recording via the daemon. The engine runs all three kata types (improvement, coaching, starter). `model_tier` was fully expunged from all 59 manifests, 4 docs, 2 code structs, templates, and scripts. The `registry/ports/` directory was deleted. Spec replica integration was tested end-to-end with per-dimension centroids. All 6 style corpora were embedded.

**However, the kata engine was built as a linear script runner, not as a cybernetic learning system.** Several design errors were identified late in the session. The kata system needs redesign to fulfill its purpose: composable tools for recursive self-improvement, where agents learn through memory accumulation and skill refinement, grounded in Toyota Kata methodology.

---

## 2. What Was Done (Keep)

| Area | What | Files |
|------|------|-------|
| Kata engine | `KataEngine`, `KataManifest` types, step execution, template rendering, gas tracking | `crates/hkask-services/src/kata.rs` |
| CLI | `kask kata list/show/start` with `--save`/`--resume`/`--ctx`/`--bot` | `crates/hkask-cli/src/commands/kata.rs` |
| Classifier routing | `classifier: true` field → `DI/google/gemma-4-26B-A4B-it` via `HkaskSettings::classifier_model()` | `kata.rs`, `improvement-kata.yaml` |
| Experience recording | Kata completion → `CliExperienceRecorder::record()` via daemon | `kata.rs` |
| CNS spans | `tracing::info!` at cycle/step/question level under `hkask.kata` target | `kata.rs` |
| State persistence | `KataState::save()`/`load()` — JSON round-trip | `kata.rs` |
| OCAP hooks | `KataEngine::with_consent()`, `with_cns()` — callback pattern, not yet wired | `kata.rs` |
| Registry wiring | Engine uses bootstrapped `SqliteRegistry` from main, `Clone` derived | `kata.rs`, `registry_sqlite.rs` |
| `model_tier` expunged | Zero occurrences across all files | 59 manifests, 4 docs, 2 structs, templates, scripts |
| `registry/ports/` deleted | Directory + all stale references in hlexicon, docs | hlexicon, user guide, skill inventory |
| Spec replica tested | `spec_require_writing_quality` + gentle-lovelace → 5 dimension scores | `mcp_protocol.rs`, `replica_centroid_test.rs` |
| All corpora embedded | 6 corpora, ~25K passages, 438MB DB | `data/hkask-styles.db` |
| Ratatui API fix | `CrosstermBackend<Stdout>` for ratatui 0.29 | `transcript_viewer.rs` |
| Template refs fixed | Manifest `template_ref` values match bootstrap registry IDs | `improvement-kata.yaml`, `starter-kata.yaml`, `coaching-kata.yaml` |
| Kata user guide | Updated with real CLI commands, CNS spans, classifier model, save/resume | `docs/guides/kata-user-guide.md` |

---

## 3. Design Errors Identified (Must Redesign)

### 3.1 Linear Script Runner, Not a Learning Loop

The Improvement Kata IS a PDCA cycle (Plan-Do-Check-Act). The engine builds Plan and Do. There is no Check (compare output to target condition) and no Act (adjust based on findings). The cybernetic loop is open.

**Required:** After each step and at cycle completion, compare outputs against declared target conditions. Compute improvement signals. Feed results back into the agent's memory stream and the kata's own state.

### 3.2 Coaching Kata Has No Learner

The 5 questions are asked to an LLM that roleplays both coach and learner. In Toyota Kata, the coach questions a real learner who brings their actual Improvement Kata storyboard data. The coaching cycle reveals the learner's thinking pattern — it's not a scripted dialogue.

**Required:** Coaching kata must be invoked in the context of an active Improvement Kata cycle. The learner's IK state (target condition, current metrics, obstacles, experiments) must be the input to the coaching dialogue.

### 3.3 Starter Kata Doesn't Practice Anything

It records practice names to state with no habit tracking, streak counting, or automaticity scoring. A starter kata that doesn't build habits isn't a starter kata.

**Required:** Track practice frequency, compute streaks, emit CNS automaticity signals. Practices should be actual routines (Five Questions Drill, PDCA on trivial processes, Observation Drill).

### 3.4 No Before/After Measurement

The manifests declare `metric_before` and `metric_after` but the engine never collects either. An improvement kata that doesn't measure improvement isn't an improvement kata.

**Required:** Before starting a cycle, capture the agent's current CNS metrics. After completion, capture again. Compute delta. Record as the improvement signal.

### 3.5 Kata Composition Is Missing

The architecture says starter → improvement → coaching, with CNS monitoring transitions. Each `kask kata start` is isolated with no awareness of prior cycles.

**Required:** The engine must track kata history per agent. Graduation criteria (automaticity > 0.5 → graduate from starter) must be checked. The bundle manifest (kata-pattern.yaml) should orchestrate routing.

### 3.6 Memory Bridge Is an Afterthought

Experience recording was added at the end, after the user pointed out that agents accumulate memory and learn. It should have been designed in from the start.

**Required:** Every step output feeds into the agent's episodic memory stream. After N steps, narrative generation creates observations. The kata's own outputs become part of the agent's semantic knowledge.

---

## 4. What Remains

### HIGH — Redesign Kata as Cybernetic Learning System

**Prerequisite reading (MUST read before implementing):**

| Document | Why |
|----------|-----|
| `docs/architecture/PRINCIPLES.md` | P1-P9 constraints, CNS spans, Magna Carta grounding |
| `docs/architecture/hKask-architecture-master.md` | Overall system architecture, component relationships |
| `docs/guides/kata-user-guide.md` | Current kata design, adoption path, Toyota Kata background |
| `registry/hlexicon/kata-hlexicon.yaml` | Functional role categorization of all kata templates |
| `registry/manifests/improvement-kata.yaml` | Current manifest structure (steps, gas, CNS, metrics) |
| `registry/manifests/starter-kata.yaml` | Current starter practices and outcomes |
| `registry/manifests/coaching-kata.yaml` | Current coaching questions and steps |
| `registry/manifests/kata-pattern.yaml` | Bundle orchestration manifest (different structure) |
| `registry/manifests/kata-iteration.yaml` | Iteration/variance assessment manifest |

**Skills to load before work:**

| Order | Skill | Why |
|-------|-------|-----|
| 1 | **essentialist** | Strip unnecessary complexity. The kata system should be minimal: execute, measure, learn, repeat. |
| 2 | **pragmatic-semantics** | Classify statements by certainty. The kata deals with IS (current condition) vs OUGHT (target condition). Don't confuse them. |
| 3 | **pragmatic-cybernetics** | Every kata component must be a closed feedback loop. Sensor → Model → Regulator → Actuator. No open loops. |
| 4 | **coding-guidelines** | Surgical changes. Touch only what must change. Match existing style. |
| 5 | **zoom-out** | Understand how kata fits into hKask's broader memory/CNS/agent architecture before touching code. |

**Redesign goals:**

1. **Close the cybernetic loop.** Kata steps collect observations → state accumulates → outputs compared to targets → improvement signal computed → agent memory updated → next cycle targets the gap.

2. **Make coaching real.** Coaching kata requires an active Improvement Kata. The learner's IK state feeds the coaching dialogue. Coach and learner are separate roles.

3. **Make starter kata build habits.** Track practice frequency, compute streaks, emit automaticity scores. Practices are real routines, not placeholder names.

4. **Add before/after measurement.** Capture CNS metrics before kata cycle, capture after, compute delta. This IS the improvement signal.

5. **Enable composition.** Track kata history per agent. Check graduation criteria. Route between kata types based on state.

6. **Design memory-first.** Every step output → episodic memory. Every N steps → narrative generation. Kata outputs become semantic knowledge.

### MEDIUM — Wire OCAP Consent

The `with_consent` callback exists but is never called. The manifests declare consent requirements (Curator for improvement, learner for coaching, self for starter). Wire the callback to actual OCAP verification.

### MEDIUM — Wire CNS Runtime

Replace tracing spans with actual `CnsRuntime` integration. Increment variety counters for kata practice. Emit algedonic alerts when improvement stalls.

### LOW — Fix Coaching Prompt Templates

Some coaching question responses come back empty. The prompt engineering in the engine is a stopgap — the actual prompts should be in the `.j2` templates. Refine templates to produce consistent, substantive responses.

---

## 5. Key Decisions to Preserve

1. **`model_tier` is permanently deleted.** Do not reintroduce. Model selection happens via `classifier: true` for classification steps (uses system classifier model `google/gemma-4-26B-A4B-it`) or via the default generation model for reasoning steps.

2. **`registry/ports/` is permanently deleted.** Hexagonal ports are documentation of interfaces already defined in Rust code. They are redundant and were removed as gratuitous complexity.

3. **Kata is 4 skills, not 1.** Based on Mike Rother's primary sources: Toyota Kata has TWO linked behaviors (Improvement Kata + Coaching Kata), Starter Kata are practice routines. Each kata is independently adoptable.

4. **The classifier model is `google/gemma-4-26B-A4B-it`** (Gemma 4 26B MoE via DeepInfra, `DI/` prefix). Defined in `HkaskSettings::classifier_model()`. Used for section type classification and triple extraction. Kata classification steps use this model.

5. **Learning is through memory accumulation.** Agents learn by recording experiences via the daemon, which dual-encodes (episodic + semantic) and generates narratives. The kata must feed into this pipeline, not bypass it.

6. **Toyota Kata is the grounding.** The Improvement Kata (4-step PDCA), Coaching Kata (5-question dialogue), and Starter Kata (practice routines) are directly from Mike Rother's research. Musk's "First Principles" derives from Toyota's Five Whys — both are about fundamentally anchored scientific understanding. The kata system must remain faithful to this methodology.

7. **`DEFAULT_DB_PATH` is `"data/hkask.db"`.** All runtime databases go in `data/`. Do not revert.

8. **`registry/registries/`, `registry/corpora/`, `registry/kata/` are permanently deleted.** Do not recreate.

---

## 6. Build & Test Commands

```bash
cargo check --workspace                    # Build verification
cargo test -p hkask-templates -p hkask-types -p hkask-mcp-spec  # Key test suites
cargo build --bin kask                      # CLI binary

# Kata commands
kask kata list                              # List 5 manifests
kask kata show improvement-kata             # Show manifest details
kask kata start starter-kata --bot Alice    # Starter (no LLM, zero gas)
kask kata start improvement-kata --bot Alice --ctx "capability=span_emission" --save /tmp/state.json
kask kata start coaching-kata --bot Alice --ctx "learner=Bob"
kask kata start improvement-kata --bot Alice --resume /tmp/state.json
```

---

## 7. Key Files

```
crates/hkask-services/src/kata.rs              ← Kata engine (needs redesign)
crates/hkask-cli/src/commands/kata.rs          ← CLI command
crates/hkask-services/src/settings.rs          ← HkaskSettings (classifier model)
crates/hkask-types/src/bundle.rs               ← BundleManifestStep (model_tier removed)
crates/hkask-templates/src/registry_sqlite.rs  ← SqliteRegistry (now Clone)
crates/hkask-services/src/experience.rs        ← CliExperienceRecorder
crates/hkask-services/src/embed.rs             ← EmbedResult (dimension centroids)
registry/manifests/improvement-kata.yaml       ← Improvement manifest
registry/manifests/starter-kata.yaml           ← Starter manifest
registry/manifests/coaching-kata.yaml          ← Coaching manifest
registry/manifests/kata-pattern.yaml           ← Bundle manifest
registry/manifests/kata-iteration.yaml         ← Iteration manifest
registry/hlexicon/kata-hlexicon.yaml           ← Kata functional role catalog
registry/templates/kata-improvement/           ← Improvement templates (5)
registry/templates/kata-starter/               ← Starter templates (5)
registry/templates/kata-coaching/              ← Coaching templates (6)
registry/templates/kata/                       ← Bundle templates (7)
docs/guides/kata-user-guide.md                 ← User guide (updated)
docs/architecture/PRINCIPLES.md                ← Must read before redesign
docs/architecture/hKask-architecture-master.md ← Must read before redesign
```

---

*ℏKask - A Minimal Viable Container for Agents — v0.27.0*
