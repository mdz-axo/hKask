#!/bin/bash
# ============================================================================
# Rust Adapter Training — Coding + Analysis — runs on RunPod pod
# ============================================================================
# Image: runpod/pytorch:2.4.0-py3.11-cuda12.4.1-devel-ubuntu22.04
# Usage: curl -sL <this-script-url> | bash
#
# MODE env var selects the dataset:
#   MODE=coding   → Strandset-Rust-v1 (191K examples, 15 task categories)
#   MODE=analysis → introspector/rust-analyser (533K semantic analysis traces)
#   MODE=both     → Combined dataset (coding + analysis)
# Default: coding
# ============================================================================
set -euo pipefail
exec > /workspace/training.log 2>&1

MODE="${MODE:-coding}"

if [ -r /proc/1/environ ]; then
    while IFS= read -r -d '' entry; do
        case "$entry" in
            HF_TOKEN=*|RUNPOD_API_KEY=*) export "${entry?}" ;;
        esac
    done < /proc/1/environ
fi

echo "============================================"
echo "Rust Adapter Training | $(date -u) | MODE=${MODE}"
echo "GPU: $(nvidia-smi --query-gpu=name --format=csv,noheader 2>/dev/null || echo 'unknown')"
echo "============================================"

export DEBIAN_FRONTEND=noninteractive
export PATH="$HOME/.local/bin:$PATH"
export HF_HOME=/workspace/.cache/huggingface
export PIP_CACHE_DIR=/workspace/.cache/pip
export PYTORCH_CUDA_ALLOC_CONF=expandable_segments:True
export HF_DEACTIVATE_ASYNC_LOAD=1
export HF_XET_HIGH_PERFORMANCE=1
mkdir -p "$HF_HOME" "$PIP_CACHE_DIR"

echo "" && echo "=== SYSTEM ==="
apt-get update -qq && apt-get install -y -qq git build-essential >/dev/null 2>&1
pip install --cache-dir "$PIP_CACHE_DIR" -q --upgrade pip setuptools wheel

echo "datasets>=3.4.1,<5.0" > /tmp/pip-constraints.txt
export PIP_CONSTRAINT=/tmp/pip-constraints.txt

echo "=== INSTALLING UNSLOTH ==="
pip install --cache-dir "$PIP_CACHE_DIR" -q unsloth unsloth_zoo

echo "=== INSTALLING CORE ==="
pip install --cache-dir "$PIP_CACHE_DIR" -q \
    "transformers>=5.2" \
    datasets trl peft accelerate requests huggingface_hub

echo "=== INSTALLING QWEN KERNELS ==="
pip install --cache-dir "$PIP_CACHE_DIR" -q \
    "flash-linear-attention[cuda]" tilelang

unset PIP_CONSTRAINT

python3 -c "
import torch
import fla, tilelang
print(f'Torch: {torch.__version__} | CUDA: {torch.version.cuda}')
print(f'BF16: {torch.cuda.is_bf16_supported()}')
"
echo "Install complete."
cd /workspace

cat > train_rust.py << 'PYEOF'
import os, sys, json, time, random
from pathlib import Path
import torch
from datasets import load_dataset, concatenate_datasets
from unsloth import FastLanguageModel
from trl import SFTTrainer, SFTConfig
from transformers import EarlyStoppingCallback

MODEL_ID = "unsloth/Qwen3.6-27B"
MAX_SEQ_LENGTH = 6144
OUTPUT_DIR = os.environ.get("OUTPUT_DIR", "/workspace/outputs")
HF_TOKEN = os.environ.get("HF_TOKEN", "")
HF_REPO = os.environ.get("HF_REPO", "Axolotl-Partners/qwen36-rust-coding-lora")
SEED = 3407
LORA_R = 16
LORA_ALPHA = 32
LORA_DROPOUT = 0
TARGET_MODULES = ["q_proj","k_proj","v_proj","o_proj","gate_proj","up_proj","down_proj"]
LEARNING_RATE = 1e-4
NUM_EPOCHS = 3
PER_DEVICE_BATCH = 1
GRAD_ACCUM = 4
WARMUP_STEPS = 50
EVAL_STEPS = 50
SAVE_STEPS = 100
MAX_GRAD_NORM = 0.3
WEIGHT_DECAY = 0.01
MAX_SAMPLES = int(os.environ.get("MAX_SAMPLES", "0"))  # 0 = all
MODE = os.environ.get("MODE", "coding")
# PiSSA: SVD-based LoRA initialization from principal singular values.
# 30-50% faster convergence, 3-5% better accuracy vs vanilla LoRA. Zero cost (~2s init).
# Must keep dropout=0 — random dropout would discard principal components.
INIT_LORA_WEIGHTS = os.environ.get("INIT_LORA_WEIGHTS", "pissa_niter_4")

