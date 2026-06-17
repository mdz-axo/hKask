---
title: "Public Surface Justification — hkask-mcp-training"
audience: [architects, developers]
last_updated: 2026-06-16
version: "0.27.0"
status: "Active"
domain: "Technology"
mds_categories: [composition]
---

# Public Surface Justification — hkask-mcp-training

**Crate:** `hkask-mcp-training` (MCP server binary)  
**Public items in lib.rs:** 4 modules (`adapters`, `dataset`, `huggingface`, `providers`) + re-exports  
**MCP tools exposed:** 17  
**Deep-module threshold:** ≤7 public functions (Ousterhout)

## Architecture

The training MCP server implements a complete **skills training** pipeline: take a `SKILL.md` document, generate type-specialized decomposition traces (WordAct/FlowDef/KnowAct/Composite), train a LoRA adapter on a base model via a harness-aware host dispatch, and produce `base_model + adapter = skill implementation` that can outperform frontier models at that specific procedural skill.

```
SKILL.md → training_generate_traces → ChatML JSONL (type-specialized)
         → training_submit → TrainingHost (harness-aware) → LoRAAdapter
         → training_evaluate → accuracy metrics
         → training_record_invocation → episodic memory
         → training_curate_feedback → failure-categorized, quality-gated traces
         → training_retrain → A/B baseline → version++ → loop
         → training_sweep → parameter grid search → best config
```

## Tool Surface (17 tools)

| # | Tool | Category | Description |
|---|------|----------|-------------|
| 1 | `training_ingest_qa` | Data ingestion | Store QA pairs in semantic memory |
| 2 | `training_ingest_dataset` | Data ingestion | Normalize raw datasets (ChatML/ShareGPT/Alpaca/text) into cache |
| 3 | `training_assemble_dataset` | Data preparation | Assemble stored QA pairs into ChatML JSONL with optional system prompt + train/test split |
| 4 | `training_generate_traces` | Data generation | Generate type-specialized decomposition traces (WordAct/FlowDef/KnowAct/Composite) from skill documents |
| 5 | `training_generate_cot` | Data generation | Generate chain-of-thought training traces with multi-step reasoning visibility |
| 6 | `training_submit` | Training | Submit training job via harness-aware host dispatch (with model provenance resolution + token-length validation) |
| 7 | `training_status` | Training | Query job status (auto-registers adapter on completion + A/B comparison + blob storage) |
| 8 | `training_cancel` | Training | Cancel running job (PID-tracked for local providers) |
| 9 | `training_evaluate` | Evaluation | Evaluate adapter against test dataset (exact/contains/semantic) |
| 10 | `training_register_adapter` | Registry | Register completed adapter in persistent store with version, metrics, skill_name |
| 11 | `training_list_adapters` | Registry | List all adapters with skill_name, version, metrics |
| 12 | `training_delete_adapter` | Registry | Delete adapter and artifacts |
| 13 | `training_recommend_model` | Guidance | Recommend base model for skills training (task type, budget, latency, license) |
| 14 | `training_record_invocation` | Continuous loop | Record adapter invocation as episodic experience with CNS span correlation |
| 15 | `training_curate_feedback` | Continuous loop | LLM-as-judge feedback curation with failure categorization (hallucination/omission/procedural_error/off_target) and quality threshold gating |
| 16 | `training_retrain` | Continuous loop | Merge original + quality-gated feedback, A/B baseline recording, retrain with incremented version |
| 17 | `training_sweep` | Optimization | Parameter grid search across learning rates, LoRA ranks, batch sizes, epochs — submits N jobs, reports best config |

## Trace Type Specialization

Decomposition traces are generated per skill type, auto-detected by hLexicon term density:

| TraceType | Purpose | Structure | Evaluation Metric |
|-----------|---------|-----------|-------------------|
| **WordAct** | Persona calibration — "how to sound" | {context, persona_constraints, target_utterance, calibration_notes} | Persona fidelity |
| **FlowDef** | Procedural decomposition — "how to think" | {situation, decomposition_sequence, synthesis, verification} | Decomposition accuracy |
| **KnowAct** | Pattern recognition — "how to classify" | {pattern_exemplar, positive_cases[], negative_cases[], decision_boundary} | Classification precision/recall |
| **Composite** | Mixed WordAct + FlowDef segments | Alternating persona + procedural segments | Combined |

