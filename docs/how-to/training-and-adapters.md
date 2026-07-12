---
title: "Training and Adapters"
audience: [operators, developers, ml-engineers]
last_updated: 2026-07-12
version: "0.31.0"
status: "Active"
domain: "Training"
mds_categories: [domain, lifecycle]
---

# Training and Adapters

Fine-tune LoRA adapters for Qwen3.6-27B on RunPod with Unsloth, evaluate them, and manage the adapter lifecycle through the `kask adapter` CLI commands. hKask provides standalone RunPod/Unsloth training scripts that are verified on H100 NVL and A100 80GB GPUs.

---

## Training Overview

hKask's training path uses standalone shell scripts that launch RunPod pods, execute Unsloth-based fine-tuning, and auto-upload LoRA adapters to HuggingFace. The MCP submission path (`hkask-mcp-training`) provides job submission, status tracking, and adapter lifecycle management, but the end-to-end contract for dataset transfer, training execution, artifact recovery, and adapter registration has not been verified through an automated integration test.

### Working Training Scripts

| Script | Purpose | Status |
|--------|---------|--------|
| `scripts/train_unsloth.sh` | Qwen3.6-27B reasoning distillation | Verified |
| `scripts/train_rust_adapter.sh` | Rust coding + analysis adapters | Available |
| `scripts/eval_unsloth.sh` | Adapter evaluation with baseline comparison | Verified |
| `scripts/eval_rust_adapter.sh` | Rust adapter evaluation | Available |
| `scripts/runpod_unsloth.sh` | Pod launcher (all modes) | Verified |

### Current Limitations

The generic CLI commands `kask docproc ingest`, `kask training create-dataset`, `kask training start`, and `kask training status` are **not implemented CLI commands**. Do not use them. Training is driven by the standalone scripts and the `kask adapter` lifecycle commands described below.

For the verified state of the replica, corpus, and RunPod/Unsloth paths, see `docs/status/replica-corpus-training-readiness.md`.

---

## Train Qwen3.6-27B on RunPod with Unsloth

Fine-tune a BF16 LoRA adapter for Qwen3.6-27B on distillation data using a single A100 or H100 GPU on RunPod, then auto-upload the adapter to HuggingFace.

### Prerequisites

- RunPod API key in `.env`
- HuggingFace write token in `.env`
- SSH public key added to RunPod account settings

### Step 1: Launch the Pod

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

### Step 2: Start Training

Paste one command in the web terminal:

```bash
curl -sL https://huggingface.co/datasets/Axolotl-Partners/qwen36-distill-opus-dsv4/raw/bfedff55f47bcf0286ff49584635e25912147c97/train_unsloth.sh | bash
```

The command returns immediately — all output is redirected to `/workspace/training.log`. Monitor progress:

```bash
tail -f /workspace/training.log
```

### Step 3: Training Stages

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

### Step 4: Training Configuration

The default hyperparameters are aligned with Unsloth's SFT recommendations and published Qwen3.6-27B training examples:

| Parameter | Value | Rationale |
|-----------|-------|-----------|
| Model | `unsloth/Qwen3.6-27B` | Unsloth-optimized BF16 checkpoint |
| Method | BF16 LoRA (not QLoRA) | QLoRA not recommended for Qwen3.5/3.6 |
| LoRA Rank | 16 | Lower rank reduces overfit risk on small datasets |
| LoRA Alpha | 32 | α=2r (standard scaling ratio) |
| LoRA Dropout | 0 | Required for Unsloth kernel fusion |
| Learning Rate | 1e-4 | Conservative for small datasets; prevents overfit |
| Max Seq Length | 6144 | Auto-adjusted down if P95 tokens < 3072 |
| Epochs | 3 | With early stopping (patience=10 evals) |
| Batch Size | 1 × 4 accumulation = 4 | Fits 80GB VRAM |
| Warmup | 50 steps | Fixed steps — avoids wasting training on excessive warmup |
| Eval Steps | 50 | Frequent eval to catch best checkpoint early |
| Data Ratio | 75% reasoning / 25% chat | Preserves thinking capabilities |

