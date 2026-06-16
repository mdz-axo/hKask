---
title: "Public Surface Justification — hkask-mcp-training"
audience: [architects, developers]
last_updated: 2026-06-15
version: "0.27.0"
status: "Active"
domain: "Technology"
mds_categories: [composition]
---

# Public Surface Justification — hkask-mcp-training

**Crate:** `hkask-mcp-training` (MCP server binary)  
**Public items in lib.rs:** 3 modules (`adapters`, `dataset`, `providers`) + 6 re-exports  
**MCP tools exposed:** 15  
**Deep-module threshold:** ≤7 public functions (Ousterhout)

## Architecture

The training MCP server implements a complete **skills training** pipeline: take a `SKILL.md` document, generate decomposition traces, train a LoRA adapter on a base model, and produce `base_model + adapter = skill implementation` that can outperform frontier models at that specific procedural skill.

```
SKILL.md → training_generate_traces → ChatML JSONL
         → training_submit → TrainingProvider → LoRAAdapter
         → training_evaluate → accuracy metrics
         → training_record_invocation → episodic memory
         → training_curate_feedback → corrected traces
         → training_retrain → version++ → loop
```

## Tool Surface (15 tools)

| # | Tool | Category | Description |
|---|------|----------|-------------|
| 1 | `training_ingest_qa` | Data ingestion | Store QA pairs in semantic memory |
| 2 | `training_ingest_dataset` | Data ingestion | Normalize raw datasets (ChatML/ShareGPT/Alpaca/text) into cache |
| 3 | `training_assemble_dataset` | Data preparation | Assemble stored QA pairs into ChatML JSONL with optional system prompt |
| 4 | `training_generate_traces` | Data generation | Generate decomposition traces from skill documents (with model override + chunking) |
| 5 | `training_submit` | Training | Submit training job via pluggable provider (with token-length validation) |
| 6 | `training_status` | Training | Query job status (auto-registers adapter on completion + blob storage) |
| 7 | `training_cancel` | Training | Cancel running job (PID-tracked for local providers) |
| 8 | `training_evaluate` | Evaluation | Evaluate adapter against test dataset (exact/contains/semantic) |
| 9 | `training_register_adapter` | Registry | Register completed adapter in persistent store |
| 10 | `training_list_adapters` | Registry | List all adapters with skill_name, version, metrics |
| 11 | `training_delete_adapter` | Registry | Delete adapter and artifacts |
| 12 | `training_recommend_model` | Guidance | Recommend base model for skills training |
| 13 | `training_record_invocation` | Continuous loop | Record adapter invocation as episodic experience |
| 14 | `training_curate_feedback` | Continuous loop | LLM-as-judge feedback curation from QA pairs |
| 15 | `training_retrain` | Continuous loop | Merge original + feedback, retrain with incremented version |

## Providers (5 backends)

| Provider | Type | Status | Key Feature |
|----------|------|--------|-------------|
| **Together AI** | Managed fine-tuning API | Production | Upload→train→deploy→infer, ~$0.005/LoRA run |
| **Baseten** | Managed infra + your code | Implemented | Generated TRL/LoRA train.py, HF-native model loading, multi-LoRA serving |
| **Runpod** | GPU pod dispatch | Implemented | Template-based pod creation, GraphQL API |
| **Axolotl** | Local CLI | Production | YAML-config-driven, PID-tracked cancellation |
| **Unsloth** | Local Python | Production | Memory-efficient, PID-tracked cancellation |

## Infrastructure

| Component | Description |
|-----------|-------------|
| `DatasetPipeline` | Format detection, normalization (ChatML/ShareGPT/Alpaca/text), validation, caching |
| `AdapterStore` (trait) | LoRA adapter metadata + blob persistence |
| `SqliteAdapterStore` | SQLite-backed production store with `lora_adapters` + `lora_blobs` tables |
| `InMemoryAdapterStore` | In-memory store for testing/fallback |
| `JobStore` | Persistent job registry (`training_jobs` table) — survives server restarts |
| `CompletionMetadata` | Provider-agnostic training completion metadata (base model, loss, tokens, duration) |

## Key Design Decisions

- **Skills training, not general fine-tuning:** Each adapter implements a specific skill from a `SKILL.md` document. The adapter IS the compiled form of the skill.
- **Decomposition traces, not QA pairs:** Traces train *how to think* (situation→decomposition→synthesis), not just *what to answer*.
- **LoRA, not full fine-tuning:** ~$0.005/run, 4-7 minutes, 200MB adapters, composable.
- **Multi-provider:** 5 backends spanning managed APIs, managed infra, GPU dispatch, and local training.
- **Continuous training loop:** train → evaluate → record → curate → retrain → version++.

## Why This Surface Is Large

The training MCP server is the **skills training engine** — it spans the full lifecycle from data generation through training, evaluation, and continuous improvement. Its 15 tools reflect the breadth of the pipeline:

1. **Data layer** (3 tools) — ingest, normalize, assemble, generate
2. **Training layer** (3 tools) — submit, status, cancel
3. **Evaluation layer** (1 tool) — evaluate
4. **Registry layer** (3 tools) — register, list, delete
5. **Guidance layer** (1 tool) — recommend model
6. **Continuous loop** (3 tools) — record, curate, retrain
7. **Provider layer** (5 backends) — Together AI, Baseten, Runpod, Axolotl, Unsloth

## Mitigations

- **Trait-based provider abstraction:** `TrainingProvider` trait isolates 5 backends behind a common interface.
- **Shared infrastructure:** Uses `hkask-mcp` for daemon communication, startup verification, tool registration.
- **Shared storage:** Uses `hkask-storage` for SQLite-backed adapter registry and job persistence.
- **Shared inference:** Uses `hkask-inference` for trace generation, evaluation, and feedback curation.

## Deletion Test

Delete `hkask-mcp-training` and the skills training pipeline (data generation, LoRA training, evaluation, continuous improvement) must be rebuilt from scratch across scattered scripts and manual processes. The 15-tool surface consolidates what would otherwise be 6+ separate workflows. The crate earns its existence.

## Multi-LoRA Inference Routing

The `hkask-inference` crate supports multi-LoRA serving via `LLMParameters.adapter`. When set, the adapter name is appended to the model in provider-specific format (e.g., Baseten: `model#adapter`). This enables a single deployed base model to serve multiple skill adapters, selected per-request.

## Deferred Items

| Item | Reason |
|------|--------|
| `training_monitor_health` | Needs sufficient active usage data for meaningful trends |
| `training_ab_test` | Needs multiple adapter versions in active use |
| Fireworks AI provider | Billing inefficiency and API complexity — not pursuing |