## Training Mode Distinction

| Mode | What is trained | Training data | Evaluation |
|------|----------------|---------------|------------|
| **QA Semantic Fact** | "What to answer" — factual domain knowledge | QA pairs (ingest_qa → assemble_dataset) | Exact/contains/semantic match |
| **Decomposition Trace** | "How to think" — procedural decomposition | Generated traces from SKILL.md | Decomposition accuracy |
| **Hybrid** | Both QA + traces | Weighted merge (default 30% QA / 70% traces) | Combined metrics |

## Harness/Host Architecture

The training dispatch is a three-layer model:

```
TrainingJob
  ├── harness: TrainingHarnessId (Axolotl | Unsloth)
  │     └── determines config format (YAML | Python script)
  └── host: TrainingHostId (Together | Runpod | Baseten)
        └── determines where compute runs
```

- **Harness** (Axolotl/Unsloth) determines *which training config to generate*
- **Host** (Together/Runpod/Baseten) determines *where to run it*
- Cloud hosts are harness-aware — they receive harness-specific configuration in dispatch payloads
- Each `TrainingJob` carries both `harness` and `host` fields; user intent is never silently dropped

## TrainingParams (Deepened)

The canonical `TrainingParams` now exposes the union of Axolotl and Unsloth capabilities:

| Sub-struct | Fields |
|-----------|--------|
| `LoraParams` | r, alpha, dropout, target_modules, modules_to_save, use_rslora |
| `QuantizationParams` | load_in_4bit, load_in_8bit, bnb_4bit_compute_dtype, bnb_4bit_quant_type, bnb_4bit_use_double_quant |
| `OptimizationParams` | optimizer, weight_decay, warmup_steps, warmup_ratio, lr_scheduler, gradient_accumulation_steps, cosine_min_lr_ratio, adam_beta1/2/epsilon, max_grad_norm |
| `SequenceParams` | sequence_len, sample_packing, pad_to_sequence_len, neftune_noise_alpha |
| `AdvancedParams` | attn_implementation, gradient_checkpointing, bf16, fp16, eval_split_ratio |

CNS span `cns.training.harness.params_used` emitted on each harness-specific config generation. 15 `HarnessCapability` variants provide per-capability CNS observability.

## Providers (5 backends)

| Provider | Type | Status | Key Feature |
|----------|------|--------|-------------|
| **Together AI** | Managed fine-tuning API | Production | Upload→train→deploy→infer, harness-aware dispatch |
| **Baseten** | Managed infra + your code | Implemented | Generated TRL/LoRA train.py, HF-native model loading, multi-LoRA serving |
| **Runpod** | GPU pod dispatch | Implemented | Template-based pod creation, GraphQL API, harness env var dispatch |
| **Axolotl** | Local CLI | Production | Full YAML-config-driven with deepened params, PID-tracked cancellation |
| **Unsloth** | Local Python | Production | Full Python script generation with deepened params, PID-tracked cancellation |

## Infrastructure

| Component | Description |
|-----------|-------------|
| `DatasetPipeline` | Format detection, normalization (ChatML/ShareGPT/Alpaca/text), validation, caching |
| `AdapterStore` (trait) | LoRA adapter metadata + blob persistence |
| `SqliteAdapterStore` | SQLite-backed production store with `lora_adapters` + `lora_blobs` tables |
| `InMemoryAdapterStore` | In-memory store for testing/fallback |
| `JobStore` | Persistent job registry (`training_jobs` table) — survives server restarts |
| `CompletionMetadata` | Provider-agnostic training completion metadata (base model, loss, tokens, duration) |
| `ModelResolver` (trait) | HuggingFace model provenance resolution (model cards, license, architecture, gating) |
| `LocalModelResolver` | Static known-model registry (llama, mistral, qwen, gemma, phi, deepseek, yi, falcon) |
| `FailureCategory` (5 types) | hallucination, omission, procedural_error, off_target, other — surfaced in curation output |
| `AbBaseline` | Previous adapter metrics recorded on retrain for A/B comparison on completion |

## Feedback Quality Gating

`training_curate_feedback` now produces quality-gated output:

