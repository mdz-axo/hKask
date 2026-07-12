---
title: "Qwen3.6-27B Training Hyperparameters — Reference"
audience: [operators, developers, ml-engineers]
last_updated: 2026-07-10
version: "0.1.0"
status: "Active"
domain: "Training"
mds_categories: [domain]
---

# Qwen3.6-27B Training Hyperparameters — Reference

This document catalogs every hyperparameter choice for the Qwen3.6-27B distillation training pipeline, with provenance for each value. Use it to reproduce the training run or to understand the rationale when tuning.

## Model Loading

| Parameter | Value | Source | Rationale |
|-----------|-------|--------|-----------|
| `model_name` | `unsloth/Qwen3.6-27B` | Unsloth Hub | Unsloth-optimized BF16 checkpoint. Do NOT use the `-MTP-GGUF` or `-NVFP4` variants — those are inference-only, not trainable[^nvfp4-bug]. |
| `max_seq_length` | 4096 | [QevosAgent][^qevosagent] | Default. Auto-reduced to `P95_tokens × 1.2` if P95 < 2048. |
| `load_in_4bit` | `False` | [Unsloth Qwen3.5 guide][^unsloth-qlora] | QLoRA is not recommended for Qwen3.5/3.6 due to quantization error. |
| `load_in_16bit` | `True` | [Unsloth SFT guide][^unsloth-sft] | BF16 LoRA — the recommended training method for Qwen3.5/3.6. |
| `full_finetuning` | `False` | VRAM constraint | Full fine-tuning needs ~224GB. BF16 LoRA needs 56GB[^unsloth-vram]. |

## LoRA Configuration

| Parameter | Value | Source | Rationale |
|-----------|-------|--------|-----------|
| `r` | 64 | [rico03][^rico03] | Matches the only published Claude→Qwen3.6 distillation precedent. High end of the 16-64 range; provides capacity for complex reasoning transfer. |
| `lora_alpha` | 64 | [Unsloth defaults][^unsloth-sft] | α=r (standard ratio). Doubling to α=2r provides no benefit per Unsloth docs. |
| `lora_dropout` | 0 | [Unsloth kernels][^unsloth-kernels] | **Required.** Setting > 0 disables Unsloth's Triton kernel fusion, costing 2x speed. |
| `target_modules` | `[q,k,v,o,gate,up,down]_proj` | [Unsloth defaults][^unsloth-sft] | All 7 attention + MLP projections. Consensus across all published Qwen3.6 examples. `out_proj` inclusion (rico03) is an outlier. |
| `bias` | `none` | [Unsloth defaults][^unsloth-sft] | Bias terms are not trained in LoRA. |
| `use_gradient_checkpointing` | `unsloth` | [Unsloth docs][^unsloth-sft] | Unsloth's optimized gradient checkpointing; reduces VRAM by ~30%. |

## Training Configuration

| Parameter | Value | Source | Rationale |
|-----------|-------|--------|-----------|
| `learning_rate` | 2e-4 | [Unsloth SFT guide][^unsloth-sft] | Standard SFT learning rate for BF16 LoRA. The QevosAgent used 2e-5 for narrow-domain (Verilog) adaptation; our broad distillation task uses the SFT default. |
| `num_train_epochs` | 3 | [Unsloth SFT guide][^unsloth-sft] | 1-3 range. Upper end for complex task. Early stopping (patience=5) will halt early if loss plateaus. |
| `per_device_train_batch_size` | 1 | VRAM constraint | Single sample per step to fit 27B in 80GB. |
| `gradient_accumulation_steps` | 4 | VRAM constraint | Effective batch size = 4. |
| `warmup_ratio` | 0.05 | [QevosAgent][^qevosagent] | 5% of total steps. Matches all published Qwen3.6 examples. |
| `lr_scheduler_type` | `cosine` | [Unsloth SFT guide][^unsloth-sft] | Standard cosine decay. |
| `optim` | `adamw_8bit` | [Unsloth SFT guide][^unsloth-sft] | 8-bit AdamW for memory efficiency. |
| `weight_decay` | 0.01 | [Unsloth SFT guide][^unsloth-sft] | Light L2 regularization. |
| `max_grad_norm` | 0.3 | [Unsloth SFT guide][^unsloth-sft] | Gradient clipping for LoRA stability. |
| `bf16` | `True` | [QevosAgent][^qevosagent] | A100/H100 natively support BF16. |
| `eval_steps` | 200 | Heuristic | ~150 eval points across 3 epochs. Fine enough for early stopping (patience=5 evals = 1000 steps of no improvement). |
| `save_steps` | 400 | Must be multiple of eval_steps | 2× eval interval. Checkpoints every ~3% of an epoch. |
| `early_stopping_patience` | 5 | Heuristic | 5 evals × 200 steps = 1000 steps (~10% of an epoch) without improvement before halting. |
| `save_total_limit` | 3 | Disk constraint | Keep 3 most recent checkpoints. |