random.seed(SEED)
Path(OUTPUT_DIR).mkdir(parents=True, exist_ok=True)

print("=" * 60)
print(f"Rust Adapter Training | {time.strftime('%Y-%m-%d %H:%M:%S')} | MODE={MODE}")
print(f"GPU: {torch.cuda.get_device_name(0)}")
print(f"LoRA: r={LORA_R}, alpha={LORA_ALPHA}, dropout={LORA_DROPOUT}")
print(f"LR: {LEARNING_RATE}, epochs={NUM_EPOCHS}, warmup={WARMUP_STEPS}")
print(f"Eval: every {EVAL_STEPS} steps, patience=7")
print("=" * 60)

# ── Dataset formatting ─────────────────────────────────────────────────────

SYSTEM_CODING = "You are a Rust programming expert. Provide idiomatic, correct, and well-structured Rust code."
SYSTEM_ANALYSIS = "You are a Rust code analysis expert. Analyze code for symbols, types, and semantic structure as rust-analyzer would."

def parse_json_field(val):
    """Parse a field that may be a JSON string, Python dict repr, or already a dict."""
    if val is None:
        return {}
    if isinstance(val, dict):
        return val
    if isinstance(val, str):
        try:
            return json.loads(val)
        except (json.JSONDecodeError, TypeError):
            pass
        # Strandset uses Python dict repr (single quotes) for input_data
        try:
            import ast
            return ast.literal_eval(val)
        except (ValueError, SyntaxError):
            return {"_raw": val}
    return {}