- **Failure categorization:** Each corrected answer is classified into one of 5 failure types
- **Inter-rater agreement:** Computed as pass rate across reviewed pairs
- **Quality threshold:** Default 0.7 — only feedback sets above threshold should proceed to retraining
- **Per-category breakdown:** `failures_by_category` included in result JSON

## Continuous Training Loop (Deepened)

The continuous loop now includes A/B evaluation:

1. **Curate** — QA pairs reviewed by LLM judge with failure categorization
2. **Quality gate** — only feedback with inter-rater agreement ≥ 0.7 proceeds
3. **Retrain** — records A/B baseline from previous adapter version
4. **Complete** — `training_status` auto-compares new vs. old loss on adapter registration
5. **Promote** — new adapter auto-promoted when loss improves

## Key Design Decisions

- **Skills training, not general fine-tuning:** Each adapter implements a specific skill from a `SKILL.md` document. The adapter IS the compiled form of the skill.
- **Type-specialized traces:** WordAct trains persona, FlowDef trains procedure, KnowAct trains classification — not all skills are procedural.
- **QA vs Decomposition distinction:** Factual QA ("what to answer") and procedural traces ("how to think") are distinct training modes with different evaluation metrics.
- **LoRA, not full fine-tuning:** ~$0.005/run, 4-7 minutes, 200MB adapters, composable.
- **Harness-aware dispatch:** Three-layer model (harness × host) — user intent is never silently dropped.
- **Deepened parameters:** Full Axolotl + Unsloth capability surface exposed through canonical `TrainingParams` sub-structs.
- **Continuous training loop:** train → evaluate → record → curate (with failure categories) → retrain (with A/B baseline) → version++.

## Why This Surface Is Large

The training MCP server is the **skills training engine** — it spans the full lifecycle from data generation through training, evaluation, and continuous improvement. Its 17 tools reflect the breadth of the pipeline:

1. **Data layer** (4 tools) — ingest QA, ingest dataset, assemble, generate traces, generate CoT
2. **Training layer** (3 tools) — submit, status, cancel
3. **Evaluation layer** (1 tool) — evaluate
4. **Registry layer** (3 tools) — register, list, delete
5. **Guidance layer** (1 tool) — recommend model
6. **Continuous loop** (3 tools) — record, curate, retrain
7. **Optimization layer** (1 tool) — sweep
8. **Provider layer** (5 backends) — Together AI, Baseten, Runpod, Axolotl, Unsloth

## Mitigations

- **Trait-based provider abstraction:** `TrainingHost` trait isolates 5 backends behind a common interface.
- **Trait-based model resolution:** `ModelResolver` trait for HuggingFace provenance before training.
- **Shared infrastructure:** Uses `hkask-mcp` for daemon communication, startup verification, tool registration.
- **Shared storage:** Uses `hkask-storage` for SQLite-backed adapter registry and job persistence.
- **Shared inference:** Uses `hkask-inference` for trace generation, evaluation, and feedback curation.

## Deletion Test

Delete `hkask-mcp-training` and the skills training pipeline (data generation, LoRA training, evaluation, continuous improvement, parameter sweep) must be rebuilt from scratch across scattered scripts and manual processes. The 17-tool surface consolidates what would otherwise be 7+ separate workflows. The crate earns its existence.

## Multi-LoRA Inference Routing

The `hkask-inference` crate supports multi-LoRA serving via `LLMParameters.adapter`. When set, this field **completely overrides the model** — it is the full model identifier including the base model (e.g., `"Qwen3.5-9B#constraint-forces-v3"`). The adapter was trained on a specific base model and cannot be applied to a different one. The caller resolves which base model the adapter needs via `AdapterStore` lookup by `skill_name`. This enables different skills to use different optimal base models (Qwen for classification, DeepSeek for reasoning, etc.).

## Deferred Items

| Item | Reason |
|------|--------|
| `training_monitor_health` | Needs sufficient active usage data for meaningful trends |
| Fireworks AI provider | Billing inefficiency and API complexity — not pursuing |
| Multi-LoRA composition (simultaneous serving) | `hkask-inference` currently supports single adapter per request |
| Adapter version drift detection | Can reuse CNS drift-detection from `DefaultSpecCurator` |
| HuggingFace dataset versioning (`datasets.load_dataset`) | Current pipeline uses local JSONL files |
| Harness-specific optimizer naming layer | Axolotl vs Unsloth optimizer names differ; needs mapping |
