# Test Run Plan — Verify Training Fixes Before Spending Real Money

**Goal**: Verify all 5 HIGH fixes work end-to-end with a $5-10 test run before committing to a full $600+ training job.

## Prerequisites

```bash
# Ensure API keys are set (already in .env)
export RUNPOD_API_KEY="your-key"
export RUNPOD_TEMPLATE_ID="your-template-id"
export HF_TOKEN="your-token"

# Ensure the training server compiles
cargo clippy -p hkask-mcp-training -- -D warnings
cargo clippy -p hkask-adapter -- -D warnings
```

## Step 1: $5 Verification Run (15 minutes on H100)

### What this verifies

| Fix | What to check |
|---|---|
| H2 (pod persistence) | Pod ID appears in `data/training-pods.json` after submit |
| H2 (drain_all_pods) | Running `drain_all_pods` terminates the pod via GraphQL |
| H5 (EVA config) | Axolotl loads with `peft_init_lora_weights: eva` without error |
| H4 (grad accum) | Not directly testable here (Tinker-only fix), but axolotl grad_accum works |
| H3 (Together poll) | Not testable here (RunPod-only run), but env vars are ready |

### Procedure

1. **Create a minimal dataset** (100 examples from the full corpus):
```bash
head -n 100 /path/to/train_chat_full.jsonl > /tmp/test_dataset.jsonl
wc -l /tmp/test_dataset.jsonl  # should be 100
```

2. **Create a minimal axolotl config** based on `corpus/lora/axolotl-lora.yaml` but with:
   - `num_epochs: 1` (not 2)
   - `val_set_size: 0.1` (10% of 100 = 10 eval examples)
   - `eval_steps: 10`
   - `early_stopping_patience: 3` (don't wait long)
   - `save_steps: 10`
   - `save_total_limit: 1`
   - Same base model: `unsloth/Qwen3.6-27B`
   - Same EVA init: `peft_init_lora_weights: eva`

3. **Submit the training job** via the MCP tool or CLI:
```bash
# Via kask CLI (if available):
kask training submit --dataset-path /tmp/test_dataset.jsonl --base-model unsloth/Qwen3.6-27B

# Or via the MCP tool directly (if the server is running):
# training_submit with dataset_path="/tmp/test_dataset.jsonl", base_model="unsloth/Qwen3.6-27B"
```

4. **Verify pod persistence**:
```bash
cat data/training-pods.json
# Should contain: {"<job_id>": "<pod_id>"}
```

5. **Monitor training status**:
```bash
# Poll until completed or failed:
kask training status --job-id <job_id>
```

6. **Verify drain_all_pods works** (if training is still running, or after completion):
```bash
# The pod should be terminated via GraphQL podTerminate
# Check RunPod console to confirm the pod is terminated
```

7. **Check CNS spans** (if tracing is enabled):
```bash
# Look for cns.training.provider.runpod.* spans in the logs
grep "cns.training.provider.runpod" /path/to/logs
```

### Expected cost

- H100 NVL at ~$3.19/hr
- 100 examples, 1 epoch, ~10 steps at ~10s/step = ~2 min training + ~5 min setup = ~7 min
- Cost: ~$0.37

### If something fails

| Symptom | Likely cause | Fix |
|---|---|---|
| Pod not created | `RUNPOD_API_KEY` or `RUNPOD_TEMPLATE_ID` not set | Check `.env` |
| Pod created but training fails | Axolotl config error (EVA init not supported by current PEFT version) | Fall back to standard LoRA init (remove `peft_init_lora_weights: eva`) |
| Pod ID not in `data/training-pods.json` | File path issue | Check `HKASK_PODS_FILE` env var |
| Pod keeps billing after cancel | `podTerminate` GraphQL mutation failed | Check RunPod console manually |

## Step 5: Full Training Run ($35-80)

Once the $5 verification passes, run the full training:

1. **Use the canonical config**: `corpus/lora/axolotl-lora.yaml` (EVA init, r=32, patience=25, val_set_size=0.05)
2. **Full dataset**: `mdz-axo/capabilities-researcher-qa` / `train_chat_full.jsonl` (229,520 examples)
3. **Expected duration**: ~26-55h on H100 (per `docs/how-to/axolotl-pissa-runpod-guide.md`)
4. **Expected cost**: ~$35-80 (per the guide's cost estimates)
5. **Monitor**: `training_status` every 30 min; watch `eval_loss` trajectory
6. **On completion**: `training_status` auto-registers the adapter; then `training_deploy` to deploy

### Key lessons to apply (from the guide)

- `flash_attention: false` (SDPA, no flash-attn compile needed)
- `sample_packing` disabled (requires flash-attn for cross-sample masking)
- `gradient_checkpointing: true` (essential for 27B on single H100)
- `eval_batch_size: 1` (prevents OOM during eval)
- `early_stopping_patience: 25` (not 7 — cosine LR needs runway)
- `lora_dropout: 0` (required for EVA — activation components must not be dropped)
- `save_total_limit: 5` (preserves best checkpoint)
- `val_set_size: 0.05` (5% gives statistically meaningful eval signal)

### After training completes

1. Download adapter weights from the RunPod pod
2. Upload to HuggingFace Hub
3. Register the adapter: `training_register_adapter`
4. Deploy: `training_deploy` with provider=together or runpod
5. Evaluate: `training_evaluate` against the test split
6. If eval_loss is good, the adapter is ready for inference