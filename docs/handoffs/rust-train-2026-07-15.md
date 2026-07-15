# Handoff: Rust Adapter Training — 2026-07-15

## 1. Session Summary

Trained a PiSSA-initialized LoRA adapter for Qwen3.6-27B using Axolotl on RunPod. The adapter was trained on 838K blended examples (Rust coding + Rust analysis + reasoning), achieving eval_loss 0.1872. Uploaded to HuggingFace. Evaluated with quick generation tests showing the adapter produces concise Rust code but is overconfident on bug detection.

## 2. What Was Done

### Adapter trained and uploaded

| Field | Value |
|-------|-------|
| HF Repo | `Axolotl-Partners/qwen36-rust-reasoning-all-lora` |
| Base model | Qwen3.6-27B (fp16) |
| Best eval_loss | 0.1872 (step 8800) |
| Training time | ~26 hours |
| Total cost | ~$90 (including failed attempts) |
| Framework | Axolotl 0.17.0 + PiSSA + Liger Kernel + Cut Cross Entropy |
| LoRA config | r=16, alpha=32, 7 target modules, 79.7M trainable params (0.29%) |
| Datasets | 191K Rust coding + 533K Rust analysis + 114K reasoning (shuffled/blended) |
| Early stopping | patience=25, eval every 200 steps |

### Previous adapters (from prior session)

| Adapter | eval_loss | HF Repo |
|---------|-----------|---------|
| Rust Coding (Unsloth) | 1.128 | `Axolotl-Partners/qwen36-rust-coding-lora` |
| Rust Analysis (Unsloth) | 1.221 | `Axolotl-Partners/qwen36-rust-analysis-lora` |
| **Combined (Axolotl + PiSSA)** | **0.1872** | `Axolotl-Partners/qwen36-rust-reasoning-all-lora` |

### Migration from Unsloth to Axolotl

The session started with an Unsloth-based training script. After identifying speed issues (13 s/it with Xformers, no Flash Attention 2), we migrated to Axolotl which provided:
- Liger Kernel (fused MLP + RMSNorm)
- Cut Cross Entropy (faster loss)
- LoRA kernel optimizations (auto-enabled)
- SDPA attention (PyTorch native flash attention on H100)
- PiSSA support via `peft_init_lora_weights: pissa_niter_4`

Speed improved from 13 s/it (Unsloth) to 8-10 s/it (Axolotl with optimizations).

### PiSSA initialization

PiSSA (`pissa_niter_4`) gave dramatic convergence speedup:
- Loss dropped from 1.386 to 0.232 in 200 steps (vs 0.25 without PiSSA)
- 30-50% faster convergence confirmed
- SVD initialization runs silently during model load (~2-5 min for 27B)
- Must keep `lora_dropout: 0` (random dropout discards principal components)

### Early stopping patience

Key finding: patience=7 is too aggressive for a cosine LR schedule spanning 146,908 steps. The LR stays at 99%+ of peak for the first ~5% of training. With patience=7, training would have stopped at ~step 3,400 with eval_loss 0.225. With patience=25, the model continued improving to 0.1872 — a 17% improvement that would have been lost.

The pattern: the model oscillates ±0.003 around a slowly decreasing trend. Every 5-7 evals, it finds a new best. Patience=25 rides these waves correctly.

### Eval/quick test results

Tested the adapter on a simple Rust code generation prompt:

| Aspect | Baseline (no adapter) | With adapter |
|--------|----------------------|--------------|
| Code generation | Stuck thinking, never produces code (runs out of tokens) | **Correct Rust code in 20s** |
| Bug detection | Correctly identifies 2 bugs (division by zero, integer truncation) | **Says "no bug found" — wrong** |
| Response length | 3,400-5,400 chars | 470-760 chars |
| Generation time | 77-112s | 10-20s (4x faster) |

**The adapter is good at**: producing concise, working Rust code quickly
**The adapter is bad at**: deep analysis tasks (bug detection, code review) — it's overconfident and misses issues

### QLoRA attempt (failed)

Attempted QLoRA (4-bit quantization) for 8x batch size speedup. Failed due to:
1. Axolotl upgraded PyTorch to 2.13.0+cu130, but RunPod image has CUDA 12.4 toolkit
2. `libnvJitLink.so.13` missing (CUDA 13.0 runtime library not installed)
3. bitsandbytes fails to 4-bit quantize Qwen3.6-27B's vision module (`model.visual.merger.linear_fc2.weight`)
4. Qwen3.6-27B is a multimodal (vision-language) model — bitsandbytes can't handle the vision weights

### Guide written

`docs/how-to/axolotl-pissa-runpod-guide.md` — comprehensive guide with all lessons learned, uploaded to `Axolotl-Partners/rust-adapter-scripts` on HF.

## 3. Key Decisions to Preserve