def format_strandset(example):
    """Convert a Strandset-Rust-v1 record to ChatML messages."""
    cat = example.get("task_category", "")
    inp = parse_json_field(example.get("input_data"))
    out = parse_json_field(example.get("output_data"))

    if cat == "code_generation":
        title = inp.get("title", "")
        desc = inp.get("description", "")
        ctx = inp.get("code_context", "")
        user = f"Generate Rust code for the following task.\n\nTitle: {title}\nDescription: {desc}"
        if ctx:
            user += f"\n\nContext:\n```rust\n{ctx}\n```"
        assistant = f"```rust\n{out.get('code', '')}\n```"

    elif cat == "bug_detection":
        buggy = inp.get("buggy_code", "")
        ctx = inp.get("code_context", "")
        user = f"Find and fix the bug in this Rust code.\n\n"
        if ctx:
            user += f"Context:\n```rust\n{ctx}\n```\n\n"
        user += f"Buggy code:\n```rust\n{buggy}\n```"
        desc = out.get("bug_description", "")
        fixed = out.get("fixed_code", "")
        assistant = f"**Bug:** {desc}\n\n**Fixed code:**\n```rust\n{fixed}\n```"

    elif cat == "code_review":
        before = inp.get("code_before", "")
        ctx = inp.get("code_context", "")
        user = f"Review this Rust code and suggest improvements.\n\n"
        if ctx:
            user += f"Context:\n```rust\n{ctx}\n```\n\n"
        user += f"Code:\n```rust\n{before}\n```"
        comment = out.get("review_comment", "")
        after = out.get("code_after", "")
        assistant = f"**Review:** {comment}\n\n**Improved code:**\n```rust\n{after}\n```"

    elif cat == "docstring_generation":
        code = inp.get("code", "")
        user = f"Generate a Rust docstring for this code.\n\n```rust\n{code}\n```"
        assistant = out.get("docstring", "")

    elif cat == "comment_generation":
        code = inp.get("code", "")
        user = f"Add meaningful inline comments to this Rust code.\n\n```rust\n{code}\n```"
        assistant = f"```rust\n{out.get('commented_code', '')}\n```"

    elif cat == "code_summarization":
        code = inp.get("code", "")
        user = f"Summarize what this Rust code does.\n\n```rust\n{code}\n```"
        assistant = out.get("summary", "")

    elif cat == "code_explanation":
        code = inp.get("code", "")
        user = f"Explain this Rust code.\n\n```rust\n{code}\n```"
        assistant = out.get("explanation", "")

    elif cat == "function_naming":
        code = inp.get("code", "")
        user = f"Suggest an idiomatic Rust function name for the placeholder in this code.\n\n```rust\n{code}\n```"
        assistant = out.get("function_name", "")

    elif cat == "variable_naming":
        code = inp.get("code", "")
        user = f"Suggest an idiomatic Rust variable name for the placeholder in this code.\n\n```rust\n{code}\n```"
        assistant = out.get("variable_name", "")

    elif cat == "code_completion":
        prefix = inp.get("prefix", "")
        suffix = inp.get("suffix", "")
        user = f"Complete this Rust code. Fill in the missing section between the prefix and suffix.\n\n"
        user += f"Prefix:\n```rust\n{prefix}\n```\n\n"
        user += f"Suffix:\n```rust\n{suffix}\n```"
        assistant = f"```rust\n{out.get('completion', '')}\n```"

    elif cat == "code_refactoring":
        before = inp.get("code_before", "")
        user = f"Refactor this Rust code to improve readability while preserving logic.\n\n```rust\n{before}\n```"
        rationale = out.get("rationale", "")
        after = out.get("code_after", "")
        assistant = f"**Rationale:** {rationale}\n\n**Refactored code:**\n```rust\n{after}\n```"

    elif cat == "code_optimization":
        before = inp.get("code_before", "")
        user = f"Optimize this Rust code.\n\n```rust\n{before}\n```"
        rationale = out.get("rationale", "")
        after = out.get("code_after", "")
        assistant = f"**Rationale:** {rationale}\n\n**Optimized code:**\n```rust\n{after}\n```"

    elif cat == "code_search":
        query = inp.get("query", "")
        ctx = inp.get("code_context", "")
        user = f"Find Rust code relevant to this query: {query}"
        if ctx:
            user += f"\n\nContext:\n```rust\n{ctx}\n```"
        assistant = f"```rust\n{out.get('code_snippet', '')}\n```"

    elif cat == "test_generation":
        code_to_test = inp.get("code_to_test", "")
        test_ctx = inp.get("test_context", "")
        ctx = inp.get("code_context", "")
        user = f"Generate unit tests for this Rust code.\n\n"
        if ctx:
            user += f"Context:\n```rust\n{ctx}\n```\n\n"
        user += f"Code to test:\n```rust\n{code_to_test}\n```"
        if test_ctx:
            user += f"\n\nTest context:\n```rust\n{test_ctx}\n```"
        assistant = f"```rust\n{out.get('test_cases', '')}\n```"

    elif cat == "api_usage_prediction":
        code = inp.get("code", "")
        ctx = inp.get("code_context", "")
        user = f"Predict the next API call or usage pattern in this Rust context.\n\n"
        if ctx:
            user += f"Context:\n```rust\n{ctx}\n```\n\n"
        user += f"Code:\n```rust\n{code}\n```"
        assistant = out.get("next_api_call", "")

    else:
        # Fallback: use raw fields
        user = json.dumps(inp, indent=2)
        assistant = json.dumps(out, indent=2)

    return {"messages": [
        {"role": "system", "content": SYSTEM_CODING},
        {"role": "user", "content": user},
        {"role": "assistant", "content": assistant},
    ]}


