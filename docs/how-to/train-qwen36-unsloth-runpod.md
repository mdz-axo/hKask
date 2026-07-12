---
title: "How to Train Qwen3.6-27B on RunPod with Unsloth"
audience: [operators, developers, ml-engineers]
last_updated: 2026-07-10
version: "0.1.0"
status: "Active"
domain: "Training"
mds_categories: [domain, lifecycle]
last-verified-against: "scripts/train_unsloth.sh"
---

# How to Train Qwen3.6-27B on RunPod with Unsloth

**Goal:** Fine-tune a BF16 LoRA adapter for Qwen3.6-27B on distillation data using a single A100 or H100 GPU on RunPod, then auto-upload the adapter to HuggingFace.

**Prerequisites:** RunPod API key in `.env`, HuggingFace write token in `.env`, SSH public key added to RunPod account settings[^runpod-settings].

## 1. Launch the Pod

From the project root:

```bash
bash scripts/runpod_unsloth.sh
```

This first tries a secure-cloud H100 NVL, then falls back to a community-cloud A100 PCIe 80GB if the H100 is unavailable, with:
- 60GB container disk, 200GB `/workspace` volume
- Environment variables: `HF_TOKEN`, `PYTORCH_CUDA_ALLOC_CONF`, `HF_DEACTIVATE_ASYNC_LOAD`
- 4-hour inactivity timeout
- Web terminal + SSH access

The script prints a web terminal URL. Open it in your browser.

## 2. Start Training

Paste ONE command in the web terminal:

```bash
curl -sL https://huggingface.co/datasets/Axolotl-Partners/qwen36-distill-opus-dsv4/raw/bfedff55f47bcf0286ff49584635e25912147c97/train_unsloth.sh | bash
```

The command returns immediately — all output is redirected to `/workspace/training.log`. Monitor progress:

```bash
tail -f /workspace/training.log
```

## 3. What Happens

The script executes these stages automatically:

| Stage | Duration | What to Check |
|-------|----------|---------------|
| Dependency install | 3-5 min | `Install complete.` |
| SDPA FlashAttn check | < 5s | `SDPA-FlashAttn: True` — if False, VRAM will be tighter |
| Dataset validation | 1-2 min | All 3 checks must show `OK` |
| Model download | 15-30 min | `Loading unsloth/Qwen3.6-27B...` then GPU memory report |
| Token length analysis | 2-5 min | P50, P95, max reported; seq may auto-adjust |
| Training | 20-40h | Loss numbers every 10 steps; eval every 200 steps |
| Early stopping | Automatic | If eval_loss doesn't improve for 5 evals (=1000 steps) |
| Save + Upload | 3-10 min | Hugging Face client retries transient failures |
| Pod cleanup | 60s | Auto-terminates on success; stays alive on failure |

## 4. Training Configuration

The default hyperparameters are aligned with Unsloth's SFT recommendations[^unsloth-sft] and published Qwen3.6-27B training examples[^qevosagent]:

| Parameter | Value | Rationale |
|-----------|-------|-----------|
| Model | `unsloth/Qwen3.6-27B` | Unsloth-optimized BF16 checkpoint |
| Method | BF16 LoRA (not QLoRA) | QLoRA not recommended for Qwen3.5/3.6[^unsloth-qlora] |
| LoRA Rank | 16 | Lower rank reduces overfit risk on small datasets |
| LoRA Alpha | 32 | α=2r (standard scaling ratio) |
| LoRA Dropout | 0 | Required for Unsloth kernel fusion[^unsloth-kernels] |
| Learning Rate | 1e-4 | Conservative for small datasets; prevents overfit |
| Max Seq Length | 6144 | Auto-adjusted down if P95 tokens < 3072 |
| Epochs | 3 | With early stopping (patience=10 evals) |
| Batch Size | 1 × 4 accumulation = 4 | Fits 80GB VRAM |
| Warmup | 50 steps | Fixed steps — avoids wasting training on excessive warmup |
| Eval Steps | 50 | Frequent eval to catch best checkpoint early |
| Data Ratio | 75% reasoning / 25% chat | Preserves thinking capabilities[^qwen3-docs] |

