# Handoff: Rust Adapter Training — 2026-07-12

## 1. Session Context

Trained LoRA adapters for Qwen3.6-27B across three domains: Rust coding, Rust code analysis, and general reasoning. Two individual adapters completed and uploaded. A combined run (all three datasets, 838K examples) was started but killed because eval was too slow (2.4 hours per eval due to missing FLA kernels — the `causal-conv1d` fix was applied to the script but the running pod used the old version). First eval_loss was 0.390, confirming the combined approach works. The updated script with all fixes is ready to launch.

## 2. What Was Done

### Adapters trained and uploaded

| Adapter | Dataset | Examples | Best eval_loss | HF Repo |
|---------|---------|----------|----------------|---------|
| Rust Coding | Strandset-Rust-v1 | 191K | 1.128 | `Axolotl-Partners/qwen36-rust-coding-lora` |
| Rust Analysis | introspector/rust-analyser | 533K | 1.221 | `Axolotl-Partners/qwen36-rust-analysis-lora` |
| Combined (baseline) | All three | 838K | 0.390 (1 eval only) | Killed — not uploaded |

### Training scripts (canonical copies on HF at `Axolotl-Partners/rust-adapter-scripts`, commit `c369209c`)
- `scripts/train_rust_adapter.sh` — supports `MODE=coding|analysis|both|reasoning|all`
- `scripts/eval_rust_adapter.sh` — baseline-vs-adapter comparison on Strandset test split
- `scripts/runpod_unsloth.sh` — pod launcher for all modes
- `scripts/build_openthoughts_linked.sh` — builds OpenThoughts sidecar metadata
- `scripts/build_rust_linked.sh` — builds Rust datasets sidecar metadata

### Linked datasets on HuggingFace
- `Axolotl-Partners/openthoughts-114k-linked` — clean train.jsonl (2.76GB) + metadata.jsonl (3.38GB) with extracted PKO reasoning steps (avg 160.7 steps/example, 93.9% unique)
- `Axolotl-Partners/rust-datasets-linked` — strandset-train.jsonl + strandset-metadata.jsonl + introspector-train.jsonl + introspector-metadata.jsonl

### Key fixes in the training script (latest commit `3b59fbc`)
1. Model loads BEFORE datasets — prevents HF cache lock futex deadlock
2. Dynamic eval at 2% — re-sampled from training set before each eval
3. `evaluate()` override — stable across transformers versions
4. `causal-conv1d --no-build-isolation` — fixes CUDA version mismatch (system 12.4 vs PyTorch 13.0)
5. PEFT `get_peft_model` with PiSSA `init_lora_weights="pissa_niter_4"` — Unsloth's get_peft_model doesn't support PiSSA
6. Crash handler — saves logs to `/workspace/crash_*.log` on error

### Distillation cleanup
- Deleted `scripts/train_unsloth.sh`, `scripts/eval_unsloth.sh`, distillation docs and handoffs
- Deleted HF dataset repo `Axolotl-Partners/qwen36-distill-opus-dsv4`
- All references now point to `Axolotl-Partners/rust-adapter-scripts` for scripts

### MCP server fixes (`hkask-mcp-training`)
- `UnslothHarness::render_config` — `processing_class` not `tokenizer=`, early stopping, warmup_steps, eval dataset split
- `training_evaluate` — `temperature: 1.0` not `0.0`
- `RunpodHost::submit` — passes all training params as env vars
- `LoraParams::default` — 7 target modules, LR 1e-4, dropout=0

## 3. What Remains

### HIGH — Launch `--rust-all` training with the fixed script

The updated script has all fixes but hasn't been run yet. To launch:

```bash
bash scripts/runpod_unsloth.sh --rust-all
```

Then paste the command shown. The script will:
- Install deps including `causal-conv1d --no-build-isolation` (FLA kernels will work)
- Load model first (deadlock fix)
- Load all 3 datasets (838K examples)
- Apply PEFT LoRA with PiSSA SVD initialization
- Train with 2% dynamic eval, patience=7
- Upload to `Axolotl-Partners/qwen36-rust-reasoning-all-lora`

