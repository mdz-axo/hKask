# hkask-mcp-training

Model training MCP server — ingests QA pairs and training data for fine-tuning pipelines.

Uses internal tool dispatch pattern (not individual `pub async fn` per tool).

## Tools (8)

Simplified from 21 → 15 → 8 across 2026-07-19 cleanups.

| Tool | Description |
|------|-------------|
| `training_ingest_qa` | Ingest QA pairs for model training. Stores question-answer pairs with provenance in semantic memory for future fine-tuning dataset assembly |
| `training_ingest_dataset` | Ingest a raw dataset file into the normalized cache without submitting a training job. Detects format (ChatML, ShareGPT, Alpaca, raw text), normalizes to canonical ChatML, validates, and caches |
| `training_assemble_dataset` | Assemble stored QA pairs into a ChatML JSONL training dataset file. Queries semantic memory for training_qa_pair triples, filters by dataset/source/bloom level, and writes a file ready for training_submit. Optionally splits into train/test |
| `training_submit` | Submit a training job for execution. Ingests, normalizes, and submits a dataset for LoRA fine-tuning via the configured host (axolotl or unsloth). When `feedback_path` is provided, enters retrain mode: merges original + feedback, deduplicates, increments version, pre-registers adapter metadata for A/B comparison |
| `training_status` | Query the status of a training job by its ID. When a job completes, automatically registers the adapter in the persistent store if not already registered |
| `training_cancel` | Cancel a running or queued training job |
| `training_evaluate` | Evaluate a trained adapter against a test dataset. Runs inference for each test example and scores accuracy using exact match, substring containment, or semantic comparison |
| `training_validate_config` | Run the lora-training skill's static math-contract gates (G-M1..G-M4, G-Q1, G-Q2, G-Q4) on training params. Emits `cns.lora.audit` spans. This is the runtime enforcement point for the `.agents/skills/lora-training/` skill's `audit-config` phase |

### Deleted tools (2026-07-19, second pass)

- `training_deploy` / `training_deployment_status` / `training_teardown` — replaced by `hkask_adapter::AdapterPort::{create_endpoint, endpoint_status, teardown_endpoint}`. The MCP server was a thin wrapper; deployment now goes through the canonical AdapterPort surface directly.
- `training_list_adapters` / `training_delete_adapter` — `AdapterPort::list_adapters` and `AdapterStore::delete` already cover these. Rare operations; route via CLI.
- `training_register_adapter` — `training_status` auto-registers on completion; manual registration is an `AdapterStore` API call, not an MCP tool.
- `training_preflight_check` — replaced by `training_validate_config`, which runs the actual lora-training skill gates (not just file-existence checks).
- `training_retrain` — merged into `training_submit` as optional `feedback_path` + `skill_name` + `adapter_name` parameters.

### Deleted tools (2026-07-19, first pass)

- `training_generate_traces`, `training_generate_chain_of_thought` (inference, not training)
- `training_sweep` (use submit in a loop)
- `training_merge_adapters` (speculative, never produced output)
- `training_record_invocation`, `training_curate_feedback` (data curation, not training)
- `training_recommend_model` (can be done offline)

## Providers

Single cloud host: **Runpod**. Single harness: **Axolotl** (YAML).

Deleted providers (2026-07-19): `TogetherHost` (Together AI REST API) and `TinkerHost` (Thinking Machines subprocess). The Runpod host is sufficient for all training workloads.

Deleted harnesses (2026-07-19): `UnslothHarness` (Python). Re-add when there's a concrete data/training need — Axolotl is sufficient until then.

## Configuration

| Variable | Description |
|----------|-------------|
| `RUNPOD_API_KEY` | Runpod API key |
| `RUNPOD_TEMPLATE_ID` | Runpod GPU pod template ID with axolotl pre-installed |
| `HKASK_PODS_FILE` | Path to RunPod pod ID persistence file (default: `data/training-pods.json`) — ensures orphaned pods can be terminated after restarts |

## lora-training skill integration

`training_validate_config` is the runtime enforcement point for the
[`.agents/skills/lora-training/`](../../.agents/skills/lora-training/SKILL.md)
skill's `audit-config` phase. The skill reasons over config files and proposes
regressions; this server enforces the static subset of gates at submit time
and emits the `cns.lora.*` spans the skill's `convergence-check` phase consumes.

## Quick Start

```bash
export RUNPOD_API_KEY="your-key"
export RUNPOD_TEMPLATE_ID="your-template-id"
# The server starts automatically with kask
kask chat
```
