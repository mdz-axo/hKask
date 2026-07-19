# hkask-mcp-training

Model training MCP server — ingests QA pairs and training data for fine-tuning pipelines.

Uses internal tool dispatch pattern (not individual `pub async fn` per tool).

## Tools (14)

Simplified from 21 tools on 2026-07-19 — 7 speculative/inference/dup tools deleted
(`generate_traces`, `generate_chain_of_thought`, `sweep`, `merge_adapters`,
`record_invocation`, `curate_feedback`, `recommend_model`).

| Tool | Description |
|------|-------------|
| `training_ingest_qa` | Ingest QA pairs for model training. Stores question-answer pairs with provenance in semantic memory for future fine-tuning dataset assembly |
| `training_assemble_dataset` | Assemble stored QA pairs into a ChatML JSONL training dataset file. Queries semantic memory for training_qa_pair triples, filters by dataset/source/bloom level, and writes a file ready for training_submit. Optionally splits into train/test |
| `training_ingest_dataset` | Ingest a raw dataset file into the normalized cache without submitting a training job. Detects format (ChatML, ShareGPT, Alpaca, raw text), normalizes to canonical ChatML, validates, and caches |
| `training_submit` | Submit a training job for execution. Ingests, normalizes, and submits a dataset for LoRA fine-tuning via the configured host (axolotl or unsloth) |
| `training_status` | Query the status of a training job by its ID. When a job completes, automatically registers the adapter in the persistent store if not already registered |
| `training_cancel` | Cancel a running or queued training job |
| `training_retrain` | Retrain an adapter with curated feedback for continuous skills training. Merges the original training dataset with a feedback JSONL file, submits a new training job with an incremented version number, and registers the new adapter on completion |
| `training_evaluate` | Evaluate a trained adapter against a test dataset. Runs inference for each test example and scores accuracy using exact match, substring containment, or semantic comparison |
| `training_preflight_check` | Run pre-flight checks on a trained LoRA adapter before evaluation or deployment. Verifies adapter_config.json, adapter_model.safetensors, and optional sanity check |
| `training_register_adapter` | Register a completed LoRA adapter in the persistent store. Call after training completes to record adapter metadata for future listing, evaluation, and composition |
| `training_list_adapters` | List all completed LoRA adapters available for model composition |
| `training_delete_adapter` | Delete a LoRA adapter and all associated artifacts |
| `training_deploy` | Deploy a trained adapter to a cloud inference endpoint. Looks up the adapter by name from AdapterStore, resolves the base model, estimates cost and setup time per provider, and provisions or locates an endpoint |
| `training_deployment_status` | Check the status of a deployed adapter endpoint. Returns current provisioning state, endpoint URL when ready, and accumulated cost |
| `training_teardown` | Tear down a deployed adapter endpoint. Stops the cloud inference endpoint and releases GPU resources |

## Configuration

| Variable | Description |
|----------|-------------|
| `TG_API_KEY` | Together AI API key |
| `RUNPOD_API_KEY` | Runpod API key |
| `RUNPOD_TEMPLATE_ID` | Runpod GPU pod template ID with axolotl pre-installed |
| `TINKER_API_KEY` | Thinking Machines Tinker API key |
| `HKASK_PODS_FILE` | Path to RunPod pod ID persistence file (default: `data/training-pods.json`) — ensures orphaned pods can be terminated after restarts |
| `TOGETHER_POLL_MAX_ATTEMPTS` | Max poll attempts for Together AI fine-tune jobs (default: 720 = 2h at 30s intervals) |
| `TOGETHER_POLL_INTERVAL_SECS` | Together AI poll interval in seconds (default: 30) |

## Quick Start

```bash
export TG_API_KEY="your-key"
# The server starts automatically with kask
kask chat
```