## Data Configuration

| Parameter | Value | Source | Rationale |
|-----------|-------|--------|-----------|
| Reasoning dataset | `Axolotl-Partners/qwen36-distill-opus-dsv4` | This project | 13,435 Claude Opus + DeepSeek v4 Pro distillation samples. All have `reasoning_content` on assistant turns. |
| Chat dataset | `mlabonne/FineTome-100k` | [Qwen3 docs][^qwen3-docs] | 100,000 ShareGPT conversations. Used for the 25% non-reasoning portion. |
| Reasoning ratio | 0.75 | [Qwen3 docs][^qwen3-docs] | 75/25 reasoning/chat to preserve thinking capabilities. |
| Fable-5 | **Removed** | This project | 4,600 agentic tool-use traces. Excluded because agentic tool-use reasoning may compete with mathematical decomposition reasoning. Flagged for ablation study. |

## Infrastructure Configuration

| Parameter | Value | Rationale |
|-----------|-------|-----------|
| Docker image | `runpod/pytorch:2.4.0-py3.11-cuda12.4.1-devel-ubuntu22.04` | Stable RunPod template with PyTorch 2.4.0 + CUDA 12.4. |
| `containerDiskInGb` | 60 | Docker image + pip packages only. Model and data go on volume. |
| `volumeInGb` | 200 | Model weights (~55GB) + HF cache (~5GB) + outputs (~1GB). |
| `PYTORCH_CUDA_ALLOC_CONF` | `expandable_segments:True` | Prevents fragmentation OOM — the #1 crash mode across Unsloth issues[^unsloth-oom]. |
| `HF_DEACTIVATE_ASYNC_LOAD` | `1` | Prevents system RAM explosion during weight loading[^unsloth-ram]. |
| `POD_INACTIVITY_TIMEOUT` | `14400` (4 hours) | Grace period for web terminal disconnection during training. |
| Self-management | GraphQL `podTerminate` mutation on success | Auto-terminates 60s after training completes (discovers pod ID via `hostname`). Stays alive on failure. Ctrl-C during countdown cancels. |

## Safety Gates (in execution order)

| Gate | When | Catches |
|------|------|---------|
| SDPA FlashAttn check | Before model download | CUDA/torch issues; warns if FlashAttention unavailable |
| Dataset validation | Before model download | Missing fields (`messages`, `conversations`), empty datasets |
| Disk space check | Before model download | Insufficient space for model + outputs |
| Token length analysis | After model load, before training | Over-provisioned `max_seq_length` (auto-adjusts) |
| Data format validation | Before training merge | ShareGPT conversion, chat template application, `assistant` presence |

[^unsloth-sft]: Unsloth. (2026). *LoRA fine-tuning Hyperparameters Guide*. https://unsloth.ai/docs/get-started/fine-tuning-llms-guide/lora-hyperparameters-guide

[^unsloth-qlora]: Unsloth. (2026). *Qwen3.5 Fine-tuning Guide*. https://unsloth.ai/docs/models/qwen3.5/fine-tune

[^unsloth-kernels]: Unsloth. (2026). *LoRA Hyperparameters Guide*. https://unsloth.ai/docs/get-started/fine-tuning-llms-guide/lora-hyperparameters-guide

[^unsloth-vram]: Unsloth. (2026). *Qwen3.5 Fine-tuning Guide*. https://unsloth.ai/docs/models/qwen3.5/fine-tune
    Qwen3.5-27B bf16 LoRA VRAM: 56GB.

[^unsloth-oom]: Unsloth. (2025). *Issue #2285: OOM on A100 for 4bit 7B model with batch size = 2*. https://github.com/unslothai/unsloth/issues/2285

[^unsloth-ram]: Unsloth. (2026). *Issue #4188: Extremely high CPU/VRAM usage and slow training with Qwen3.5*. https://github.com/unslothai/unsloth/issues/4188

[^qevosagent]: QevosAgent. (2026). *Fine-tuning Qwen3.6-27B for Verilog Code Generation with Unsloth*. https://qevos.ai/blog/en/2026-05-03-qwen36-verilog-lora-finetuning.html

[^rico03]: rico03. (2026). *Qwen3.6-27B-Claude-Opus-Reasoning-Distilled*. https://huggingface.co/rico03/Qwen3.6-27B-Claude-Opus-Reasoning-Distilled

[^qwen3-docs]: QwenLM. (2025). *Qwen3 — Training with Unsloth*. https://github.com/QwenLM/Qwen3/blob/main/docs/source/training/unsloth.md

[^nvfp4-bug]: Unsloth. (2026). *Issue #6023: Qwen3.6 NVFP4 model fails to load due to CompressedTensorsConfig / BitsAndBytesConfig conflict*. https://github.com/unslothai/unsloth/issues/6023
