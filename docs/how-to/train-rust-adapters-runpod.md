# How to Train Rust Adapters on RunPod with Unsloth

## Overview

Train LoRA adapters for Qwen3.6-27B specialized for Rust programming:

| Mode | Dataset | Size | Focus | HF Repo |
|------|---------|------|-------|---------|
| `--rust-coding` | `Fortytwo-Network/Strandset-Rust-v1` | 191K | Code generation, bug detection, review, refactoring, docs | `qwen36-rust-coding-lora` |
| `--rust-analysis` | `introspector/rust-analyser` | 533K | Symbol resolution, type inference, semantic analysis | `qwen36-rust-analysis-lora` |
| `--rust-both` | Combined | 724K | All of the above | `qwen36-rust-combined-lora` |

## 1. Launch a Pod

```bash
bash scripts/runpod_unsloth.sh --rust-coding
```

This launches an H100 NVL pod (falls back to A100 80GB) and prints the SSH command + dashboard URL.

## 2. Start Training

Paste the ONE command shown in the output. For example:

```bash
MODE=coding curl -sL https://huggingface.co/datasets/Axolotl-Partners/qwen36-distill-opus-dsv4/raw/1394b763400304f2cfe70c50d16a34a916c6c580/train_rust_adapter.sh | bash
```

The command returns immediately — all output goes to `/workspace/training.log`.

## 3. Monitor

```bash
ssh root@<SSH_HOST> -p <SSH_PORT> 'tail -f /workspace/training.log'
```

## 3b. Evaluate the Adapter

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


## 4. Training Configuration

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

## 5. Datasets

### Strandset-Rust-v1 (Apache-2.0)

191K verified Rust examples across 15 task categories:

- `code_generation` (17K) — Generate functions from specs
- `docstring_generation` (17K) — Produce API documentation
- `code_explanation` (17K) — Explain what code does
- `comment_generation` (16K) — Add inline comments
- `code_summarization` (16K) — Summarize function purpose
- `function_naming` (16K) — Suggest idiomatic names
- `variable_naming` (16K) — Generate semantic names
- `code_review` (15K) — Critique and improve
- `code_completion` (15K) — Fill missing sections
- `code_refactoring` (14K) — Improve readability
- `bug_detection` (13K) — Identify and fix bugs
- `code_optimization` (13K) — Optimize algorithms
- `code_search` (4K) — Return relevant code
- `test_generation` (3K) — Generate unit tests
- `api_usage_prediction` (490) — Predict next API call

94.3% compilation success verified with `rustc`. Peer-reviewed via Fortytwo's Swarm Inference.

### introspector/rust-analyser (AGPL-3.0)

533K semantic analysis traces from rust-analyzer analyzing its own codebase:

- `name_resolution` — Symbol binding, scope analysis, import resolution
- `type_inference` — Type checking, inference decisions
- `parsing` — Syntax tree generation, tokenization

**Note:** AGPL-3.0 license. Adapters trained on this data may require AGPL-compatible distribution terms.

## 6. Output

On success:
- LoRA adapter weights uploaded to the corresponding HF model repo
- Training log preserved in `/workspace/training.log`
- Pod auto-terminates (60s grace period)

On failure:
- Error logged with exit code
- Pod stays alive for debugging