def format_introspector(example):
    """Convert a rust-analyser semantic analysis record to ChatML messages."""
    phase = example.get("phase", "unknown")
    snippet = example.get("source_snippet", "")
    element_type = example.get("element_type", "")
    element_name = example.get("element_name", "") or ""
    element_sig = example.get("element_signature", "") or ""
    symbol_data = parse_json_field(example.get("symbol_data"))
    type_data = parse_json_field(example.get("type_data"))
    syntax_data = parse_json_field(example.get("syntax_data"))
    ctx_before = example.get("context_before", "") or ""

    if phase == "name_resolution":
        user = f"Analyze this Rust code and identify the symbols defined.\n\n"
        if ctx_before:
            user += f"Context before:\n```rust\n{ctx_before}\n```\n\n"
        user += f"Code:\n```rust\n{snippet}\n```"
        parts = [f"Element type: {element_type}"]
        if element_name:
            parts.append(f"Name: {element_name}")
        if element_sig:
            parts.append(f"Signature: {element_sig}")
        if symbol_data:
            parts.append(f"Symbol data: {json.dumps(symbol_data, indent=2)}")
        assistant = "\n".join(parts)

    elif phase == "type_inference":
        user = f"What is the type information for this Rust code?\n\n```rust\n{snippet}\n```"
        parts = []
        if element_name:
            parts.append(f"Element: {element_name}")
        if element_type:
            parts.append(f"Type: {element_type}")
        if type_data:
            parts.append(f"Type data: {json.dumps(type_data, indent=2)}")
        assistant = "\n".join(parts) if parts else "No type information available."

    elif phase == "parsing":
        user = f"Parse this Rust code and describe the syntax structure.\n\n```rust\n{snippet}\n```"
        parts = [f"Element type: {element_type}"]
        if element_name:
            parts.append(f"Name: {element_name}")
        if syntax_data:
            parts.append(f"Syntax data: {json.dumps(syntax_data, indent=2)}")
        assistant = "\n".join(parts)

    else:
        user = f"Analyze this Rust code.\n\n```rust\n{snippet}\n```"
        parts = [f"Phase: {phase}", f"Element type: {element_type}"]
        if element_name:
            parts.append(f"Name: {element_name}")
        if element_sig:
            parts.append(f"Signature: {element_sig}")
        assistant = "\n".join(parts)

    return {"messages": [
        {"role": "system", "content": SYSTEM_ANALYSIS},
        {"role": "user", "content": user},
        {"role": "assistant", "content": assistant},
    ]}


def load_and_format(dataset_id, formatter, split="train", max_samples=0):
    """Load a HF dataset, format to ChatML, and optionally truncate."""
    print(f"Loading {dataset_id} (split={split})...", flush=True)
    ds = load_dataset(dataset_id, split=split)
    print(f"  Loaded {len(ds)} records", flush=True)

    # Format to messages
    ds = ds.map(formatter, remove_columns=ds.column_names, num_proc=4)
    print(f"  Formatted {len(ds)} examples", flush=True)

    # Filter out empty examples
    def is_valid(ex):
        msgs = ex.get("messages", [])
        if len(msgs) < 3:
            return False
        for m in msgs:
            if not m.get("content", "").strip():
                return False
        return True
    ds = ds.filter(is_valid, num_proc=4)
    print(f"  Valid examples: {len(ds)}", flush=True)

    if max_samples > 0 and len(ds) > max_samples:
        indices = random.sample(range(len(ds)), max_samples)
        ds = ds.select(indices)
        print(f"  Truncated to {len(ds)} samples", flush=True)

    return ds



# Keep all data for training — eval is dynamically re-sampled from the
# training set before each eval step (2%, non-contiguous, re-shuffled each time).
# This prevents overfitting to a fixed eval set and keeps eval time reasonable.
EVAL_RATIO = 0.02

# ── Load model BEFORE datasets ─────────────────────────────────────────────
# Model weights must be fully downloaded and cached before any dataset .map()
# calls spawn multiprocessing workers. Otherwise the workers and the main
# process compete for HuggingFace cache locks, causing a futex deadlock.

print(f"\nLoading {MODEL_ID}...", flush=True)
model, tokenizer = FastLanguageModel.from_pretrained(
    model_name=MODEL_ID, max_seq_length=MAX_SEQ_LENGTH,
    load_in_4bit=False, load_in_16bit=True, full_finetuning=False,
)
print(f"GPU: {torch.cuda.memory_allocated()/1e9:.1f}GB", flush=True)

model = FastLanguageModel.get_peft_model(
    model, r=LORA_R, lora_alpha=LORA_ALPHA,
    target_modules=TARGET_MODULES,
    lora_dropout=LORA_DROPOUT, bias="none",
    use_gradient_checkpointing="unsloth", random_state=SEED,
    max_seq_length=MAX_SEQ_LENGTH,
    init_lora_weights=INIT_LORA_WEIGHTS,
)
tr = sum(p.numel() for p in model.parameters() if p.requires_grad)
tt = sum(p.numel() for p in model.parameters())
print(f"Trainable: {tr/1e6:.1f}M / {tt/1e9:.2f}B ({100*tr/tt:.2f}%)", flush=True)

# ── Load and format datasets ───────────────────────────────────────────────

