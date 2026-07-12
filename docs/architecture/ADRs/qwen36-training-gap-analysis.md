# Qwen3.6-27B Training — Gap Analysis vs Reference

**Date:** 2026-07-10 | **Reference:** `docs/reference/qwen36-training-hyperparameters.md`

## Parameters Fixed ✅

| Parameter | Was | Now (matches reference) |
|-----------|-----|------------------------|
| `lora_dropout` | Not set (defaults to 0.05) | **0** — required for Unsloth Triton kernels |
| `target_modules` | 4 attn layers only | **7 layers** — added gate/up/down MLP projections |
| `gradient_checkpointing` | Not set | **unsloth** — 30% VRAM savings |
| `lr_scheduler_type` | Not set | **cosine** |
| `optim` | Not set | **adamw_8bit** |
| `weight_decay` | Not set | **0.01** |
| `max_grad_norm` | Not set | **0.3** |
| `eval_steps` | Not set | **200** |
| `save_steps` | Not set | **400** |
| `early_stopping_patience` | Not set | **5** |
| `save_total_limit` | Not set | **3** |
| `max_seq_length` | 2048 | **4096** |
| `gradient_accumulation_steps` | 16 | **4** (effective batch 4) |
| `learning_rate` | 1.0e-4 | **2.0e-4** (standard SFT default) |
| Environment (`PYTORCH_CUDA_ALLOC_CONF`, `HF_DEACTIVATE_ASYNC_LOAD`) | Not set | **Added** — prevents #1 and #2 OOM crash modes |

## Risks Still Open 🟡

| Risk | Detail | Mitigation |
|------|--------|------------|
| **Reasoning loss** | We have 100% domain QA (no chat mixing). Reference uses 75/25 reasoning/chat split. Without chat data, the model may lose general reasoning capabilities from the base model. | Mix `mlabonne/FineTome-100k` at 25% ratio, OR accept domain-specialization tradeoff (Company Researcher doesn't need general chat). |
| **Small dataset** | 2,550 training QAs vs 13,435 in reference. Token count ~1.5M vs ~5M+ in reference. | Monitor loss curves closely. If underfitting, augment with more generated QAs or reduce epochs to 1-2. |
| **No reasoning_content** | Our QAs don't have `reasoning_content` on assistant turns like the distillation dataset. This means the model won't learn to "think" before answering. | For Company Researcher use case, this may be acceptable — direct investment answers don't require visible reasoning chains. |
| **No safety gates** | Reference has 5 pre-training validation gates (FlashAttn, dataset, disk, token analysis, format). Our script has none. | Add validation checks to `pipeline_bloom.sh` before training submission. |

## Recommended Next Steps

1. **Run training** with current config — the parameters are now reference-aligned
2. **Monitor** loss curves via eval_steps=200
3. **A/B test**: one run with 100% domain QA, one with 75/25 mix — compare MT-Bench / AlpacaEval scores
4. **Add safety gates** to pipeline script (pre-training validation)