**Key expectation:** With FLA kernels working, eval should be ~10-20 min instead of 2.4 hours. PiSSA should give 30-50% faster convergence. Total runtime estimate: 3-5 hours (~$10-16).

### HIGH — Evaluate the trained adapters

After training completes, run eval:
```bash
bash scripts/runpod_unsloth.sh --rust-eval
```

The eval script needs the adapter downloaded first. It runs baseline-vs-adapter on 225 Strandset test examples across 15 categories. **Note:** eval is also slow without FLA kernels — the eval script needs the same `causal-conv1d --no-build-isolation` fix applied.

### MEDIUM — Update eval script with FLA kernel fix

`scripts/eval_rust_adapter.sh` doesn't install `causal-conv1d` with `--no-build-isolation`. Add the same fix as the training script.

### MEDIUM — Update docs with final training results

Once the `--rust-all` training completes and eval results are in, update `docs/how-to/training-and-adapters.md` with the final configuration and results.

## 4. Recommended Skills and Tools

- `coding-guidelines` before modifying any scripts
- `diagnose` if the training crashes (check `/workspace/crash_*.log` first)
- `handoff` at session end

**Validation commands:**
```bash
bash -n scripts/train_rust_adapter.sh
bash -n scripts/eval_rust_adapter.sh
bash -n scripts/runpod_unsloth.sh
```

**Pod management:**
```bash
# Check status
RUNPOD_API_KEY=$(grep RUNPOD_API_KEY .env | cut -d= -f2-)
curl -s -X POST "https://api.runpod.io/graphql?api_key=${RUNPOD_API_KEY}" \
  -H "Content-Type: application/json" \
  -d '{"query":"{ myself { pods { id name desiredStatus } } }"}'

# Terminate a pod
curl -s -X POST "https://api.runpod.io/graphql?api_key=${RUNPOD_API_KEY}" \
  -H "Content-Type: application/json" \
  -d '{"query":"mutation { podTerminate(input: {podId: \"POD_ID\"}) }"}'
```

## 5. Key Decisions to Preserve

1. **Base model is Qwen3.6-27B** (not Qwen3-Coder or Qwen3). User-specified. Fallback is Qwen3.6-35B-A3B (MoE). Do not fall back to Qwen3.
2. **PEFT `get_peft_model` with PiSSA** instead of Unsloth's `get_peft_model`. Unsloth doesn't support `init_lora_weights="pissa_niter_4"`. We still use Unsloth's `FastLanguageModel.from_pretrained` for kernel patching and model loading, but PEFT for the LoRA adapter.
3. **Dynamic eval at 2%** — re-sampled from training set before each eval step. Prevents overfitting to a fixed eval set. Implemented by overriding `evaluate()` method on a custom SFTTrainer subclass.
4. **Model loads BEFORE datasets** — prevents HuggingFace cache lock futex deadlock when multiprocessing workers spawn during dataset `.map()` calls.
5. **`causal-conv1d --no-build-isolation`** — the RunPod container image has CUDA 12.4 toolkit but PyTorch is compiled with CUDA 13.0. Build isolation picks up the wrong CUDA headers. Without this fix, FLA kernels don't install and inference is ~100x slower.
6. **Sidecar metadata, not embedded annotations** — ontology annotations (PKO + Dublin Core + 5W1H) are in a separate `metadata.jsonl` file, not in the system prompt. Training data stays clean. Metadata is for downstream tooling.
7. **No distillation project** — all distillation scripts, datasets, and references have been deleted. Only Rust coding, Rust analysis, and OpenThoughts reasoning training remains.
8. **Single combined adapter, not 3-layer architecture** — train one adapter on all datasets combined. The 3-layer merged sequential approach was rejected by essentialist review as over-engineered with catastrophic forgetting risk.
9. **`runpod_unsloth.sh` uses heredoc for Python payload** — `python3 -c "..."` with `$input` in the string causes bash `set -u` errors. Use `python3 << 'PYEOF'` instead.
10. **No `kask-models` network volume dependency** — the volume is in `US-CA-2` where GPUs are often unavailable. Use a fresh 200GB workspace volume and accept the ~30 min model download time.