1. **Axolotl over Unsloth** — Axolotl provides Liger Kernel, Cut Cross Entropy, LoRA kernel optimizations, and better config management. Unsloth lacks these.
2. **PiSSA initialization** — `peft_init_lora_weights: pissa_niter_4` gives 30-50% faster convergence. Free lunch — just a few minutes of SVD at init.
3. **patience=25** — The cosine LR schedule spans 146K steps. patience=7 stops too early (eval_loss 0.225 vs 0.187 with patience=25).
4. **SDPA over flash-attn** — `flash_attention: false` uses PyTorch's native SDPA which works on H100. flash-attn can't compile on RunPod due to CUDA version mismatches.
5. **No sample_packing without flash-attn** — Sample packing requires flash-attn for cross-sample attention masking. Without it, attention leaks between packed examples.
6. **HF_HOME must be set in SSH sessions** — Pod environment variables are NOT inherited by SSH. Without `HF_HOME=/workspace/.cache/huggingface`, caches go to the 60GB container disk and fill it up.
7. **gradient_checkpointing + eval_batch_size=1** — Essential for 27B model on single H100 (93GB). Without these, OOM during eval.
8. **QLoRA fails on Qwen3.6-27B** — The model has a vision module that bitsandbytes can't quantize. QLoRA is not viable for this model without excluding the vision module.
9. **Adapter init_lora_weights must be changed for inference** — The adapter_config.json has `init_lora_weights: pissa_niter_4`. When loading on a 4-bit model, PEFT tries to re-run PiSSA SVD and fails. Fix: change to `init_lora_weights: true` in the config before loading.
10. **Adapter is overconfident on analysis tasks** — Trained on "direct response" data format, the adapter produces concise code but misses bugs. Use the base model for analysis tasks.

## 4. What Remains

### HIGH — Improve adapter for analysis tasks

The adapter misses bugs and says code is correct when it's not. Options:
- Train on more bug detection examples with deeper reasoning
- Use a higher LoRA rank (r=32 or r=64) for more capacity
- Train with a system prompt that emphasizes thoroughness
- Use the base model (without adapter) for analysis tasks, and the adapter only for code generation

### MEDIUM — Run full eval on 225 Strandset test examples

The quick test showed the adapter works for code generation. A full eval on all 225 test examples across 15 categories would give per-category accuracy. The eval script is ready (`scripts/eval_rust_adapter.sh`) but takes ~18 hours at 146 s/example. Needs batched inference or vLLM for practical eval times.

### MEDIUM — Fix QLoRA for faster training

QLoRA would give 8x batch size (14GB vs 54GB model). Needs:
- A text-only version of Qwen3.6-27B (without vision module)
- Or a bitsandbytes config that excludes vision modules from quantization
- Or a RunPod image with CUDA 13.0 toolkit

### LOW — Try flash-attn with sample_packing

Would give 3-4x additional throughput on top of QLoRA. Needs:
- A RunPod image with matching CUDA toolkit version
- Or a pre-built flash-attn wheel for PyTorch 2.13 + CUDA 13.0

## 5. Files and Artifacts

### HuggingFace
- **Adapter**: `Axolotl-Partners/qwen36-rust-reasoning-all-lora` (eval_loss 0.1872)
- **Scripts**: `Axolotl-Partners/rust-adapter-scripts` (training scripts, eval script, guide)
- **Datasets**: `Axolotl-Partners/rust-datasets-linked`, `Axolotl-Partners/openthoughts-114k-linked`

### Local files (in hKask repo)
- `scripts/axolotl_rust_all.yml` — Axolotl config (fp16 LoRA, what we trained with)
- `scripts/axolotl_qlora_warmstart.yml` — QLoRA config (failed, needs fixes)
- `scripts/preprocess_rust_datasets.py` — Dataset preprocessing (raw → ChatML JSONL)
- `scripts/multi_test.py` — Quick eval script (baseline vs adapter comparison)
- `scripts/eval_rust_adapter.sh` — Full eval script (225 examples, 15 categories)
- `scripts/train_rust_adapter.sh` — Original Unsloth training script (deprecated)
- `scripts/runpod_unsloth.sh` — RunPod pod launcher
- `docs/how-to/axolotl-pissa-runpod-guide.md` — Comprehensive guide with lessons learned

### Training config (final, what produced the adapter)
```yaml
base_model: unsloth/Qwen3.6-27B
adapter: lora
load_in_4bit: false
sequence_len: 6144
bf16: true
lora_r: 16
lora_alpha: 32
lora_dropout: 0
peft_init_lora_weights: pissa_niter_4
lora_target_modules: [q_proj, k_proj, v_proj, o_proj, gate_proj, up_proj, down_proj]
micro_batch_size: 1
eval_batch_size: 1
gradient_accumulation_steps: 16
gradient_checkpointing: true
lr_scheduler: cosine
learning_rate: 1e-4
warmup_steps: 100
num_epochs: 3
val_set_size: 0.0012
eval_steps: 200
save_steps: 200
save_total_limit: 5
early_stopping_patience: 25
liger_kernel: true
flash_attention: false
cut_cross_entropy: true
optim: adamw_8bit
```

## 6. Cost Summary

| Activity | Time | Cost |
|----------|------|------|
| Unsloth training (killed, eval too slow) | 7h | $22 |
| Axolotl migration + failed attempts (OOM, disk, deps) | 4h | $13 |
| Axolotl + PiSSA training (successful) | 26h | $83 |
| QLoRA attempt (failed) | 2h | $6 |
| Eval/quick test | 2h | $6 |
| **Total** | **~41h** | **~$130** |

For future runs, the guide at `docs/how-to/axolotl-pissa-runpod-guide.md` should help avoid the ~$40 of failed attempt costs.