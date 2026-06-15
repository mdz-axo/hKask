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

- [x] Training MCP server (8 tools)
- [x] Together AI provider integration (upload + fine-tune + deploy)
- [x] Decomposition trace generation (`training_generate_traces`)
- [x] constraint-forces skill adapter (v2, 100% accuracy)
- [x] End-to-end pipeline proven (generate → upload → train → deploy → evaluate)

### 4.2 Near-Term (Next 2-4 Weeks)

- [ ] **pragmatic-semantics adapter**: Classification-shaped (IS vs OUGHT, declarative vs probabilistic vs subjunctive). Similar structure to constraint-forces — easy to auto-evaluate.
- [ ] **`training_evaluate` tool**: Automated holdout evaluation — split traces into train/test, run inference against trained adapter, score accuracy.
- [ ] **Multi-skill evaluation harness**: Test adapter composition — can the model apply constraint-forces AND pragmatic-semantics in the same response?

### 4.3 Medium-Term (1-2 Months)

- [ ] **Procedural skill adapters**: `essentialist` (3-gate deletion test), `diagnose` (spec-anchored debugging loop), `tdd` (red-green-refactor). These require more complex traces — multi-step procedures with branching.
- [ ] **Adapter registry**: Versioned adapter store with metadata (skill, base model, evaluation scores, training date, trace count).
- [ ] **Automatic adapter selection**: Inference router detects task type from user query and routes to appropriate adapter.

### 4.4 Long-Term (3-6 Months)

- [ ] **Multi-LoRA serving**: Single Together AI endpoint serving multiple hKask skill adapters. Adapter selected per-request by the router.
- [ ] **Adapter composition**: Merge or route between multiple adapters for cross-skill tasks (e.g., sovereignty audit requiring constraint-forces + pragmatic-semantics + magna-carta-verifier).
- [ ] **Continual adaptation**: Generate training traces from agent experiences (CNS episodic memory) — adapters improve from real usage, not just static documents.
- [ ] **Cross-model adapters**: Train adapters for different base models (Qwen3.5, DeepSeek, Llama 4) — skill portability across inference providers.

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

### 5.4 Why Qwen3.5-9B?

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

*This document is a living architecture record. Update as the training pipeline evolves.*