### Step 5: Output

On success:
- LoRA adapter weights uploaded to `Axolotl-Partners/qwen36-distill-opus-dsv4-lora`
- Training log preserved in `/workspace/training.log`
- Pod auto-terminates (60s grace period, Ctrl-C to cancel)

On failure:
- Error message logged with exit code
- Pod kept alive for debugging
- `/workspace/outputs` may contain partial checkpoints

### Step 6: Post-Training

After training completes, evaluate the adapter:

1. Download from HF: `huggingface-cli download Axolotl-Partners/qwen36-distill-opus-dsv4-lora`
2. Merge LoRA: `model.merge_and_unload()` in Unsloth
3. Run benchmarks: GPQA Diamond, MATH-500, MMLU-Pro
4. Manual review: inspect 20-30 thinking traces for quality

### Cost

| GPU | Spot Rate | Est. Time | Est. Cost |
|-----|-----------|-----------|-----------|
| A100 PCIe 80GB | ~$1.19/hr | 30-40h | ~$35-48 |
| A100 SXM 80GB | ~$1.39/hr | 25-35h | ~$35-49 |
| H100 NVL 80GB | ~$2.59/hr | 10-15h | ~$26-39 |

Costs are approximate. Spot prices fluctuate. The pod auto-terminates on completion, so you never pay for idle time.

---

## Train Rust Adapters on RunPod with Unsloth

Train LoRA adapters for Qwen3.6-27B specialized for Rust programming:

| Mode | Dataset | Size | Focus | HF Repo |
|------|---------|------|-------|---------|
| `--rust-coding` | `Fortytwo-Network/Strandset-Rust-v1` | 191K | Code generation, bug detection, review, refactoring, docs | `qwen36-rust-coding-lora` |
| `--rust-analysis` | `introspector/rust-analyser` | 533K | Symbol resolution, type inference, semantic analysis | `qwen36-rust-analysis-lora` |
| `--rust-both` | Combined | 724K | All of the above | `qwen36-rust-combined-lora` |

### Step 1: Launch a Pod

```bash
bash scripts/runpod_unsloth.sh --rust-coding
```

This launches an H100 NVL pod (falls back to A100 80GB) and prints the SSH command + dashboard URL.

### Step 2: Start Training

Paste the one command shown in the output. For example:

```bash
MODE=coding curl -sL https://huggingface.co/datasets/Axolotl-Partners/qwen36-distill-opus-dsv4/raw/1394b763400304f2cfe70c50d16a34a916c6c580/train_rust_adapter.sh | bash
```

The command returns immediately — all output goes to `/workspace/training.log`.

### Step 3: Monitor

```bash
ssh root@<SSH_HOST> -p <SSH_PORT> 'tail -f /workspace/training.log'
```

### Step 4: Evaluate the Adapter

After training completes and the adapter is uploaded to HF, launch an eval pod:

```bash
bash scripts/runpod_unsloth.sh --rust-eval
```

Then paste the eval command:

```bash
curl -sL https://huggingface.co/datasets/Axolotl-Partners/qwen36-distill-opus-dsv4/raw/eae9bcdd2605a0b80e81af728d89278b0c368ce9/eval_rust_adapter.sh | bash
```

The eval script:
- Loads the Strandset-Rust-v1 test split (225 examples, 15 per category)
- Runs baseline (no adapter) and adapter inference on all 225 examples
- Scores per-category using category-appropriate metrics:
  - **Naming tasks** (function/variable): exact match
  - **Text tasks** (summary/explanation/docstring): word overlap ≥30%
  - **Code tasks** (generation/bug/review/refactor/optimization/completion/search/test/comment): token overlap ≥50%
- Prints per-category accuracy and overall delta (adapter - baseline)
- Saves results to `/workspace/eval_results/rust_eval_*.json`

Monitor:

```bash
ssh root@<SSH_HOST> -p <SSH_PORT> 'tail -f /workspace/rust_eval.log'
```

### Training Configuration

