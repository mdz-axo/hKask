---
title: "Replica LoRA Training — Style Confidence Feedback Loop"
audience: [architects, developers, agents]
last_updated: 2026-06-30
version: "0.31.0"
status: "Draft — 3-Phase Plan"
domain: "Cross-cutting"
mds_categories: [domain, composition, trust, lifecycle]
anchored_on: [PRINCIPLES.md §0, P3, P4, P5, P8, P9, P12]
reviewed_via: [improve-codebase-architecture, coding-guidelines, grill-me, essentialist]
grounded_in: "Mavridis et al. (2025) — LLMs for Intelligent RDF Knowledge Graph Construction, Frontiers in AI, doi:10.3389/frai.2025.1546179"
---

# Replica LoRA Training — Style Confidence Feedback Loop

**Purpose:** Define the phased architecture for adding LLM confidence scoring to the replica compose pipeline, using those scores to gate a closed training feedback loop that produces authorial-style LoRA adapters.

**Decision:** The paper by Mavridis et al. (2025) demonstrates that **vectors for retrieval + LLM for reasoning** is the pattern, and that **LLM confidence scoring** provides a second quality signal orthogonal to embedding distance. hKask's replica server already implements the retrieval + generation pattern (via `sqlite-vec` KNN → LLM prose generation → centroid distance validation). The missing element is LLM confidence — a semantic quality assessment that complements the geometric centroid distance. Adding it closes the gap with the paper and unlocks a training feedback loop.

**Status:** Not implemented. This plan defines the architecture; implementation is pending.

---

## 1. Architecture Overview

### 1.1 The Full Loop (Target State)

```
                        ┌───────────────────────────────────────────────────┐
                        │                 REPLICA COMPOSE PIPELINE           │
                        │                                                   │
  User prompt ──────────►  1. Embed prompt (Qwen3-Embedding-0.6B)           │
                        │  2. KNN retrieve exemplar passages (sqlite-vec)   │
                        │  3. LLM generates prose (generation model)        │
                        │  4. CLASSIFIER evaluates style confidence ────────┤
                        │  5. Centroid distance validation (embedding)      │
                        │  6. Combined pass/fail verdict                    │
                        │                                                   │
                        └───────────────────────┬───────────────────────────┘
                                                │
                        ┌───────────────────────▼───────────────────────────┐
                        │              QUALITY ROUTING                       │
                        │                                                   │
                        │  High confidence + low centroid distance          │
                        │    → training_ingest_qa (positive example)        │
                        │    → training_record_invocation (success)         │
                        │                                                   │
                        │  Low confidence OR high centroid distance         │
                        │    → training_curate_feedback (negative example)  │
                        │    → training_record_invocation (failure)         │
                        │                                                   │
                        └───────────────────────┬───────────────────────────┘
                                                │
                        ┌───────────────────────▼───────────────────────────┐
                        │               TRAINING PIPELINE                    │
                        │                                                   │
                        │  training_ingest_qa  →  ChatML dataset (JSONL)    │
                        │       │                                           │
                        │       ▼                                           │
                        │  training_assemble_dataset → canonical ChatML     │
                        │       │                                           │
                        │       ▼                                           │
                        │  training_submit → Together/Runpod/Baseten        │
                        │       │         (axolotl / unsloth harness)       │
                        │       ▼                                           │
                        │  LoRA adapter (StyleReplication domain)           │
                        │       │                                           │
                        │       ▼                                           │
                        │  training_deploy → provisioned endpoint           │
                        │       │                                           │
                        │       └─── feeds back into compose pipeline ──────┘
                        │                                                   │
                        └───────────────────────────────────────────────────┘
```

### 1.2 Why This Architecture

The paper from Mavridis et al. (2025) teaches nine lessons. Three are directly applied here:

| Lesson | Paper Finding | hKask Application |
|--------|--------------|-------------------|
| **RAG pattern**: vectors retrieve, LLMs reason | Embedding-only (BERTMap): 57.93% F1. Embedding+LLM (GPT-4o): 96.26% F1 | Replica already does this (KNN → generation). Adding classifier evaluation adds a second LLM reasoning step for quality assessment. |
| **Confidence scoring on every output** | LLMs output confidence alongside mapping decisions; low confidence → human review | Classifier evaluates style fidelity with 0.0–1.0 score. Routes by threshold (pattern already established in `qa-triage.yaml`: ≥0.95 → auto, <0.70 → human). |
| **Domain-specific evaluation model** | BioBERT for biomedical text; general embeddings insufficient | Classifier model (`qwen3-235b-a22b`) for style evaluation, not the generation model. Separates creative generation from structured judgment. |

---

## 2. Phased Implementation

### 2.1 Phase 1 — Informational Confidence (Observer Mode)

**Goal:** Add confidence scoring as a visible, non-gating signal. Gather data on centroid-vs-confidence correlation before making gating decisions.

**Principle anchoring:** P3 (Generative Space — surface the signal, don't hide it), P5 (Essentialism — minimal change, reuse classifier infrastructure), P9 (Homeostatic Self-Regulation — CNS span for evaluation cost)

**Changes (estimated ~40 lines across 3 files):**

| File | Change | Lines |
|------|--------|-------|
| `registry/classify/style-evaluator.yaml` | New classifier config: model, prompt, thresholds | ~30 |
| `crates/hkask-services-compose/src/` | +`StyleConfidence` struct, +evaluator call via `classify_batch` (single item), wire into `ComposeResult` | ~25 |
| `mcp-servers/hkask-mcp-replica/src/lib.rs` | +`llm_confidence`, +`confidence_reasoning` fields in `ComposeResult` | ~6 |

**New classifier config** (`registry/classify/style-evaluator.yaml`):

```yaml
# Style Evaluator — Classifier for replica prose quality assessment
# Used by: ComposeService::compose() (optional, gated by validation.llm_evaluate config)
# Model: qwen3-235b-a22b-2507 (22B active MoE, via KiloCode)
# Pattern: Follows qa-triage.yaml — structured JSON response with confidence + reasoning
# Confidence routing (enforced in Rust, not in prompt):
#   ≥ 0.85 → high quality (training candidate)
#   0.50–0.84 → moderate (needs human review if centroid also borderline)
#   < 0.50 → off-style (training negative example)

classifier:
  name: style-evaluator
  model: qwen/qwen3-235b-a22b-2507
  provider: kilocode
  concurrency: 1
  timeout_secs: 15

  system_prompt: >
    You are a literary style evaluator. Given exemplar passages from {author}
    and a piece of generated prose, rate how well the prose matches {author}'s
    authentic style. Return ONLY valid JSON, no commentary.

    Evaluation dimensions:
    - Rhythm: sentence length variation, cadence, musicality
    - Syntax: paratactic vs. hypotactic, clause structure, punctuation patterns
    - Diction: vocabulary choice, register, concreteness vs. abstraction
    - Tone: emotional register, narrative distance, irony/sincerity
    - Voice: narrative perspective, characteristic phrasing, signature devices

    Return: {"confidence": 0.0-1.0, "reasoning": "one sentence explaining the rating",
             "dimension_scores": {"rhythm": 0.0-1.0, "syntax": 0.0-1.0,
                                  "diction": 0.0-1.0, "tone": 0.0-1.0,
                                  "voice": 0.0-1.0}}

    {author} is known for {style_brief}. Score strictly against this reference.

  base_url: https://api.kilo.ai/api/gateway/chat/completions
  api_key_env: KC_API_KEY

  temperature: 0.0
  max_tokens: 256

  fallback_category: unevaluated
```

**New response type** (add to `compose.rs`):

```rust
/// LLM self-assessed style confidence — complements centroid distance
/// with a semantic quality signal. Per Mavridis et al. (2025), combining
/// both signals catches failures that either misses alone.
#[derive(Debug, Clone, Serialize)]
pub struct StyleConfidence {
    /// 0.0 (completely off-style) to 1.0 (perfect match).
    pub score: f64,
    /// Per-dimension scores: rhythm, syntax, diction, tone, voice.
    pub dimensions: Option<HashMap<String, f64>>,
    /// Brief explanation of the rating from the evaluator.
    pub reasoning: String,
    /// Model used for evaluation.
    pub evaluator_model: String,
}
```

**Pipeline insertion point** — after generation (step 6), before centroid validation (step 7), gated behind `validation.llm_evaluate`:

```rust
// Step 6b: LLM style confidence evaluation (gated by config)
let style_confidence = match &request.cognition.validation.llm_evaluate {
    Some(_eval_cfg) => {
        let passages = vec![evaluator_prompt_assembled];
        let classifier_cfg = ClassifierConfig::from_def(
            &load_classifier_config("style-evaluator", registry_dir)?
        );
        let results = classify_batch(&passages, classifier_cfg, None).await?;
        // Parse first result's JSON into StyleConfidence
        // ...
    }
    None => None,
};
```

**Success criteria (Phase 1):**

- [ ] `replica_compose` MCP response includes `llm_confidence` and `confidence_reasoning` fields when `validation.llm_evaluate` is configured
- [ ] CNS span emitted for each evaluation call (`cns.classify` domain, style-evaluator classifier)
- [ ] Evaluation cost tracked in gas budget (via existing classifier cost accounting)
- [ ] 20+ compose calls logged with both confidence and centroid distance for correlation analysis
- [ ] No change to existing pass/fail behavior — confidence is informational only

### 2.2 Phase 2 — Gated Quality (AND-Gate + Routing)

**Goal:** After Phase 1 data shows confidence scores correlate with centroid distance (or reveals where they disagree), add configurable quality gating and route outputs to the training pipeline.

**Principle anchoring:** P9 (Homeostatic Self-Regulation — quality gate is a cybernetic feedback mechanism), P4 (Clear Boundaries — gating thresholds are user-configurable, not hidden), P8 (Semantic Grounding — `StyleReplication` domain anchors adapter identity)

**Changes:**

| File | Change |
|------|--------|
| `crates/hkask-services-compose/src/` | Add `confidence_min` threshold to `EvaluateSection`; combine with centroid in pass/fail |
| `crates/hkask-adapter/src/expertise.rs` | Add `StyleReplication` variant to `MdsDomain` |
| `mcp-servers/hkask-mcp-replica/src/lib.rs` | Add `training_ingest_qa` call path for high-confidence outputs; CNS spans for training routing |

**Quality routing logic:**

```
Phase 1 data analysis informs thresholds. Default starting point:

  confidence ≥ 0.80 AND centroid_distance ≤ 0.30
    → PASS + route to training_ingest_qa (positive example)

  confidence < 0.50 OR centroid_distance > 0.50
    → FAIL + route to training_curate_feedback (negative example)

  everything else
    → PASS (borderline) — informational only, no training routing
```

**`EvaluateSection` config extension:**

```yaml
validation:
  centroid_distance_max: 0.35
  llm_evaluate:
    confidence_min: 0.80          # below this → fail regardless of centroid
    centroid_confidence_and: true # both must pass (AND-gate)
    route_to_training: true       # high-confidence outputs → training_ingest_qa
```

**`MdsDomain` addition** (`crates/hkask-adapter/src/expertise.rs`):

```rust
pub enum MdsDomain {
    // ... existing variants ...
    SecurityAnalysis,
    /// Authorial style replication — LoRA adapter trained on
    /// (prompt, generated_prose) pairs from successful compose calls.
    StyleReplication,
}
```

**Success criteria (Phase 2):**

- [ ] `style_passed` in MCP response reflects both centroid AND confidence thresholds
- [ ] High-confidence outputs automatically populate `training_ingest_qa` with `source = "replica-compose"`, `bloom_level = "creating"`
- [ ] Low-confidence outputs appear in `training_curate_feedback` for human review
- [ ] `training_record_invocation` calls include confidence scores for continuous training tracking
- [ ] CNS spans distinguish evaluation-routed outcomes from generation outcomes

### 2.3 Phase 3 — Closed Training Loop

**Goal:** After accumulating sufficient training data (target: 500+ high-confidence QA pairs per author), train, deploy, and hot-swap a LoRA adapter into the compose pipeline.

**Principle anchoring:** P3 (Generative Space — adapter is user-visible and user-controllable), P9 (Homeostatic Self-Regulation — training cost tracked in gas budget), P12 (Affirmative Consent — adapter ownership and deployment require explicit consent)

**Changes:**

| File | Change |
|------|--------|
| `mcp-servers/hkask-mcp-training/src/lib.rs` | New `TrainingMode::StyleReplication` or reuse `Expertise` mode with `StyleReplication` domain |
| `registry/training/style-replication.yaml` | Axolotl config template for style adapter training |
| `crates/hkask-services-compose/src/` | Adapter-aware generation: if deployed adapter exists for author, route through it instead of raw generation model |
| `mcp-servers/hkask-mcp-replica/src/lib.rs` | `replica_build` optionally triggers training when corpus embedding completes |

**Training pipeline:**

```
1. Curator accumulates 500+ QA pairs for "hemingway"
   → Source: replica_compose calls with confidence ≥ 0.80
   → Format: QaItem { question: user_prompt, answer: generated_prose, bloom_level: "creating" }

2. training_assemble_dataset(source="hemingway")
   → DatasetPipeline normalizes to ChatML JSONL
   → ChatMessage { role: "user", content: prompt }
   → ChatMessage { role: "assistant", content: prose }

3. training_submit(
     base_model="meta-llama/Llama-3.3-70B-Instruct",
     adapter_name="hemingway-style-v1",
     expertise="StyleReplication",
     host="together"
   )
   → HarnessAdapter dispatches to axolotl
   → LoRA rank 16, alpha 32, target_modules=["q_proj","v_proj"]

4. training_deploy(adapter_name="hemingway-style-v1", provider="together")
   → AdapterRouter provisions endpoint
   → EndpointLifecycle: Provisioning → Ready → Active

5. replica_compose routes through deployed adapter
   → If adapter exists for author → inference via adapter endpoint
   → If no adapter → fallback to raw generation model + prompt engineering
```

**Success criteria (Phase 3):**

- [ ] At least one author has 500+ high-confidence QA pairs in semantic memory
- [ ] `training_submit` produces a LoRA adapter with `StyleReplication` expertise domain
- [ ] `training_deploy` provisions an active endpoint
- [ ] `replica_compose` detects deployed adapter and routes generation through it
- [ ] Centroid distance improves on adapter-generated prose vs. prompt-only generation (measured over 20+ calls)
- [ ] Adapter cost tracked per invocation in CNS gas budget

---

## 3. Architectural Integration

### 3.1 Crate Touch Points

```
┌──────────────────────────────────────────────────────────────────┐
│                        LAYER MAP                                 │
│                                                                  │
│  MCP SURFACE                                                     │
│  mcp-servers/hkask-mcp-replica/                                  │
│    └─ ComposeResult extended (Phase 1)                           │
│    └─ training_ingest_qa routing (Phase 2)                       │
│    └─ adapter-aware generation routing (Phase 3)                 │
│                                                                  │
│  SERVICE LAYER                                                   │
│  crates/hkask-services-compose/src/                              │
│    └─ StyleConfidence type (Phase 1)                             │
│    └─ Classifier evaluator call (Phase 1)                        │
│    └─ AND-gate pass/fail logic (Phase 2)                         │
│    └─ Adapter dispatch for generation (Phase 3)                  │
│                                                                  │
│  CLASSIFIER INFRASTRUCTURE (reused — no new code)                │
│  crates/hkask-services-runtime/src/classify_impl.rs              │
│    └─ classify_batch (single-item) → already exists              │
│  registry/classify/style-evaluator.yaml                          │
│    └─ New config — follows qa-triage.yaml pattern                │
│                                                                  │
│  ADAPTER LIFECYCLE (reused — 1 new enum variant)                 │
│  crates/hkask-adapter/src/expertise.rs                           │
│    └─ MdsDomain::StyleReplication (Phase 2)                      │
│  crates/hkask-adapter/src/adapter_router/                        │
│    └─ Existing: provision, deploy, teardown                      │
│                                                                  │
│  TRAINING PIPELINE (reused — no new code)                        │
│  mcp-servers/hkask-mcp-training/src/                             │
│    └─ training_ingest_qa → already accepts QaItem                │
│    └─ training_assemble_dataset → already produces ChatML        │
│    └─ training_submit → already dispatches to TrainingHost       │
│    └─ training_deploy → already provisions endpoints             │
│    └─ training_record_invocation → already takes confidence      │
│                                                                  │
│  STORAGE                                                         │
│  crates/hkask-storage/src/embeddings.rs                          │
│    └─ sqlite-vec KNN search → unchanged, already works           │
│                                                                  │
└──────────────────────────────────────────────────────────────────┘
```

### 3.2 Principle Compliance

| Principle | How This Plan Satisfies It |
|-----------|---------------------------|
| **P1 — User Sovereignty** | Adapter training data is user-owned (lives in per-pod `pod.db`). Training artifacts (LoRA weights) are user-exportable via backup. |
| **P2 — Affirmative Consent** | `training_submit` and `training_deploy` are explicit user actions through OCAP-gated MCP tools. No automatic training without consent. |
| **P3 — Generative Space** | All thresholds (`confidence_min`, `centroid_distance_max`, routing decisions) are user-configurable in YAML. No hidden settings. |
| **P4 — Clear Boundaries** | Adapter deployment is OCAP-gated through `DelegationToken`. The MCP server (replica) does not directly call training — it routes through governed tools. |
| **P5 — Essentialism** | Phase 1 is ~40 lines. Reuses classifier infrastructure (`classify_batch`, `ClassifierConfig`, YAML registry) instead of building parallel evaluator. One new enum variant (`StyleReplication`). No new crates. |
| **P8 — Semantic Grounding** | `StyleReplication` domain grounds adapter identity. `StyleConfidence` struct carries provenance (evaluator model, dimensions). Classifier config anchors the evaluation prompt in the registry. |
| **P9 — Homeostatic Self-Regulation** | Evaluation cost tracked per call (classifier cost accounting). Training cost tracked per job. CNS spans distinguish generation, evaluation, and training routing. |
| **P12 — Replicant Host Mandate** | Every training action attributed to a replicant. `training_record_invocation` carries replicant identity. Adapter ownership tracked via `owner_webid`. |

### 3.3 CNS Span Integration

| Span | Domain | When Emitted |
|------|--------|-------------|
| `cns.classify` (existing) | Style evaluation call | Each evaluator inference |
| `cns.mcp.replica` (existing) | Replica compose | Extended with `llm_confidence` field |
| `cns.mcp.training` (existing) | Training ingest/submit/deploy | Triggered by routing decisions |
| `cns.adapter` (new) | Adapter dispatch | When compose routes through deployed adapter |

---

## 4. Dependencies & Prerequisites

### 4.1 What Must Exist First

| Prerequisite | Status | Notes |
|-------------|--------|-------|
| `classify_batch` supports single-item calls | ✅ Exists | Already used in `embed_corpus` triple extraction |
| `ClassifierConfig::from_def()` resolves provider + API key | ✅ Exists | KiloCode provider already configured for `triple-extractor-literary` |
| `training_ingest_qa` accepts arbitrary QA pairs | ✅ Exists | `QaItem { question, answer, bloom_level }` — no source restrictions |
| `training_record_invocation` accepts confidence field | ✅ Exists | `TrainRecordInvocationRequest { confidence, success, ... }` |
| `AdapterRouter` provisions endpoints for LoRA adapters | ✅ Exists | Together, Runpod, Baseten backends all functional |
| `MdsDomain` enum is extensible | ✅ Exists | Adding a variant is non-breaking |
| `ComposeResult` is extensible | ✅ Exists | Adding `Option<StyleConfidence>` is backward-compatible |

### 4.2 What Does NOT Exist Yet

| Gap | Phase | Resolution |
|-----|-------|-----------|
| `registry/classify/style-evaluator.yaml` | Phase 1 | New file — follows `qa-triage.yaml` pattern |
| `StyleConfidence` type | Phase 1 | New struct in `compose.rs` |
| Evaluator prompt assembly with author-specific style brief | Phase 1 | Template function in `compose.rs` — author style brief comes from corpus config |
| Correlation data (centroid vs. confidence) | Phase 1→2 | Gathered during Phase 1 observation period |
| `EvaluateSection` config with gating thresholds | Phase 2 | Extends existing `ValidationSection` |
| Routing logic (confidence → training) | Phase 2 | New function in replica server |
| `StyleReplication` domain variant | Phase 2 | One-line addition to `MdsDomain` |
| Axolotl config for style adapter training | Phase 3 | New template in `registry/training/` |
| Adapter-aware generation dispatch in compose pipeline | Phase 3 | New conditional in `ComposeService::compose()` |
| 500+ QA pairs per author for training dataset | Phase 3 | Accumulated during Phase 1–2 usage |

---

## 5. Risk & Mitigation

| Risk | Likelihood | Impact | Mitigation |
|------|-----------|--------|-----------|
| Classifier model cannot distinguish good/bad style imitation | Medium | High — uncalibrated scores → bad training data | Phase 1 gathers correlation data before gating. Test with known good/bad Hemingway passages. |
| Confidence and centroid distance are uncorrelated | Low-Medium | Medium — AND-gate rejects everything or passes everything | Phase 1 observes correlation. If uncorrelated, keep signals separate (OR-gate or weighted composite). |
| Training on self-assessed outputs amplifies LLM biases | Medium | High — adapter learns to satisfy classifier, not actual style | Curate feedback manually for first training run. Human review of 50 random samples before training. |
| Cost of evaluation doubles inference spend per compose call | Low | Medium — classifier model is cheaper than generation model | Track cost in CNS. Classifier model ($0.03/1M tokens input, ~256 tokens output) costs ~$0.008 per evaluation. Generation model costs 10–100x more. |
| Not enough training data accumulates naturally | Medium | Low — delays Phase 3 | Synthetic data generation: run compose with diverse prompts across temperature range, keep high-confidence outputs. |
| Adapter deployment cost exceeds budget | Low | Medium — Together AI endpoints cost ~$1–3/hour | Tear down endpoint when idle. `EndpointLifecycle` already supports Draining → Terminated. |
| Author style briefs are inconsistent or missing | Medium | Low — evaluator prompt degrades without style reference | Style brief derived from corpus config `foundational_rules`. Fallback to generic "match exemplar style" prompt if no rules declared. |

---

## 6. Open Questions

| # | Question | Resolution Needed By |
|---|----------|---------------------|
| OQ-1 | Should the evaluator re-embed the *exemplar passages* or just the generated prose? Paper re-embeds source concepts. For style, embedding the generated prose + comparing to author centroid is the existing pattern. The evaluator is semantic, not geometric — it works from text, not vectors. | Phase 1 implementation |
| OQ-2 | Should `StyleReplication` use `TrainingMode::Expertise` (QA pairs) or a new mode? QA pairs fit naturally: prompt → prose. No new mode needed. But `DecompositionTrace` mode could also apply if we want the adapter to *reason* about style before writing. | Phase 3 design |
| OQ-3 | Should adapter be per-author or multi-author? Per-author is simpler and matches the corpus structure. Multi-author would require training on cross-author QA pairs and the adapter would need author metadata in the prompt. | Phase 3 design |
| OQ-4 | Does the classifier model need a dedicated style evaluation fine-tune before it can reliably evaluate? The paper used domain-specific embeddings (BioBERT). A style-specific classifier might need similar domain adaptation. Without it, confidence scores may be noisy. | Phase 1 data analysis |

---

## 7. References

- Mavridis, A., Tegos, S., Anastasiou, C., Papoutsoglou, M. C., & Meditskos, G. (2025). Large language models for intelligent RDF knowledge graph construction: results from medical ontology mapping. *Frontiers in Artificial Intelligence*, 8, 1546179. doi:10.3389/frai.2025.1546179
- ADR-035: Replicant Server Mode — AgentMode, Daemon Transport, Dual Memory Encoding
- `docs/architecture/core/PRINCIPLES.md` — P1–P12 architecture principles
- `docs/architecture/core/FUNCTIONAL_SPECIFICATION.md` — Domain-to-contract mapping
- `docs/architecture/hKask-architecture-master.md` — Service layer, daemon, ACP replicant patterns
- `registry/classify/qa-triage.yaml` — Confidence routing pattern (≥0.95 → auto-repair, <0.70 → human)
- `registry/classify/triple-extractor-literary.yaml` — Literary feature extraction classifier (reference for style evaluation prompt design)