## 5. Output

On success:
- LoRA adapter weights uploaded to `Axolotl-Partners/qwen36-distill-opus-dsv4-lora`
- Training log preserved in `/workspace/training.log`
- Pod auto-terminates (60s grace period, Ctrl-C to cancel)

On failure:
- Error message logged with exit code
- Pod kept alive for debugging
- `/workspace/outputs` may contain partial checkpoints

## 6. Post-Training

After training completes, evaluate the adapter:

1. Download from HF: `huggingface-cli download Axolotl-Partners/qwen36-distill-opus-dsv4-lora`
2. Merge LoRA: `model.merge_and_unload()` in Unsloth
3. Run benchmarks: GPQA Diamond, MATH-500, MMLU-Pro
4. Manual review: inspect 20-30 thinking traces for quality

See the [Qwen3.6 Training Reference](../reference/qwen36-training-hyperparameters.md) for the full hyperparameter rationale and literature survey.

## 7. Cost

| GPU | Spot Rate | Est. Time | Est. Cost |
|-----|-----------|-----------|-----------|
| A100 PCIe 80GB | ~$1.19/hr | 30-40h | ~$35-48 |
| A100 SXM 80GB | ~$1.39/hr | 25-35h | ~$35-49 |
| H100 NVL 80GB | ~$2.59/hr | 10-15h | ~$26-39 |

Costs are approximate. Spot prices fluctuate. The pod auto-terminates on completion, so you never pay for idle time.

[^runpod-settings]: RunPod. (2026). *Account Settings*. https://www.runpod.io/console/user/settings
    SSH public keys must be registered for `ssh.runpod.io` proxy access.

[^unsloth-sft]: Unsloth. (2026). *LoRA fine-tuning Hyperparameters Guide*. https://unsloth.ai/docs/get-started/fine-tuning-llms-guide/lora-hyperparameters-guide
    Default SFT parameters: lora_dropout=0, lora_alpha=r, learning_rate=2e-4, warmup=3-5%.

[^unsloth-qlora]: Unsloth. (2026). *Qwen3.5 Fine-tuning Guide*. https://unsloth.ai/docs/models/qwen3.5/fine-tune
    "It is not recommended to do QLoRA (4-bit) training on the Qwen3.5 models, no matter MoE or dense, due to higher than normal quantization differences."

[^qevosagent]: QevosAgent. (2026). *Fine-tuning Qwen3.6-27B for Verilog Code Generation with Unsloth*. https://qevos.ai/blog/en/2026-05-03-qwen36-verilog-lora-finetuning.html
    Published A100 training run: r=32, alpha=64, max_seq=4096, LR=2e-5, epochs=3, warmup_ratio=0.05, BF16.

[^rico03]: rico03. (2026). *Qwen3.6-27B-Claude-Opus-Reasoning-Distilled*. https://huggingface.co/rico03/Qwen3.6-27B-Claude-Opus-Reasoning-Distilled
    Claude Opus reasoning distillation into Qwen3.6-27B using Unsloth with LoRA rank 64.

[^unsloth-kernels]: Unsloth. (2026). *LoRA Hyperparameters Guide*. https://unsloth.ai/docs/get-started/fine-tuning-llms-guide/lora-hyperparameters-guide
    "Critical: Unsloth requires `lora_dropout=0` for kernel fusion optimization. Setting dropout > 0 disables the custom kernels and you lose the speed advantage."

[^qwen3-docs]: QwenLM. (2025). *Qwen3 — Training with Unsloth*. https://github.com/QwenLM/Qwen3/blob/main/docs/source/training/unsloth.md
    "To retain Qwen3's reasoning capabilities, use a 75% reasoning to 25% non-reasoning dataset ratio."