# In 'both' mode, use the full combined dataset. The deadlock was caused by
# model download competing with dataset workers for HF cache locks, not by
# dataset size. Loading the model first fixes it.
MAX_PER_DATASET = MAX_SAMPLES

datasets_list = []

if MODE in ("coding", "both", "all"):
    coding_ds = load_and_format(
        "Fortytwo-Network/Strandset-Rust-v1",
        format_strandset,
        split="train",
        max_samples=MAX_PER_DATASET,
    )
    datasets_list.append(("coding", coding_ds))

if MODE in ("analysis", "both", "all"):
    analysis_ds = load_and_format(
        "introspector/rust-analyser",
        format_introspector,
        split="train",
        max_samples=MAX_PER_DATASET,
    )
    datasets_list.append(("analysis", analysis_ds))

if MODE in ("reasoning", "all"):
    # OpenThoughts-114k linked dataset — clean ChatML from HF dataset repo
    print("Loading Axolotl-Partners/openthoughts-114k-linked (split=train)...", flush=True)
    reasoning_ds = load_dataset("json", data_files="hf://datasets/Axolotl-Partners/openthoughts-114k-linked/train.jsonl", split="train")
    print(f"  Loaded {len(reasoning_ds)} records", flush=True)
    # Already in ChatML format (messages field) — just validate
    def is_valid_msg(ex):
        msgs = ex.get("messages", [])
        return len(msgs) >= 3 and all(m.get("content", "").strip() for m in msgs)
    reasoning_ds = reasoning_ds.filter(is_valid_msg, num_proc=4)
    print(f"  Valid examples: {len(reasoning_ds)}", flush=True)
    if MAX_PER_DATASET > 0 and len(reasoning_ds) > MAX_PER_DATASET:
        indices = random.sample(range(len(reasoning_ds)), MAX_PER_DATASET)
        reasoning_ds = reasoning_ds.select(indices)
        print(f"  Truncated to {len(reasoning_ds)} samples", flush=True)
    datasets_list.append(("reasoning", reasoning_ds))

if not datasets_list:
    print(f"FATAL: Unknown MODE={MODE}. Use 'coding', 'analysis', 'reasoning', 'both', or 'all'.", flush=True)
    sys.exit(1)

# Combine datasets
if len(datasets_list) == 1:
    train_ds = datasets_list[0][1]
    print(f"\nTraining on {datasets_list[0][0]}: {len(train_ds)} examples", flush=True)
else:
    train_ds = concatenate_datasets([ds for _, ds in datasets_list]).shuffle(seed=SEED)
    print(f"\nTraining on combined: {len(train_ds)} examples", flush=True)
    for name, ds in datasets_list:
        print(f"  {name}: {len(ds)} examples", flush=True)

eval_size = max(1, int(len(train_ds) * EVAL_RATIO))
print(f"Train: {len(train_ds)} | Eval: {eval_size} dynamically re-sampled ({EVAL_RATIO*100:.1f}%)", flush=True)

# ── Format with chat template ──────────────────────────────────────────────

def fmt_msgs(ex):
    return {"text": tokenizer.apply_chat_template(ex["messages"], tokenize=False, add_generation_prompt=False)}

print("Formatting with chat template...", flush=True)
train_ds = train_ds.map(fmt_msgs, remove_columns=train_ds.column_names, num_proc=4)

# ── Train ─────────────────────────────────────────────────────────────────

# Dynamic eval: override the public evaluate() method to re-sample before eval.
# The public API is stable across transformers versions, unlike _evaluate().
from trl import SFTTrainer as _BaseSFTTrainer

class DynamicEvalSFTTrainer(_BaseSFTTrainer):
    """SFTTrainer that re-samples eval examples from the training set before each eval."""

    def evaluate(self, eval_dataset=None, ignore_keys=None, metric_key_prefix="eval"):
        # Re-sample eval set from the training set
        n = len(self.train_dataset)
        k = max(1, int(n * EVAL_RATIO))
        indices = random.sample(range(n), k)
        eval_dataset = self.train_dataset.select(indices)
        print(f"  [dynamic eval] sampled {k} examples from {n} training examples", flush=True)
        return super().evaluate(eval_dataset=eval_dataset, ignore_keys=ignore_keys, metric_key_prefix=metric_key_prefix)

