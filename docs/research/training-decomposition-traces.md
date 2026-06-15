---
title: "Training Decomposition Traces"
audience: [architects, developers, agents]
last_updated: 2026-06-15
version: "0.27.0"
status: "Active"
domain: "Training"
mds_categories: [domain, composition, lifecycle, curation]
---

# hKask Agent Skill Training: Decomposition Traces, LoRA Adapters, and the Future of Fine-Tuning

**Status:** Research & Architecture Document  
**Date:** 2026-06-15  
**Version:** v0.27.0 context  
**Related:** `hkask-mcp-training`, constraint-forces skill, Together AI integration

---

## 1. What We've Been Doing: Decomposition Traces as Training Data

### 1.1 The Problem

hKask agents need to internalize skills — not just retrieve facts from documents, but execute procedural reasoning: classify constraints, diagnose failures, apply the essentialist deletion test, run TDD red-green-refactor loops. These are **indirect knowledge** tasks: the agent must decompose a situation into sub-questions, apply a decision framework, and synthesize a conclusion.

Traditional QA pairs ("Q: What is a Prohibition? A: An inviolable rule") train factual recall but don't teach the **process of getting to the right question**. An agent trained on QA pairs can recite definitions but can't walk the classification decision tree when faced with an ambiguous constraint like "AES-256-GCM encryption is required."

### 1.2 The Solution: Decomposition Traces

A **decomposition trace** is a structured reasoning record that captures the full decision process:

```
Situation → Sub-questions → Decision tree walk → Synthesis → Implications
```

Each trace teaches the model **how to think**, not just what to answer. The trace format we use is ChatML (the standard for fine-tuning chat models):

```json
{
  "messages": [
    {"role": "system", "content": "You are an hKask agent trained in constraint-forces..."},
    {"role": "user", "content": "Classify: 'The database must use AES-256-GCM encryption...'"},
    {"role": "assistant", "content": "Decision tree:\n\n**Step 1: Statement about the system?** Yes...\n**Step 4: Classification: Prohibition (Rank 1).**"}
  ]
}
```

The assistant's response walks through the **decision tree explicitly** — each step labeled, each criterion checked, the final classification justified. This is not a one-word answer; it's a **reasoning trace** that teaches the model the methodology.

### 1.3 What We Built

**Training pipeline** (`hkask-mcp-training`, 8 MCP tools):

| Tool | Purpose |
|------|---------|
| `training_generate_traces` | Use inference to generate decomposition traces from skill documents |
| `training_assemble_dataset` | Query stored triples, write ChatML JSONL |
| `training_submit` | Upload dataset + submit fine-tuning job to provider |
| `training_status` / `training_cancel` | Job lifecycle management |
| `training_list_adapters` / `training_delete_adapter` | Adapter inventory |
| `training_ingest_qa` | Store QA pairs in semantic memory (for docproc-derived data) |

**Provider surface:** Together AI (cloud), Axolotl (local/cloud), Unsloth (local). Together AI is our primary provider — cloud fine-tuning + dedicated endpoint deployment, no local GPU required.

### 1.4 Results: constraint-forces Skill

| Iteration | Traces | Accuracy | Weak Spots |
|-----------|--------|----------|------------|
| v1 (baseline) | 25 basic | 80% (8/10) | Guardrail vs Evidence confusion; Prohibition with technical details misclassified |
| v2 (targeted) | 25 edge-case | **100% (20/20)** | All fixed |

**v1 weak spots:**
- "Variety deficit > 100 triggers alert" → classified as **Evidence** instead of **Guardrail** (the number distracted from the threshold+consequence pattern)
- "AES-256-GCM encryption required" → classified as "Policy" instead of **Prohibition** (technical jargon obscured the "must" force language)

**v2 targeted traces addressed:**
- Guardrail vs Evidence: paired examples with identical numbers, differing only in presence/absence of a triggered consequence
- Prohibition with technical details: "must"/"never" buried in crypto jargon, protocol names, platform APIs
- Guardrail vs Prohibition: identical thresholds, differing only in presence/absence of a consent-gated override path
- Guideline vs Guardrail: identical domains, differing only in presence/absence of enforcement triggers

**Training cost:** ~$0.005 per run, ~4-7 minutes on Together AI, 11,854 tokens.

---

## 2. Where the Adapter Lives and How It's Applied

### 2.1 Adapter Lifecycle

```
Skill document (SKILL.md)
    ↓ training_generate_traces (inference-powered)
Decomposition traces (JSONL)
    ↓ training_submit (Together AI API)
LoRA adapter (trained weights)
    ↓ Deploy to dedicated endpoint
Inference endpoint (serving)
    ↓ hKask inference router (TG/ prefix)
Agent session (kask chat)
```

### 2.2 Storage & Registry

Trained adapters are stored in two places:

1. **Together AI model registry** — `mdz_7e9b/Qwen3.5-9B-hkask-{skill}-{hash}`. This is the canonical storage. Adapters are downloadable as safetensors files.

