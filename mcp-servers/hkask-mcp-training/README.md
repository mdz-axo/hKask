# hkask-mcp-training

Model training MCP server — ingests QA pairs and training data for fine-tuning pipelines.

Uses internal tool dispatch pattern (not individual `pub async fn` per tool).

## Tools (15)

Per `docs/OPEN_QUESTIONS.md` §8.2: 15 tools fully implemented across 5 providers (Together AI, Baseten, Runpod, Axolotl, Unsloth).

| Category | Tools |
|----------|-------|
| **Data** | `training_generate_traces`, `training_assemble_dataset`, `training_ingest_dataset` |
| **Training** | `training_submit`, `training_status`, `training_cancel`, `training_retrain` |
| **Evaluation** | `training_evaluate` |
| **Registry** | `training_register_adapter`, `training_list_adapters`, `training_recommend_model` |
| **Feedback** | `training_record_invocation`, `training_curate_feedback` |

## Configuration

| Variable | Description |
|----------|-------------|
| `TOGETHER_API_KEY` | Together AI API key |
| `BASETEN_API_KEY` | Baseten API key |
| `RUNPOD_API_KEY` | Runpod API key |