trainer = DynamicEvalSFTTrainer(
    model=model, processing_class=tokenizer,
    train_dataset=train_ds, eval_dataset=train_ds.select(range(min(100, len(train_ds)))),
    callbacks=[EarlyStoppingCallback(early_stopping_patience=7)],
    args=SFTConfig(
        max_seq_length=MAX_SEQ_LENGTH, dataset_text_field="text",
        per_device_train_batch_size=PER_DEVICE_BATCH,
        gradient_accumulation_steps=GRAD_ACCUM,
        num_train_epochs=NUM_EPOCHS,
        learning_rate=LEARNING_RATE,
        warmup_steps=WARMUP_STEPS,
        logging_steps=10, eval_strategy="steps", eval_steps=EVAL_STEPS,
        load_best_model_at_end=True, metric_for_best_model="eval_loss",
        greater_is_better=False, save_strategy="steps", save_steps=SAVE_STEPS,
        save_total_limit=3, bf16=True, optim="adamw_8bit",
        weight_decay=WEIGHT_DECAY, max_grad_norm=MAX_GRAD_NORM,
        lr_scheduler_type="cosine", output_dir=OUTPUT_DIR,
        report_to="none", dataset_num_proc=1,
    ),
)
print("\n=== STARTING TRAINING ===", flush=True)
trainer.train()
print("=== TRAINING COMPLETE ===", flush=True)
print(f"Finished: {time.strftime('%Y-%m-%dT%H:%M:%SZ', time.gmtime())}", flush=True)

model.save_pretrained(OUTPUT_DIR)
tokenizer.save_pretrained(OUTPUT_DIR)
print(f"Saved to {OUTPUT_DIR}", flush=True)

if HF_TOKEN:
    print("Uploading to HuggingFace...", flush=True)
    from huggingface_hub import HfApi
    api = HfApi(token=HF_TOKEN)
    api.create_repo(repo_id=HF_REPO, repo_type="model", exist_ok=True)
    api.upload_folder(
        folder_path=OUTPUT_DIR, path_in_repo=".",
        repo_id=HF_REPO, repo_type="model",
    )
    print(f"Uploaded to {HF_REPO}", flush=True)
else:
    print("No HF_TOKEN. Model saved locally.", flush=True)
PYEOF

# Set HF_REPO based on MODE
if [ "$MODE" = "analysis" ]; then
    export HF_REPO="Axolotl-Partners/qwen36-rust-analysis-lora"
elif [ "$MODE" = "both" ]; then
    export HF_REPO="Axolotl-Partners/qwen36-rust-combined-lora"
elif [ "$MODE" = "reasoning" ]; then
    export HF_REPO="Axolotl-Partners/qwen36-reasoning-lora"
elif [ "$MODE" = "all" ]; then
    export HF_REPO="Axolotl-Partners/qwen36-rust-reasoning-all-lora"
else
    export HF_REPO="Axolotl-Partners/qwen36-rust-coding-lora"
fi

START_TS=$(date +%s); echo "=== RUNNING TRAINING (MODE=${MODE}, REPO=${HF_REPO}) ==="
set +e
python3 train_rust.py
EXIT_CODE=$?
set -e
ELAPSED=$(( $(date +%s) - START_TS ))
echo "=== FINISHED (exit=${EXIT_CODE}, ${ELAPSED}s) ==="

if [ "${EXIT_CODE}" -eq 0 ]; then
    POD_ID=$(hostname 2>/dev/null || echo "")
    if [ -n "${POD_ID}" ] && [ -n "${RUNPOD_API_KEY:-}" ]; then
        echo "Training OK. Terminating pod ${POD_ID} in 60s (Ctrl-C to cancel)..."
        trap 'echo "Auto-terminate cancelled."; exit 0' INT
        sleep 60
        trap - INT
        curl -s --max-time 30 -X POST \
            "https://api.runpod.io/graphql?api_key=${RUNPOD_API_KEY}" \
            -H "Content-Type: application/json" \
            -d "{\"query\":\"mutation { podTerminate(input: {podId: \\\"${POD_ID}\\\"}) }\"}"
        echo "Pod terminated."
    else
        echo "Pod kept alive (no pod ID or API key). Check RunPod dashboard."
    fi
else
    echo "Training FAILED (exit=${EXIT_CODE}). Pod kept alive for debugging."
fi