2. **hKask adapter store** (`hkask-mcp-training`'s `AdapterStore`) — in-memory registry tracking adapter IDs, base models, associated skills, and evaluation scores. Persisted to the training server's state.

### 2.3 Inference Routing

The hKask inference router (`hkask-inference`) already supports provider-prefixed model names:

```
TG/mdz_7e9b/Qwen3.5-9B-hkask-cf-v2-aaa51b20
```

When an agent session needs a specific skill, the router selects the appropriate adapter. The architecture supports **multi-adapter serving** — multiple LoRA adapters sharing one base model (Qwen3.5-9B), loaded on demand.

**Current state:** One adapter per dedicated endpoint (Together AI's current model).  
**Target state:** Multi-LoRA serving — single endpoint serving multiple skill adapters, selected at inference time by the router.

### 2.4 Application Flow

```
User: "Classify this constraint..."
    ↓
hKask inference router
    ├── Detects: constraint classification task
    ├── Selects: constraint-forces adapter
    ├── Routes: TG/mdz_7e9b/Qwen3.5-9B-hkask-cf-v2
    └── Returns: classified response with decision tree walk
```

The adapter doesn't replace the base model — it **specializes** it. The base model (Qwen3.5-9B) provides general language understanding; the LoRA adapter adds the constraint-forces decision methodology. Together they produce responses that follow the skill's prescribed reasoning pattern.

### 2.5 Future: Adapter Composition

As we train adapters for more skills, we'll need **adapter composition** — combining multiple skill adapters at inference time. Research directions:

- **Adapter merging:** Linear combination of LoRA weights (weighted sum of A/B matrices). Simple but can produce interference between skills.
- **Adapter switching:** Route to the appropriate adapter based on task classification. Clean separation but can't handle cross-skill tasks.
- **Mixture-of-adapters:** Learn a routing function that selects which adapter(s) to activate per token or per layer. Most flexible but requires additional training.

---

## 3. Research Landscape: Fine-Tuning, LoRA, and the Future

### 3.1 Foundational Papers

| Paper | Year | Key Contribution |
|-------|------|-----------------|
| **LoRA: Low-Rank Adaptation of Large Language Models** (Hu et al., 2022) | 2022 | Introduced LoRA — injects trainable low-rank decomposition matrices into frozen weights. The foundation of all modern PEFT. |
| **QLoRA: Efficient Finetuning of Quantized LLMs** (Dettmers et al., 2023) | 2023 | Combined LoRA with 4-bit quantization — made fine-tuning of 65B models possible on a single 48GB GPU. |
| **DoRA: Weight-Decomposed Low-Rank Adaptation** (Liu et al., 2024) | 2024 | Decomposes pre-trained weights into magnitude and direction, applying LoRA only to direction. Improves learning capacity without increasing rank. |
| **S-LoRA: Serving Thousands of Concurrent LoRA Adapters** (Sheng et al., 2023) | 2023 | First system to demonstrate multi-adapter serving at scale — unified batching, adapter clustering, CPU-GPU memory management. |

### 3.2 Current Research Directions (2024–2025)

#### A. LoRA Rank Optimization

**Key question:** What rank is sufficient? Higher rank = more capacity but more parameters.

- **"How Much is Too Much?"** (AACL IJCNLP 2025): Systematic rank sweep across reasoning and recall datasets. Found that rank-performance trade-off is task-dependent — reasoning tasks benefit from higher ranks more than knowledge-recall tasks.
- **"Learning Rate Matters: Vanilla LoRA May Suffice"** (2025): Demonstrated that careful learning rate tuning can close the gap between vanilla LoRA and advanced variants. Optimal LR is contingent on both base model and target task.
- **"LoRA Learns Less and Forgets Less"** (Biderman et al., 2024): LoRA substantially underperforms full fine-tuning on coding and math, but better preserves base model capabilities (less catastrophic forgetting).

**Implication for hKask:** Our classification tasks (constraint-forces, pragmatic-semantics) are closer to knowledge-recall than complex reasoning. Lower ranks (r=16–64) are likely sufficient. Together AI's default r=64 with alpha=128 worked well for our 25-trace dataset.

#### B. Adapter Serving at Scale

**Key question:** How to serve hundreds of adapters on shared infrastructure?

- **LoRAServe** (Jaiswal et al., Nov 2025): Workload-aware dynamic adapter placement and routing. Analyzed production traces from real LoRA deployments. Key insight: adapter popularity follows power-law distribution — a few adapters get most traffic.
- **InfiniLoRA** (2025): Disaggregated multi-LoRA serving — separates prefill and decode phases, enabling independent scaling of adapter computation.
- **EdgeLoRA** (MobiSys 2025): Multi-tenant LoRA serving on edge devices. Adaptive adapter selection, heterogeneous memory management, batch LoRA inference.
- **Together AI Multi-LoRA** (production, 2025): Serverless multi-LoRA with Cross-LoRA Continuous Batching and Adapter Prefetching. Pay-per-token pricing. Used by Salesforce, Zomato, Washington Post.

**Implication for hKask:** As we train adapters for 10+ skills, we'll need multi-adapter serving. Together AI's serverless multi-LoRA is the natural path — single base model, multiple adapters, pay-per-token.

#### C. Reasoning Distillation

**Key question:** How to transfer reasoning capabilities from large models to smaller ones via fine-tuning?

- **"Reasoning Scaffolding"** (2025): Distills the "flow of thought" from LLMs — extracts structural patterns from reasoning traces, not just final answers. Uses rationale decomposition and modular architectures.
- **"Distilling the Essence"** (2025): Sequence truncation for efficient reasoning distillation. Shows that long CoT traces can be compressed without losing training signal.
- **Chain-of-Thought Fine-Tuning** (survey, 2025): CoT Collection (1.84M machine-generated rationales), synthetic and distilled traces, difficulty-aware and length-conditioned traces. Human reasoning parallels via "Six Thinking Hats" framework.

**Implication for hKask:** Our decomposition traces are a form of reasoning distillation. We're distilling the constraint-forces decision methodology from the SKILL.md document into the model's weights. The research validates this approach — SFT on high-quality reasoning traces is a proven method for imparting procedural knowledge.

#### D. Agent-Specific Fine-Tuning

**Key question:** How to train models for specific agent roles?

- **"Fine-tuning LLMs for Specific Agent Roles"** (2025): Data acquisition is the biggest bottleneck. Specialized datasets must capture the agent's decision process, not just factual knowledge. Multi-agent systems benefit from distinct adapters per role.
- **"Memento: Fine-tuning LLM Agents without Fine-tuning LLMs"** (2025): Alternative approach — memory-augmented MDP with online soft Q-learning for continual adaptation without parameter updates. Complements LoRA-based approaches.
- **System-2 Fine-Tuning** (Park et al., 2025): Robust integration of new knowledge through deliberate, analytical reasoning patterns — closely aligned with our decomposition trace methodology.

**Implication for hKask:** Our approach — training per-skill LoRA adapters on decomposition traces — sits at the intersection of reasoning distillation and agent-specific fine-tuning. Each skill adapter is a specialized agent capability module.

### 3.3 The Future of Fine-Tuning (2025–2027)

#### Trend 1: From Full Models to Adapter Ecosystems

The industry is moving from "one fine-tuned model per task" to **adapter ecosystems** — hundreds of lightweight LoRA adapters sharing a base model. Key enablers:

- **Serverless multi-LoRA** (Together AI, Predibase, Anyscale): Pay-per-token, no dedicated endpoint management
- **Adapter registries**: Versioned, discoverable, composable
- **Automatic adapter selection**: Task-classification routers that pick the right adapter per request

hKask is architecturally aligned with this trend — our skill adapters are designed to be composed and routed.

#### Trend 2: From Human-Written to Synthetic Training Data

The bottleneck is shifting from compute to **data quality**. Emerging approaches:

- **LLM-as-teacher distillation**: Large models generate reasoning traces; small models learn from them
- **On-policy self-distillation**: Models generate their own training data, filter for quality, retrain
- **Hypernetwork-generated adapters**: D2L (Documents to LoRA) — a hypernetwork maps documents directly to LoRA weights without explicit fine-tuning

hKask's `training_generate_traces` tool is an instance of LLM-as-teacher distillation — we use inference to generate decomposition traces from skill documents.

#### Trend 3: From Static to Continual Adaptation

Models that adapt continuously rather than being trained once:

- **Test-time training**: Adapt adapter weights at inference time based on the specific input
- **Online RL fine-tuning**: Reinforcement learning during agent operation
- **Memory-augmented adaptation**: Learn from experience without weight updates (Memento approach)

hKask's CNS and episodic memory systems provide the infrastructure for continual adaptation — training traces could be generated from agent experiences, not just static skill documents.

#### Trend 4: Adapter Composition and Merging

As adapter counts grow, composition becomes critical:

- **Linear merging**: Weighted average of LoRA weights (simple, fast, but lossy)
- **Task arithmetic**: Add/subtract task vectors in weight space
- **Mixture-of-LoRA**: Learn routing weights per token or per layer
- **AdapterFusion**: Attention-based composition of multiple adapters

hKask will need composition when agents must apply multiple skills simultaneously (e.g., constraint-forces + pragmatic-semantics for a sovereignty audit).

### 3.4 Key Academic References

| Reference | Focus | Relevance to hKask |
|-----------|-------|-------------------|
| Hu et al. (2022) "LoRA: Low-Rank Adaptation" | Foundational PEFT method | Our training infrastructure |
| Sheng et al. (2023) "S-LoRA" | Multi-adapter serving | Future multi-skill deployment |
| Biderman et al. (2024) "LoRA Learns Less and Forgets Less" | LoRA vs full fine-tuning trade-offs | Validates our PEFT choice for skill preservation |
| Liu et al. (2024) "DoRA" | Weight-decomposed adaptation | Potential upgrade path for higher-capability skills |
| Jaiswal et al. (2025) "LoRAServe" | Production multi-adapter serving | Architecture reference for adapter routing |
| "Reasoning Scaffolding" (2025) | Distilling reasoning flow | Validates decomposition trace methodology |
| Park et al. (2025) "System-2 Fine-Tuning" | Deliberate reasoning integration | Aligned with our decision-tree training approach |
| "Chain-of-Thought Fine-Tuning" (survey, 2025) | Comprehensive CoT training survey | Framework for expanding to procedural skills |
| Together AI "Serverless Multi-LoRA" (2025) | Production multi-adapter platform | Our deployment target |

---

## 4. hKask Training Roadmap

### 4.1 Completed

- [x] Training MCP server (14 tools — full skills training surface)
- [x] Together AI provider integration (upload + fine-tune + deploy)
- [x] Runpod provider integration (GPU pod dispatch for axolotl training)
- [x] Decomposition trace generation (`training_generate_traces`) with model override + chunking
- [x] constraint-forces skill adapter (v2, 100% accuracy)
- [x] End-to-end pipeline proven (generate → upload → train → deploy → evaluate)
- [x] `training_evaluate` — automated holdout evaluation (exact/contains/semantic)
- [x] `training_register_adapter` — persistent adapter registry with versioning
- [x] `training_recommend_model` — base model selection guidance
- [x] `training_record_invocation` — episodic invocation recording for continuous loop
- [x] `training_curate_feedback` — LLM-as-judge feedback curation from QA pairs
- [x] `training_retrain` — merge + dedup + retrain with auto-incremented version
- [x] Adapter registry: Versioned adapter store with metadata (skill, base model, version, evaluation scores, training date)
- [x] Job persistence: `training_jobs` table survives server restarts
- [x] Blob storage: Adapter weights stored for local providers (Axolotl/Unsloth)
- [x] PID tracking: Real job cancellation for local providers (SIGTERM)
- [x] Token-length validation in `training_submit`
- [x] System prompt support in `training_assemble_dataset`
- [x] Chunking for large skill documents in `training_generate_traces`

### 4.2 Near-Term (Next 2-4 Weeks)

- [ ] **pragmatic-semantics adapter**: Classification-shaped (IS vs OUGHT, declarative vs probabilistic vs subjunctive). Similar structure to constraint-forces — easy to auto-evaluate.
- [ ] **Multi-skill evaluation harness**: Test adapter composition — can the model apply constraint-forces AND pragmatic-semantics in the same response?

### 4.3 Medium-Term (1-2 Months)

- [ ] **Procedural skill adapters**: `essentialist` (3-gate deletion test), `diagnose` (spec-anchored debugging loop), `tdd` (red-green-refactor). These require more complex traces — multi-step procedures with branching.
- [ ] **Automatic adapter selection**: Inference router detects task type from user query and routes to appropriate adapter.

### 4.4 Long-Term (3-6 Months)

- [ ] **Multi-LoRA serving**: Single Together AI endpoint serving multiple hKask skill adapters. Adapter selected per-request by the router.
- [ ] **Adapter composition**: Merge or route between multiple adapters for cross-skill tasks (e.g., sovereignty audit requiring constraint-forces + pragmatic-semantics + magna-carta-verifier).
- [ ] **Continual adaptation**: Generate training traces from agent experiences (CNS episodic memory) — adapters improve from real usage, not just static documents.
- [ ] **Cross-model adapters**: Train adapters for different base models (Qwen3.5, DeepSeek, Llama 4) — skill portability across inference providers.
- [ ] **`training_monitor_health`** (DEFERRED): Track adapter quality metrics over time (accuracy trend, alert correlation, confidence distribution). Deferred until we have sufficient active usage data to make trends meaningful.
- [ ] **`training_ab_test`** (DEFERRED): Serve multiple adapter versions simultaneously, route fraction of traffic to each, compare outcomes. Deferred until we have multiple adapter versions in active use.

### 4.5 Providers

| Provider | Type | Status | Notes |
|----------|------|--------|-------|
| **Together AI** | Managed fine-tuning API | ✅ Production | Upload → train → deploy → infer. ~$0.005/LoRA run. Primary provider. |
| **Runpod** | GPU pod dispatch | ✅ Implemented | Creates GPU pods from template, dispatches axolotl training. Requires `RUNPOD_API_KEY` + `RUNPOD_TEMPLATE_ID`. |
| **Axolotl** | Local CLI | ✅ Production | YAML-config-driven. PID-tracked cancellation. |
| **Unsloth** | Local Python | ✅ Production | Memory-efficient. PID-tracked cancellation. |

---

## 5. Key Design Decisions

### 5.1 Why Decomposition Traces, Not QA Pairs?

QA pairs train **what** to answer. Decomposition traces train **how** to think. For procedural skills (constraint classification, diagnosis, essentialist review), the methodology is the skill. A model that can recite "Prohibition = Rank 1" but can't walk the decision tree when faced with "AES-256-GCM must be used" hasn't learned the skill.

### 5.2 Why LoRA, Not Full Fine-Tuning?

- **Cost**: ~$0.005 per training run vs $10-100+ for full fine-tuning
- **Speed**: 4-7 minutes vs hours/days
- **Preservation**: Base model capabilities intact — LoRA "learns less and forgets less" (Biderman et al., 2024)
- **Composability**: Multiple adapters share one base model — essential for multi-skill agents
- **Storage**: Adapters are ~1% of model size — 200MB vs 20GB for Qwen3.5-9B

### 5.3 Why Together AI, Not Local Training?

- **No GPU required**: AMD GPU without CUDA makes local training impractical
- **Integrated pipeline**: Upload → fine-tune → deploy → infer, all one API
- **Serverless multi-LoRA**: Future path to serving multiple adapters without managing endpoints
- **Pay-per-use**: No idle endpoint costs when not training

### 5.5 Why Runpod for GPU Dispatch?

- **GPU flexibility**: Choose GPU type per job (RTX 4090, A100, etc.) via `RUNPOD_GPU_TYPE`
- **Template-based**: Pre-built axolotl templates enable one-click training pod creation
- **Complement to Together AI**: Use Runpod when you need specific GPU types, custom training configurations, or want to run axolotl/unsloth in the cloud instead of locally
- **Pod lifecycle**: Pods are created on `submit`, terminated on `cancel`/completion — no idle costs

### 5.6 Why Qwen3.5-9B?

- **Apache 2.0 license**: No usage restrictions
- **Broad provider support**: Unsloth, Axolotl, Together AI all support it
- **Strong base capabilities**: Good general reasoning, instruction following
- **Right size**: 9B parameters — large enough to learn procedural patterns, small enough for fast/cheap fine-tuning

---

## 6. Appendix: Decomposition Trace Format Specification

### 6.1 ChatML Structure

```json
{
  "messages": [
    {
      "role": "system",
      "content": "You are an hKask agent trained in {skill-name}. {skill-instructions}"
    },
    {
      "role": "user",
      "content": "{situation or classification request}"
    },
    {
      "role": "assistant",
      "content": "{step-by-step reasoning trace with explicit methodology}"
    }
  ]
}
```

### 6.2 Trace Quality Criteria

1. **Explicit methodology**: Each step labeled (Step 1, Step 2...), each criterion checked
2. **Decision tree visible**: The reader can follow the reasoning path
3. **Justification, not just answer**: Why this classification, not just what classification
4. **Implications stated**: What does this classification mean for enforcement?
5. **Key distinctions highlighted**: "Key distinction from X: ..." — teaches boundary cases
6. **Edge cases covered**: Ambiguous situations, "when uncertain" rule application

### 6.3 Generation Pipeline

```
Skill document (SKILL.md)
    ↓
training_generate_traces tool
    ├── Reads skill document
    ├── Extracts decision methodology
    ├── Generates diverse situations (basic + edge cases)
    ├── Produces step-by-step reasoning traces
    └── Writes ChatML JSONL
    ↓
training_assemble_dataset (optional — for docproc-derived QA)
    ↓
training_submit → Together AI fine-tuning API
    ↓
LoRA adapter
```

---

## 7. Training & Deployment Workflow: From Skill to Serving

### 7.1 End-to-End Process (Current)

This is the workflow we execute today for each skill adapter:

```
┌─────────────────────────────────────────────────────────┐
│ PHASE 1: TRACE GENERATION                               │
│                                                         │
│  Skill document (SKILL.md)                              │
│      ↓                                                  │
│  training_generate_traces                                │
│      ├── Extract decision methodology from doc          │
│      ├── Generate basic traces (25-50)                  │
│      ├── Generate edge-case traces (25-50)              │
│      │   ├── Boundary confusions (e.g., Guardrail vs    │
│      │   │   Evidence)                                  │
│      │   ├── Technical detail distractions             │
│      │   ├── Ambiguous language cases                   │
│      │   └── Conflict resolution scenarios             │
│      └── Write ChatML JSONL to data/{skill}-traces.jsonl│
│                                                         │
│  Output: 50-100 decomposition traces in ChatML format   │
└─────────────────────────────────────────────────────────┘
                          ↓
┌─────────────────────────────────────────────────────────┐
│ PHASE 2: INITIAL TRAINING                               │
│                                                         │
│  training_submit                                        │
│      ├── Upload JSONL to Together AI files API          │
│      ├── Submit fine-tuning job:                        │
│      │   model: Qwen/Qwen3.5-9B                         │
│      │   epochs: 3                                      │
│      │   lora_r: 16-64 (task-dependent)                 │
│      │   lora_alpha: 32-128                             │
│      │   learning_rate: 2e-4                            │
│      │   batch_size: 8                                  │
│      ├── Poll job status until completed                │
│      └── Record adapter ID in AdapterStore              │
│                                                         │
│  Output: LoRA adapter (safetensors, ~200MB)             │
│  Time: ~4-7 minutes | Cost: ~$0.005                     │
└─────────────────────────────────────────────────────────┘
                          ↓
┌─────────────────────────────────────────────────────────┐
│ PHASE 3: EVALUATION                                     │
│                                                         │
│  Deploy adapter to dedicated endpoint                   │
│      ↓                                                  │
│  Run evaluation suite:                                  │
│      ├── Basic classification tests (10 items)          │
│      ├── Edge-case tests (10 items)                     │
│      ├── Conflict resolution tests (5 items)            │
│      └── Adversarial/distractor tests (5 items)         │
│      ↓                                                  │
│  Score accuracy per category                            │
│      ↓                                                  │
│  Decision gate:                                         │
│      ├── ≥90% accuracy → promote to production          │
│      ├── 70-89% → identify weak spots, generate more    │
│      │            targeted traces, retrain              │
│      └── <70% → review trace quality, methodology       │
│                                                         │
│  Output: Evaluation report + adapter readiness status   │
└─────────────────────────────────────────────────────────┘
                          ↓
┌─────────────────────────────────────────────────────────┐
│ PHASE 4: DEPLOYMENT                                     │
│                                                         │
│  Register adapter in hKask inference router:            │
│      adapter_id: "hkask-cf-v2"                          │
│      model_name: "mdz_7e9b/Qwen3.5-9B-hkask-cf-v2-..." │
│      skill: "constraint-forces"                         │
│      base_model: "Qwen3.5-9B"                           │
│      evaluation_score: 1.0                              │
│      status: "production"                               │
│      ↓                                                  │
│  Deploy to serving endpoint:                            │
│      ├── Current: dedicated endpoint per adapter        │
│      └── Future: multi-LoRA endpoint (shared base)      │
│      ↓                                                  │
│  Router configuration:                                  │
│      skill → adapter mapping                            │
│      fallback → base model (no adapter)                 │
│                                                         │
│  Output: Live adapter serving requests                  │
└─────────────────────────────────────────────────────────┘
```

### 7.2 Continuous Training Loop (Future)

Once an adapter is in production, agent usage generates data that can improve the adapter. This is the **continuous training loop** — a closed cycle where system operation produces training signal.

```
┌──────────────────────────────────────────────────────────────┐
│ CONTINUOUS TRAINING LOOP                                     │
│                                                              │
│  ┌──────────────────────────────────────────────────────┐    │
│  │ DATA GENERATION (ongoing, automatic)                 │    │
│  │                                                      │    │
│  │  Source 1: CNS Feedback Signals                      │    │
│  │  ┌─────────────────────────────────────────────┐    │    │
│  │  │ • Algedonic alerts: when does the adapter    │    │    │
│  │  │   produce responses that trigger CNS warnings│    │    │
│  │  │   (e.g., variety deficit after classification│    │    │
│  │  │   suggests the model is stuck/repeating)     │    │    │
│  │  │ • Span anomalies: cns.cybernetics.backpressure│    │    │
│  │  │   events correlated with adapter usage       │    │    │
│  │  │ • Confidence tracking: when does the model  │    │    │
│  │  │   produce low-confidence classifications?   │    │    │
│  │  └─────────────────────────────────────────────┘    │    │
│  │                                                      │    │
│  │  Source 2: Episodic Memory                            │    │
│  │  ┌─────────────────────────────────────────────┐    │    │
│  │  │ • Record every adapter invocation as an      │    │    │
│  │  │   experience: {query, adapter, response,     │    │    │
│  │  │   user_feedback, cns_spans}                  │    │    │
│  │  │ • User corrections: when a user overrides or │    │    │
│  │  │   rejects the adapter's classification,     │    │    │
│  │  │   capture the correction as a training pair  │    │    │
│  │  │ • Curator escalations: when the Curation     │    │    │
│  │  │   Loop flags an adapter response for review  │    │    │
│  │  └─────────────────────────────────────────────┘    │    │
│  │                                                      │    │
│  │  Source 3: Semantic Memory                            │    │
│  │  ┌─────────────────────────────────────────────┐    │    │
│  │  │ • Query triples tagged with skill domain     │    │    │
│  │  │ • Extract new edge cases from related        │    │    │
│  │  │   knowledge (e.g., new constraint patterns   │    │    │
│  │  │   discovered in other skills' documents)     │    │    │
│  │  │ • Consolidation pipeline: as episodic         │    │    │
│  │  │   experiences consolidate into semantic       │    │    │
│  │  │   knowledge, extract training-relevant facts  │    │    │
│  │  └─────────────────────────────────────────────┘    │    │
│  │                                                      │    │
│  │  Source 4: Feedback Collection (/feedback in REPL)   │    │
│  │  ┌─────────────────────────────────────────────┐    │    │
│  │  │ • Explicit user feedback on adapter quality  │    │    │
│  │  │ • "This classification was wrong because..." │    │    │
│  │  │ • Feature requests: "the adapter should also │    │    │
│  │  │   handle X scenario"                         │    │    │
│  │  └─────────────────────────────────────────────┘    │    │
│  └──────────────────────────────────────────────────────┘    │
│                          ↓                                   │
│  ┌──────────────────────────────────────────────────────┐    │
│  │ DATA CURATION (periodic, semi-automatic)             │    │
│  │                                                      │    │
│  │  training_curate_feedback (new tool)                 │    │
│  │      ├── Query episodic memory for adapter sessions  │    │
│  │      ├── Filter: only sessions with user corrections │    │
│  │      │   or CNS alerts                               │    │
│  │      ├── Deduplicate: cluster similar failure cases  │    │
│  │      ├── Generate corrected traces:                  │    │
│  │      │   original query + corrected classification   │    │
│  │      │   + reasoning trace showing why original      │    │
│  │      │   was wrong and corrected is right            │    │
│  │      ├── Curator review gate: human approves or     │    │
│  │      │   rejects each new trace before training      │    │
│  │      └── Append to skill's trace corpus              │    │
│  │                                                      │    │
│  │  Trigger conditions:                                 │    │
│  │      • Time-based: every N days of adapter usage     │    │
│  │      • Volume-based: after M user corrections         │    │
│  │      • CNS-based: algedonic alert rate exceeds        │    │
│  │        threshold for adapter-tagged sessions         │    │
│  └──────────────────────────────────────────────────────┘    │
│                          ↓                                   │
│  ┌──────────────────────────────────────────────────────┐    │
│  │ BATCHED RETRAINING (triggered)                       │    │
│  │                                                      │    │
│  │  training_retrain (new tool)                         │    │
│  │      ├── Merge original traces + curated feedback    │    │
│  │      │   traces into combined dataset                │    │
│  │      ├── Split: 80% train / 20% holdout evaluation   │    │
│  │      ├── Submit fine-tuning job with combined data   │    │
│  │      ├── Evaluate against holdout + original test    │    │
│  │      │   suite (regression check)                    │    │
│  │      ├── Decision gate:                              │    │
│  │      │   ├── Score improved → promote new adapter    │    │
│  │      │   ├── Score same → keep current adapter       │    │
│  │      │   └── Score worse → revert, investigate       │    │
│  │      └── Update adapter registry with new version    │    │
│  │                                                      │    │
│  │  Versioning:                                         │    │
│  │      constraint-forces-v1 → v2 → v3 ...              │    │
│  │      Router always serves latest production version  │    │
│  │      Previous versions retained for rollback          │    │
│  └──────────────────────────────────────────────────────┘    │
└──────────────────────────────────────────────────────────────┘
```

### 7.3 CNS as Training Data Engine

The Cybernetic Nervous System is uniquely positioned to drive continuous training because it already monitors what matters:

| CNS Signal | Training Implication |
|------------|---------------------|
| **Algedonic alerts** during adapter sessions | High alert rate → adapter may be producing incorrect classifications that trigger downstream CNS warnings. These sessions are prime candidates for trace extraction. |
| **Variety deficit** after adapter-heavy turns | Low variety → adapter may be stuck in a narrow classification pattern, missing edge cases. Generate traces targeting the missing categories. |
| **Backpressure events** (`cns.cybernetics.backpressure`) | Spans emitted when constraints conflict. Extract the conflicting constraints as new training scenarios. |
| **Confidence tracking** (per-response logprobs) | Low-confidence responses → the model is uncertain. These are exactly the cases where more training data is needed. |
| **Curator escalation rate** | High escalation rate for adapter-tagged sessions → systematic quality issue. Trigger retraining cycle. |
| **Gas consumption patterns** | Adapter sessions consuming unusual energy → possible loop or repetitive correction pattern. Flag for review. |

**Implementation sketch:**

```
CNS span emission
    ↓
Tagged with adapter_id + skill + session_id
    ↓
Episodic memory: store_experience(adapter_invocation)
    ↓
Consolidation pipeline: every 10 experiences → generate_narrative()
    ↓
Narrative analysis: "adapter X produced 3 low-confidence
    Guardrail classifications in session Y"
    ↓
Trigger: training_curate_feedback when narrative count
    exceeds threshold
```

### 7.4 Tool Development Roadmap for Continuous Training

Tools to close the continuous training loop:

| Tool | Purpose | Status |
|------|---------|--------|
| `training_record_invocation` | Record each adapter use as an episodic experience with CNS span correlation | ✅ Built (2026-06) |
| `training_curate_feedback` | Query semantic memory for QA pairs, validate with LLM-as-judge, generate corrected traces | ✅ Built (2026-06) |
| `training_retrain` | Merge original + feedback traces, deduplicate, submit retraining job with incremented version | ✅ Built (2026-06) |
| `training_monitor_health` | Track adapter quality metrics over time (accuracy trend, alert correlation, confidence distribution) | 🔵 Deferred — needs active usage data |
| `training_ab_test` | Serve multiple adapter versions simultaneously, route fraction of traffic to each, compare outcomes | 🔵 Deferred — needs multiple active versions |

---

## 8. Statistical Settings: Inference Parameters in Training Context

### 8.1 The Two Modes: Generation vs Evaluation

Training involves two distinct inference modes with different optimal settings:

| Parameter | Trace Generation Mode | Evaluation Mode |
|-----------|----------------------|-----------------|
| **Purpose** | Produce diverse, creative, methodologically correct reasoning traces | Produce consistent, deterministic classifications for scoring |
| **temperature** | 0.7–0.9 (high diversity) | 0.0 (deterministic) |
| **top_p** | 0.9–0.95 (wide sampling) | 1.0 (no truncation, but temp=0 makes this irrelevant) |
| **top_k** | 40–60 (broad candidate pool) | 1 (greedy, but temp=0 makes this irrelevant) |
| **min_p** | 0.0 (disabled) | 0.0 (disabled) |
| **typical_p** | 0.0 (disabled) | 0.0 (disabled) |
| **max_tokens** | 512–1024 (full reasoning traces) | 20–50 (classification word only) |
| **seed** | random (diversity) | fixed (reproducibility) |
| **reasoning** | enabled (for trace generation model) | **disabled** (classification tasks — reasoning tokens eat budget without improving accuracy) |

### 8.2 Temperature's Role in Training Data Quality

Temperature is the most impactful parameter for training data generation:

**High temperature (0.7–0.9) — Trace Generation:**
- **Benefit**: Produces diverse reasoning paths. Two traces for the same constraint type will phrase the decision tree walk differently, use different examples, emphasize different distinctions. This diversity prevents the trained adapter from memorizing specific phrasings.
- **Risk**: At very high temperatures (>1.0), traces may become incoherent — reasoning steps that don't follow from each other, incorrect classifications justified with confident-sounding but wrong logic.
- **Mitigation**: Validate generated traces before training. The `training_generate_traces` tool should include a validation pass: does the trace's final classification match the expected answer? Does the reasoning chain logically hold?

**Zero temperature (0.0) — Evaluation:**
- **Benefit**: Deterministic output. Same input always produces same classification. This is essential for scoring — you need to know whether the model consistently gets a test case right or wrong.
- **Risk**: None for classification tasks. The model always picks the highest-probability token sequence.
- **Caveat**: Zero temperature doesn't guarantee correctness — it guarantees consistency. A model with temp=0 can be consistently wrong.

**Temperature during training (fine-tuning job):**
- Temperature is NOT a training hyperparameter. The fine-tuning job uses a fixed training loss (cross-entropy) regardless of what temperature was used to generate the training data.
- However, the temperature used during **data generation** affects what the model learns:
  - Traces generated at temp=0.0 → model learns one "correct" way to answer. Brittle — fails on paraphrased inputs.
  - Traces generated at temp=0.7 → model learns the methodology, not the phrasing. Robust — generalizes to novel inputs.

### 8.3 Top-P and Top-K: Controlling Diversity vs Precision

**Top-P (nucleus sampling):**
- `top_p=0.9` means: sample from the smallest set of tokens whose cumulative probability ≥ 0.9. This dynamically adjusts the candidate pool based on confidence — when the model is confident, few candidates; when uncertain, more candidates.
- For trace generation: `top_p=0.9–0.95` provides a good balance. Lower values (0.5–0.7) produce more conservative, repetitive traces. Higher values (1.0) include low-probability tokens that may introduce errors.
- For evaluation: irrelevant when temperature=0 (only the single highest-probability token is selected).

**Top-K:**
- `top_k=40` means: only consider the 40 highest-probability tokens at each step.
- For trace generation: `top_k=40–60` prevents the model from sampling extremely unlikely tokens while still allowing diversity.
- Interaction with temperature: high temp + high top_k = maximum diversity (may be incoherent). High temp + low top_k = diverse but constrained. Low temp + any top_k ≈ deterministic.

### 8.4 Seed and Reproducibility

- **Trace generation**: `seed=random` — each generation run produces different traces, building a diverse corpus across multiple runs.
- **Evaluation**: `seed=fixed` (e.g., `seed=42`) — ensures that evaluation results are reproducible. Same test suite, same seed, same scores.
- **Training**: Seed is NOT a training hyperparameter for Together AI fine-tuning (the API sets `random_seed` automatically).

### 8.5 Reasoning Mode: Critical for Training, Dangerous for Evaluation

Qwen3.5 models support a `reasoning` mode that produces internal chain-of-thought before the visible response. This has opposite effects in our two modes:

**Trace Generation — Reasoning ON:**
- The reasoning trace IS the training data. We WANT the model to produce explicit step-by-step reasoning. For the teacher model generating traces, reasoning mode enriches the output.
- However: if the teacher model is itself a fine-tuned adapter, reasoning mode may produce reasoning about reasoning — meta-cognitive loops that waste tokens.
- Recommendation: Use base model (not adapter) for trace generation, with reasoning enabled.

**Evaluation — Reasoning OFF (mandatory):**
- Reasoning tokens count against `max_tokens` but are NOT visible in `choices[0].message.content`. With `max_tokens=20` and reasoning enabled, the model may spend all 20 tokens on internal reasoning and produce an empty visible response.
- This was a discovered pitfall: v1 evaluation initially showed empty responses because reasoning consumed the token budget.
- **Hard rule**: Always set `"reasoning": {"enabled": false}` for classification evaluation.

### 8.6 Max Tokens: Budgeting for Thought vs Answer

| Mode | max_tokens | Rationale |
|------|-----------|----------|
| Trace generation | 512–1024 | Full decision tree walk requires space. A typical constraint-forces trace is 200–400 tokens. Allow headroom. |
| Classification evaluation | 20–50 | We only need the classification word ("Prohibition", "Guardrail", etc.). 20 tokens is sufficient; 50 provides safety margin. |
| Conflict resolution evaluation | 200–400 | Conflict resolution traces include classification of both constraints + resolution logic. |
| Procedural skill evaluation | 512–1024 | Skills like `diagnose` or `essentialist` produce multi-step procedures. |

### 8.7 Training Hyperparameters (Fine-Tuning Job)

These are set in the `training_submit` request, separate from inference parameters:

| Parameter | Default | Effect of Changing |
|-----------|---------|-------------------|
| **n_epochs** | 3 | More epochs = more learning from limited data, but risk of overfitting (memorizing traces instead of learning methodology). For small datasets (<100 traces), 3 epochs is a safe default. For larger datasets (500+), 1-2 epochs may suffice. |
| **learning_rate** | 2e-4 | Higher LR (5e-4–1e-3) = faster adaptation but risk of catastrophic forgetting (overwriting base model capabilities). Lower LR (5e-5–1e-4) = gentler adaptation but may underfit small datasets. 2e-4 is the community standard for LoRA. |
| **lora_r (rank)** | 16–64 | Higher rank = more adapter capacity. For classification tasks (constraint-forces), r=16 is sufficient. For procedural tasks (diagnose), r=32–64 may be needed. Together AI defaults to r=64. |
| **lora_alpha** | 32–128 | Scaling factor for LoRA updates. Typically alpha = 2× rank. Higher alpha = stronger adaptation signal. Together AI defaults to alpha=128. |
| **batch_size** | 8 | Larger batches = more stable gradients but require more data. For small datasets, batch_size must be ≤ dataset size / epochs. Together AI enforces batch_size ≥ 8. |
| **target_modules** | ["q_proj","v_proj","k_proj","o_proj"] | Which attention layers to adapt. Default targets all attention projections. For some tasks, adding "gate_proj" and "up_proj" (MLP layers) improves capacity. Together AI defaults include all 7 modules. |

### 8.8 Settings Decision Matrix

```
┌─────────────────────────────────────────────────────────────┐
│ WHICH SETTINGS FOR WHICH PHASE?                             │
│                                                             │
│  TRACE GENERATION (training_generate_traces)                │
│  ┌─────────────────────────────────────────────────────┐   │
│  │ temp=0.8  top_p=0.9  top_k=50  max_tokens=1024     │   │
│  │ seed=random  reasoning=enabled (base model)         │   │
│  │                                                     │   │
│  │ Goal: diverse, methodologically correct traces      │   │
│  │ Risk: incoherence at temp > 1.0                     │   │
│  │ Validation: check final classification matches      │   │
│  │   expected answer before adding to dataset          │   │
│  └─────────────────────────────────────────────────────┘   │
│                                                             │
│  CLASSIFICATION EVALUATION (testing adapter accuracy)       │
│  ┌─────────────────────────────────────────────────────┐   │
│  │ temp=0.0  max_tokens=30  reasoning=disabled         │   │
│  │ seed=42   top_p/top_k: irrelevant (temp=0)          │   │
│  │                                                     │   │
│  │ Goal: deterministic, reproducible scoring           │   │
│  │ Risk: reasoning mode silently consuming tokens      │   │
│  │ Hard rule: ALWAYS disable reasoning for eval        │   │
│  └─────────────────────────────────────────────────────┘   │
│                                                             │
│  PROCEDURAL EVALUATION (testing diagnose/essentialist)      │
│  ┌─────────────────────────────────────────────────────┐   │
│  │ temp=0.0  max_tokens=1024  reasoning=disabled       │   │
│  │ seed=42                                             │   │
│  │                                                     │   │
│  │ Goal: deterministic procedure output                │   │
│  │ Note: even at temp=0, procedural outputs may vary   │   │
│  │   slightly due to long-form generation dynamics.    │   │
│  │   Evaluate with rubric, not exact string match.     │   │
│  └─────────────────────────────────────────────────────┘   │
│                                                             │
│  PRODUCTION SERVING (agent sessions using adapter)          │
│  ┌─────────────────────────────────────────────────────┐   │
│  │ temp=0.3  top_p=0.9  top_k=40  max_tokens=512      │   │
│  │ reasoning=disabled (classification skills)          │   │
│  │                                                     │   │
│  │ Goal: reliable but not brittle responses            │   │
│  │ Why not temp=0? Slight diversity prevents the       │   │
│  │   adapter from producing identical phrasing for     │   │
│  │   similar inputs, which feels robotic. But low      │   │
│  │   enough to maintain classification accuracy.      │   │
│  └─────────────────────────────────────────────────────┘   │
└─────────────────────────────────────────────────────────────┘
```

---

*This document is a living architecture record. Update as the training pipeline evolves.*