| Parameter | Value | Rationale |
|-----------|-------|-----------|
| Model | `unsloth/Qwen3.6-27B` | Unsloth-optimized BF16 checkpoint |
| LoRA Rank | 16 | Lower rank reduces overfit risk |
| LoRA Alpha | 32 | α=2r (standard scaling ratio) |
| LoRA Dropout | 0 | Unsloth kernel optimization requires 0 |
| Learning Rate | 1e-4 | Conservative for large datasets |
| Max Seq Length | 6144 | Accommodates code + context |
| Epochs | 3 | With early stopping (patience=10) |
| Warmup | 50 steps | Fixed steps — avoids excessive warmup |
| Eval Steps | 50 | Frequent eval to catch best checkpoint |
| Batch Size | 1 × 4 accumulation = 4 | Fits 80GB VRAM |

### Datasets

**Strandset-Rust-v1 (Apache-2.0):** 191K verified Rust examples across 15 task categories:

| Category | Count | Description |
|----------|-------|-------------|
| `code_generation` | 17K | Generate functions from specs |
| `docstring_generation` | 17K | Produce API documentation |
| `code_explanation` | 17K | Explain what code does |
| `comment_generation` | 16K | Add inline comments |
| `code_summarization` | 16K | Summarize function purpose |
| `function_naming` | 16K | Suggest idiomatic names |
| `variable_naming` | 16K | Generate semantic names |
| `code_review` | 15K | Critique and improve |
| `code_completion` | 15K | Fill missing sections |
| `code_refactoring` | 14K | Improve readability |
| `bug_detection` | 13K | Identify and fix bugs |
| `code_optimization` | 13K | Optimize algorithms |
| `code_search` | 4K | Return relevant code |
| `test_generation` | 3K | Generate unit tests |
| `api_usage_prediction` | 490 | Predict next API call |

94.3% compilation success verified with `rustc`. Peer-reviewed via Fortytwo's Swarm Inference.

**introspector/rust-analyser (AGPL-3.0):** 533K semantic analysis traces from rust-analyzer analyzing its own codebase:

- `name_resolution` — Symbol binding, scope analysis, import resolution
- `type_inference` — Type checking, inference decisions
- `parsing` — Syntax tree generation, tokenization

Adapters trained on this data may require AGPL-compatible distribution terms.

### Output

On success:
- LoRA adapter weights uploaded to the corresponding HF model repo
- Training log preserved in `/workspace/training.log`
- Pod auto-terminates (60s grace period)

On failure:
- Error logged with exit code
- Pod stays alive for debugging

---

## Adapter Lifecycle via CLI

The `kask adapter` commands manage trained adapter deployment to cloud inference providers. These commands delegate to the training MCP server.

### List Trained Adapters

```bash
kask adapter list
kask adapter list --skill <skill-name>
```

### Deploy an Adapter

Deploy an adapter to a cloud inference provider:

```bash
kask adapter deploy <adapter-name> --provider together
```

The `--provider` flag accepts `together` (default) or `runpod`.

### Check Deployment Status

```bash
kask adapter status <deployment_id>
```

Use the deployment ID returned by the `deploy` command.

### Tear Down a Deployed Endpoint

```bash
kask adapter teardown <deployment_id>
```

This removes the deployed inference endpoint and releases associated resources.

---

## References

- [Unsloth LoRA fine-tuning Hyperparameters Guide](https://unsloth.ai/docs/get-started/fine-tuning-llms-guide/lora-hyperparameters-guide) — Default SFT parameters
- [Unsloth Qwen3.5 Fine-tuning Guide](https://unsloth.ai/docs/models/qwen3.5/fine-tune) — QLoRA not recommended for Qwen3.5/3.6
- [QwenLM Qwen3 Training with Unsloth](https://github.com/QwenLM/Qwen3/blob/main/docs/source/training/unsloth.md) — 75% reasoning / 25% non-reasoning dataset ratio
- [Qwen3.6 Training Reference](../reference/qwen36-training-hyperparameters.md) — Full hyperparameter rationale and literature survey
- [Replica, Corpus, and Training Readiness](../status/replica-corpus-training-readiness.md) — Verified state of training paths