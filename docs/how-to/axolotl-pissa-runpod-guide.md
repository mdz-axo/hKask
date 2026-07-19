# Prompt: Train a LoRA Adapter with Axolotl + PiSSA on RunPod

## Objective

Train a PiSSA-initialized LoRA adapter on a large language model (e.g., Qwen3.6-27B) using Axolotl on a RunPod H100 GPU pod. The adapter should blend multiple datasets (coding, analysis, reasoning) with shuffled mixing, Liger Kernel optimizations, and sufficient early-stopping patience to let the cosine LR schedule work.

## Prerequisites

- RunPod account with API key and SSH key configured
- HuggingFace token with write access to the target model repo
- Training scripts from HuggingFace (`Axolotl-Partners/rust-adapter-scripts` — curl-piped to RunPod pods)
- `.env` file with `RUNPOD_API_KEY=` and `HF_TOKEN=`

## Step 1: Launch the Pod

Use the HF-hosted `runpod_unsloth.sh --rust-all` launcher or deploy manually. Target: H100 NVL (93GB VRAM, ~$3.19/hr). The pod's container disk is only 60GB — all caches MUST go to the 200GB+ workspace volume.

**Critical environment variables** (set these in the pod's env config AND when running commands via SSH):

```bash
export HF_HOME=/workspace/.cache/huggingface      # Model + dataset cache on workspace
export PIP_CACHE_DIR=/workspace/.cache/pip          # Pip cache on workspace
export TMPDIR=/workspace/tmp                         # Temp files on workspace
export PYTORCH_CUDA_ALLOC_CONF=expandable_segments:True  # Reduce GPU fragmentation
export HF_TOKEN=<your_token>
```

**Lesson learned**: SSH sessions do NOT inherit pod environment variables. You must explicitly `export` them in every `nohup bash -c '...'` command. Without `HF_HOME` set, HuggingFace caches to the 60GB container disk at `/root/.cache/huggingface/`, which fills up immediately during dataset tokenization and causes `No space left on device` → SIGSEGV crash.

## Step 2: Install Axolotl

```bash
pip install --cache-dir /workspace/.cache/pip axolotl
```

This pulls in `liger-kernel`, `transformers` (v5+), `trl`, `accelerate`, `bitsandbytes`, and other dependencies. It will conflict with any pre-installed Unsloth (different transformers/trl version requirements) — that's expected and harmless if you're only using Axolotl.

**Do NOT install `flash-attn`** — compiling from source takes 20-30 minutes and may fail due to CUDA version mismatches. Instead, use PyTorch's native SDPA attention, which uses flash attention internally on H100.

**If the RunPod image already has `causal-conv1d` and `flash-linear-attention` installed** (for Qwen3.5+ hybrid linear attention models), verify they import correctly:

```bash
python3 -c "import fla, tilelang; print('FLA kernels OK')"
```

## Step 3: Preprocess Datasets to ChatML JSONL

Axolotl's `type: chat_template` expects datasets with a `messages` field in OpenAI Messages format:

```json
{"messages": [
  {"role": "system", "content": "..."},
  {"role": "user", "content": "..."},
  {"role": "assistant", "content": "..."}
]}
```

If your raw datasets have custom fields (e.g., `task_category`, `input_data`, `output_data`), you MUST preprocess them to JSONL with `messages` fields before training. Write a preprocessing script that loads each dataset, applies your formatting functions, and saves as `.jsonl` files to `/workspace/data/`.

**Lesson learned**: Axolotl does not apply custom formatting functions. The `field_messages` parameter only tells Axolotl which field to read — it doesn't convert non-messages formats. Preprocessing is required.

Use `num_proc=8` or higher in the `.map()` calls for fast preprocessing (~5 min for 700K examples).

## Step 4: Create the Axolotl Config YAML

```yaml
# Key parameters that matter:

base_model: unsloth/Qwen3.6-27B       # Use the exact cached model ID to avoid re-download
adapter: lora
load_in_4bit: false                    # fp16 for PiSSA SVD initialization
load_in_8bit: false

sequence_len: 6144
bf16: true

# PiSSA — SVD-based LoRA initialization from principal singular values.
# Gives 30-50% faster convergence. The adapter starts in the meaningful
# subspace instead of random init. Loss drops from 1.4 to 0.23 in 200 steps
# (vs 0.25 without PiSSA). Must keep dropout=0.
lora_r: 16
lora_alpha: 32
lora_dropout: 0
peft_init_lora_weights: pissa_niter_4
lora_target_modules:
  - q_proj
  - k_proj
  - v_proj
  - o_proj
  - gate_proj
  - up_proj
  - down_proj

datasets:
  - path: /workspace/data/strandset.jsonl
    type: chat_template
  - path: /workspace/data/introspector.jsonl
    type: chat_template
  - path: Axolotl-Partners/openthoughts-114k-linked
    type: chat_template
    data_files: train.jsonl

# Training
num_epochs: 3
learning_rate: 1e-4
warmup_steps: 100
micro_batch_size: 1
eval_batch_size: 1                     # Prevents OOM during eval
gradient_accumulation_steps: 16        # Effective batch = 16
gradient_checkpointing: true           # Essential for 27B on single H100
lr_scheduler: cosine
weight_decay: 0.01
max_grad_norm: 0.3
optim: adamw_8bit

# Eval
val_set_size: 0.0012                   # ~942 examples from 838K — fast eval (~2.5 min)
eval_steps: 200
save_steps: 200
save_total_limit: 5                    # Keep enough checkpoints to preserve best
early_stopping_patience: 25            # CRITICAL — see lesson below

# Optimizations (Axolotl auto-enables LoRA kernel optimizations too)
liger_kernel: true                     # Fused MLP + RMSNorm kernels
flash_attention: false                 # Uses SDPA (PyTorch native flash on H100)
cut_cross_entropy: true               # Faster loss computation

# Output
output_dir: /workspace/outputs
hub_model_id: YourOrg/your-adapter-name
hub_strategy: end

strict: false
```

### Key config decisions and why:

**`flash_attention: false` + no `sample_packing`**: `sample_packing: true` requires flash-attn for proper cross-sample attention masking. Without flash-attn, packed sequences leak attention between examples. SDPA alone doesn't support varlen packing (requires torch >= 2.11). So we disable sample_packing and lose the packing speedup, but keep Liger Kernel + LoRA kernel optimizations + Cut Cross Entropy.

**`early_stopping_patience: 25`**: The cosine LR schedule spans `num_epochs × steps_per_epoch` steps (e.g., 146,908 for 3 epochs). The LR stays at 99%+ of peak for the first ~5% of training. With patience=7, early stopping triggers at ~step 3,400 (2.3% of schedule) when the eval_loss plateaus due to high-LR oscillation — NOT because the model has converged. With patience=25, the model has 5,000 steps of runway and continues finding new bests: 0.225 → 0.205 → 0.198. The patience must be large enough that the LR has time to meaningfully decay.

**`gradient_checkpointing: true`**: A 27B model in fp16 uses ~54GB for weights. Without gradient checkpointing, activations for seq_len 6144 push memory past 93GB → OOM. With checkpointing, peak memory is ~73GB.

**`eval_batch_size: 1`**: The eval forward pass on 942 examples with seq_len up to 6144 can OOM if eval_batch_size > 1. Keep it at 1.

## Step 5: Launch Training

```bash
nohup bash -c 'export HF_HOME=/workspace/.cache/huggingface; \
  export PIP_CACHE_DIR=/workspace/.cache/pip; \
  export TMPDIR=/workspace/tmp; \
  export PYTORCH_CUDA_ALLOC_CONF=expandable_segments:True; \
  export HF_TOKEN=<token>; \
  axolotl train /workspace/axolotl_config.yml' \
  > /workspace/axolotl_training.log 2>&1 &
```

## Step 6: Monitor Training

```bash
# Check eval_loss progression
ssh root@<host> -p <port> 'grep "eval_loss" /workspace/axolotl_training.log'

# Check current step and speed
ssh root@<host> -p <port> 'tail -1 /workspace/axolotl_training.log | cat -v | tr "^M" "\n" | grep -v "^$" | tail -1'

# Check for errors
ssh root@<host> -p <port> 'grep -E "Error|Traceback|OOM|SIGSEGV" /workspace/axolotl_training.log | tail -5'
```

### What to expect:

1. **Model load**: ~2 min from cache, ~30 min if downloading
2. **Dataset tokenization**: ~10-15 min (Axolotl uses 128 processes)
3. **Initial baseline eval**: ~3-5 min (942 forward passes)
4. **PiSSA SVD initialization**: Silent, during model load, ~2-5 min for 27B
5. **Training**: 8-25 s/step (varies with sequence length), eval every 200 steps

### Expected loss trajectory with PiSSA:

| Step | eval_loss (typical) |
|------|---------------------|
| 0 (baseline) | ~1.4 |
| 200 | ~0.23 |
| 400 | ~0.22 |
| 1000 | ~0.215 |
| 2000 | ~0.205 |
| 3000 | ~0.204 |
| 3200 | ~0.198 |

The loss drops fast in the first 200 steps (PiSSA advantage), then improves slowly but steadily. eval_loss fluctuates ±0.03 between evals — this is noise from the 942-example eval set, not regression.

## Lessons Learned (Hard Won)

### Process Management on RunPod
- `pkill -f "python"` or `pkill -f "bash -c"` will kill your SSH session too. Use targeted `kill <PID>` after finding PIDs with `pgrep`.
- `$(pgrep ...)` inside SSH double-quoted commands gets expanded by the local shell. Use single quotes for the SSH command.
- `nohup bash -c '...' &` is the correct pattern for background processes that survive SSH disconnect.
- Environment variables set during pod creation are NOT inherited by SSH sessions. Always `export` them explicitly.

### Disk Space
- The 60GB container disk fills up fast. Axolotl's dependencies (~12GB) + Python packages + temp files = ~46GB used before any training starts.
- Dataset tokenization caches are large (GBs). They MUST go to the workspace volume via `HF_HOME` and `TMPDIR`.
- "No space left on device" manifests as `SIGSEGV: 11` (segfault) as a secondary effect — the actual error is buried in the log. Always check for "No space left" when you see SIGSEGV.

### Resume from Checkpoint
- `auto_resume_from_checkpoints: true` fails with `KeyError: 'EarlyStoppingCallback'` if you change `early_stopping_patience` between runs. The checkpoint's trainer_state.json doesn't match the new callback config.
- If you need to change early stopping config, start fresh (no resume). PiSSA converges fast enough that re-running from step 0 only costs ~30 min to get back to 0.23.
- `save_total_limit` must be high enough to preserve the best checkpoint. With `save_total_limit: 3`, the best checkpoint (step 1800) was rotated out by newer checkpoints (step 2200, 2400, 2600). Use `save_total_limit: 5` or higher.

### Attention Backends
- `flash-attn` is NOT pre-installed on RunPod's pytorch image. Compiling from source is slow (20-30 min) and may fail.
- PyTorch SDPA (`flash_attention: false` in Axolotl) uses native flash attention on H100 — nearly as fast for standard attention layers.
- `flash-linear-attention` (FLA) is a separate library for linear attention models (Qwen3.5+). It handles the linear attention layers; SDPA/Xformers handles standard attention layers.
- `sample_packing: true` requires flash-attn for cross-sample masking. Without it, Axolotl warns about missing decontamination. Do NOT use sample_packing without flash-attn.

### Early Stopping Patience
- patience=7 is too aggressive for a cosine LR schedule spanning 146K steps. The LR is still at 99% of peak when early stopping triggers at ~step 3,400.
- The eval_loss "plateau" at high LR is oscillation, not convergence. As the cosine LR decays, the model settles into deeper minima.
- patience=25 confirmed the model continues improving well past where patience=7 would have stopped: 0.225 (step 1800) → 0.205 (step 2000) → 0.198 (step 3200).
- Rule of thumb: set patience high enough that `patience × eval_steps` covers at least 3-5% of total steps, so the LR has time to start decaying.

### PiSSA vs Random Init
- PiSSA (`peft_init_lora_weights: pissa_niter_4`) gives dramatic convergence speedup: loss reaches 0.23 in 200 steps vs 0.25 without PiSSA.
- The SVD initialization is silent (no log output) and takes ~2-5 min for a 27B model.
- Must keep `lora_dropout: 0` — random dropout would discard the principal components that PiSSA initialized.
- PiSSA is a free lunch: zero training-time cost, just a few minutes of initialization.

### Cost Management
- H100 NVL at $3.19/hr. Training at ~10 s/step with 200-step eval intervals.
- Each eval cycle (200 steps + 2.5 min eval) ≈ 35 min ≈ $1.86.
- With patience=25, minimum training is ~5,000 steps ≈ 14 hours ≈ $45.
- The adapter at eval_loss 0.198 (step 3200) cost ~$35 total (including setup, failed attempts, and training).
- Failed attempts (OOM, disk space, checkpoint resume) added ~$20 of wasted compute. Getting the config right the first time would save this.

## v2: Improving Analysis Task Performance

The v1 adapter (`qwen36-rust-reasoning-all-lora`, eval_loss 0.1872) produces concise Rust code quickly but is **overconfident on analysis tasks** — it says "no bug found" when bugs exist, and gives terse reviews without examining the code.

### Root Cause

Three converging causes:

1. **Direct-answer targets**: The v1 preprocessing formatted `bug_detection` as `**Bug:** {description}\n**Fixed code:** {code}` — no reasoning trace. The model learns P(conclusion | code) directly and emits the most likely conclusion token immediately.
2. **Shallow analysis data**: The 533K introspector examples are pattern extraction ("Element type: X, Name: Y"), not analytical reasoning. They teach terseness.
3. **Capacity ceiling**: r=16 (79.7M params, 0.29%) is enough for codegen pattern-matching but thin for multi-step reasoning.

The base model CAN reason (3,400–5,400 char responses with thinking traces), but the adapter's training format teaches it to bypass reasoning.

### Fix: Two Levers

| Lever | Change | Why |
|-------|--------|-----|
| **Format** | Add chain-of-thought to analysis targets (bug_detection, code_review, refactoring, optimization) | Forces deliberation before conclusion at inference |
| **Capacity** | r=16 → r=32 (159.4M params, 0.59%) | More headroom for multi-step reasoning |

### v2 Training Workflow (on RunPod)

All v2 scripts (distillation, preprocessing, config, eval) live in the HuggingFace repo (`Axolotl-Partners/rust-adapter-scripts`), not in hKask — hKask is a Rust project and Python is not an acceptable dependency.

1. **Distill reasoning traces** — Use the base Qwen3.6-27B (4-bit, via Unsloth) to generate real chain-of-thought reasoning traces for the analysis subset (bug_detection, code_review, code_refactoring, code_optimization). The base model reasons well (3,400-5,400 char responses); this captures its actual analysis of each specific code example — not a generic checklist. ~4h on H100. Output: `/workspace/data/distilled/{category}/{idx}.txt`.
2. **Preprocess v2 data** — Reformat analysis targets with the distilled CoT traces prepended to the conclusion. Use `--use-distilled /workspace/data/distilled/`. Two system prompts: concise for codegen, thorough for analysis. Without distilled traces, falls back to a structured scaffolding checklist (less effective — it's generic boilerplate, not real reasoning).
3. **Train with v2 config** — Same optimizations as v1 (PiSSA `pissa_niter_4`, patience=25, SDPA, Liger, CCE) but r=32, alpha=64. PiSSA at r=32 is untested — if convergence is slow, increase SVD iterations to `pissa_niter_8`. Expect ~26h, ~$83.
4. **Merge and save** — After training completes, merge the adapter into the base model and save the merged model. This is MANDATORY for PiSSA adapters — see "PiSSA Inference" below.
5. **Eval** — Load the merged model (not the adapter) and run the 225-example Strandset test split. Use the pre-flight checks from the `adapter-eval` skill before running.

### PiSSA Inference (Critical Lesson)

PiSSA (`pissa_niter_4`) decomposes the base model weights W = W_residual + A_init × B_init at **load time** via SVD. The decomposition is deterministic for a given set of weights, but **different library versions (transformers, torch) load the model weights slightly differently**. This means:

- **Training pod** (transformers 5.9.0 via Axolotl): SVD(W_training) → principal_training
- **Eval pod** (transformers 5.5.0 via Unsloth): SVD(W_eval) → principal_eval ≠ principal_training
- **Result**: The residual base is wrong, so (W - wrong_principal) + saved_adapter = garbage

This was confirmed empirically: the PiSSA residual base alone (adapter disabled) produces multilingual garbage tokens. The adapter weights are correct, but they're paired with the wrong residual base.

**The fix**: Always save the merged model at the end of training, on the same pod with the same library versions. The merged model = W_residual + A_final × B_final, computed consistently. At inference, load the merged model directly — no PiSSA decomposition, no version mismatch.

**SVD conversion to standard LoRA also fails**: Converting PiSSA to standard LoRA by computing delta = W_merged - W_original and SVD-decomposing to rank-r has ~40-50% relative error per layer. This compounds across 256 layers and produces garbage output.

**Rule**: PiSSA adapters are NOT portable across library versions. Always save the merged model. The merged model is the deployment artifact; the adapter is for reference only.

### Immediate Alternative (No Retrain)

Route analysis tasks to the base model (no adapter) and use the adapter only for code generation. The base model handles bug detection correctly but takes 77–112s per example — acceptable for analysis, not for codegen. This costs nothing and can be deployed while v2 training is planned.

### System Prompt Split

v2 uses two system prompts:
- **Codegen** (concise): "You are a Rust programming expert. Provide idiomatic, correct, and well-structured Rust code."
- **Analysis** (thorough): "You are a Rust code analysis expert. Before stating your conclusion, carefully trace through the code and check for common Rust pitfalls: arithmetic safety, ownership and borrowing, edge cases, and type correctness. Only conclude after you have examined the code."

The analysis prompt explicitly delays the conclusion, which counteracts the overconfidence pattern.