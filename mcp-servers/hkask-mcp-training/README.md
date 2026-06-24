# hkask-mcp-training

Model training MCP server — ingests QA pairs and training data for fine-tuning pipelines.

Uses internal tool dispatch pattern (not individual `pub async fn` per tool).

## Tools (21)

| Tool | Description |
|------|-------------|
| `training_ingest_qa` | Ingest QA pairs for model training. Stores question-answer pairs with provenance in semantic memory for future fine-tuning dataset assembly |
| `training_assemble_dataset` | Assemble stored QA pairs into a ChatML JSONL training dataset file. Queries semantic memory for training_qa_pair triples, filters by dataset/source/bloom level, and writes a file ready for training_submit. Optionally splits into train/test |
| `training_generate_traces` | Generate decomposition traces from a skill document for LoRA fine-tuning. Uses the inference engine to produce varied scenario→decomposition→synthesis training examples in ChatML format |
| `training_generate_chain_of_thought` | Generate chain-of-thought training traces with multi-step reasoning. Produces ChatML traces where each assistant turn represents one reasoning step (r1→r2→r3→conclusion) |
| `training_ingest_dataset` | Ingest a raw dataset file into the normalized cache without submitting a training job. Detects format (ChatML, ShareGPT, Alpaca, raw text), normalizes to canonical ChatML, validates, and caches |
| `training_submit` | Submit a training job for execution. Ingests, normalizes, and submits a dataset for LoRA fine-tuning via the configured host (axolotl or unsloth) |
| `training_sweep` | Submit a parameter sweep across learning rates, LoRA ranks, batch sizes, and epochs. All combinations submitted as separate jobs. Use training_status to track results |
| `training_status` | Query the status of a training job by its ID. When a job completes, automatically registers the adapter in the persistent store if not already registered |
| `training_cancel` | Cancel a running or queued training job |
| `training_retrain` | Retrain an adapter with curated feedback for continuous skills training. Merges the original training dataset with a feedback JSONL file, submits a new training job with an incremented version number, and registers the new adapter on completion. Closes the continuous training loop: train → evaluate → curate → retrain |
| `training_evaluate` | Evaluate a trained adapter against a test dataset. Runs inference for each test example and scores accuracy using exact match, substring containment, or semantic comparison |
| `training_register_adapter` | Register a completed LoRA adapter in the persistent store. Call after training completes to record adapter metadata for future listing, evaluation, and composition |
| `training_list_adapters` | List all completed LoRA adapters available for model composition |
| `training_delete_adapter` | Delete a LoRA adapter and all associated artifacts |
| `training_merge_adapters` | Merge multiple LoRA adapters into a single composite adapter for multi-skill inference. Uses weighted averaging with optional TIES density filtering |
| `training_recommend_model` | Recommend a base model for fine-tuning based on task type, budget, latency, and license requirements. Returns ranked recommendations with rationale |
| `training_deploy` | Deploy a trained adapter to a cloud inference endpoint. Looks up the adapter by name from AdapterStore, resolves the base model, estimates cost and setup time per provider, and provisions or locates an endpoint |
| `training_deployment_status` | Check the status of a deployed adapter endpoint. Returns current provisioning state, endpoint URL when ready, and accumulated cost |
| `training_teardown` | Tear down a deployed adapter endpoint. Stops the cloud inference endpoint and releases GPU resources |
| `training_record_invocation` | Record an adapter invocation as an episodic experience for future training data curation. Stores input/output summaries with CNS span correlation and confidence |
| `training_curate_feedback` | Curate feedback from stored QA pairs for continuous skills training. Queries semantic memory for training_qa_pair triples, validates each answer with inference, and generates corrected ChatML traces where the original answer is wrong or incomplete |

## Configuration

| Variable | Description |
|----------|-------------|
| `TOGETHER_API_KEY` | Together AI API key |
| `BASETEN_API_KEY` | Baseten API key |
| `RUNPOD_API_KEY` | Runpod API key |

## Quick Start

```bash
export TOGETHER_API_KEY="your-key"
# The server starts automatically with kask
kask chat
```
