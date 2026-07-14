#!/bin/bash
# ============================================================================
# Capabilities Researcher — PiSSA LoRA Training — runs on RunPod pod
# ============================================================================
# Image: runpod/pytorch:2.4.0-py3.11-cuda12.4.1-devel-ubuntu22.04
# Usage: curl -sL <this-script-url> | bash
#
# Trains a LoRA adapter on Qwen3.6-27B with PiSSA initialization
# from the Capabilities Researcher corpus (80,947 QAs).
# ============================================================================
set -euo pipefail
exec > /workspace/training.log 2>&1

trap 'cp /workspace/training.log /workspace/crash_$(date +%s).log 2>/dev/null || true' ERR TERM

if [ -r /proc/1/environ ]; then
    while IFS= read -r -d '' entry; do
        case "$entry" in
            HF_TOKEN=*|RUNPOD_API_KEY=*) export "${entry?}" ;;
        esac
    done < /proc/1/environ
fi

echo "============================================"
echo "Capabilities Researcher Training | $(date -u)"
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
pip install --cache-dir "$PIP_CACHE_DIR" -q "flash-linear-attention[cuda]" tilelang
pip install --cache-dir "$PIP_CACHE_DIR" -q causal-conv1d --no-build-isolation

unset PIP_CONSTRAINT

python3 -c "
import torch
print(f'Torch: {torch.__version__} | CUDA: {torch.version.cuda}')
print(f'BF16: {torch.cuda.is_bf16_supported()}')
"
echo "Install complete."
cd /workspace

cat > train_capabilities.py << 'PYEOF'
import os, sys, json, time, random
from pathlib import Path
import torch
from datasets import load_dataset
from unsloth import FastLanguageModel
from trl import SFTTrainer, SFTConfig
from transformers import EarlyStoppingCallback
from peft import LoraConfig, get_peft_model

MODEL_ID = "unsloth/Qwen3.6-27B"
MAX_SEQ_LENGTH = 4096
OUTPUT_DIR = os.environ.get("OUTPUT_DIR", "/workspace/outputs")
HF_TOKEN = os.environ.get("HF_TOKEN", "")
HF_REPO = os.environ.get("HF_REPO", "mdz-axolotl/capabilities-researcher-pissa-lora")
# Dataset: local JSONL files (uploaded via SCP)
TRAIN_FILE = "/workspace/data/train_chat.jsonl"
VAL_FILE = "/workspace/data/val_chat.jsonl"
SEED = 3407

# PiSSA LoRA configuration
# r=16: balances expressiveness with knowledge preservation for QA tasks
# alpha=32: 2:1 alpha:r ratio (standard practice)
# dropout=0: REQUIRED for PiSSA — principal components must not be randomly dropped
LORA_R = 16
LORA_ALPHA = 32
LORA_DROPOUT = 0
TARGET_MODULES = ["q_proj","k_proj","v_proj","o_proj","gate_proj","up_proj","down_proj"]
INIT_LORA_WEIGHTS = "pissa_niter_4"  # Fast SVD, <2 min init for 27B

# Training
LEARNING_RATE = 1e-4   # Lower LR — PiSSA converges faster than LoRA
NUM_EPOCHS = 3
PER_DEVICE_BATCH = 1
GRAD_ACCUM = 16
WARMUP_STEPS = 100     # PiSSA needs less warmup than LoRA
EVAL_STEPS = 200
SAVE_STEPS = 200
MAX_GRAD_NORM = 0.3
WEIGHT_DECAY = 0.01
EARLY_STOPPING_PATIENCE = 25

random.seed(SEED)
Path(OUTPUT_DIR).mkdir(parents=True, exist_ok=True)

print("=" * 60)
print(f"Capabilities Researcher Training | {time.strftime('%Y-%m-%d %H:%M:%S')}")
print(f"GPU: {torch.cuda.get_device_name(0)}")
print(f"LoRA: r={LORA_R}, alpha={LORA_ALPHA}, dropout={LORA_DROPOUT}")
print(f"PiSSA init: {INIT_LORA_WEIGHTS}")
print(f"LR: {LEARNING_RATE}, epochs={NUM_EPOCHS}, warmup={WARMUP_STEPS}")
print(f"Dataset: {TRAIN_FILE}")
print("=" * 60)

# ── Load model BEFORE datasets (avoids HF cache lock deadlock) ─────────────
print(f"\nLoading {MODEL_ID}...", flush=True)
model, tokenizer = FastLanguageModel.from_pretrained(
    model_name=MODEL_ID, max_seq_length=MAX_SEQ_LENGTH,
    load_in_4bit=False, load_in_16bit=True, full_finetuning=False,
    attn_implementation="sdpa",
)
print(f"GPU: {torch.cuda.memory_allocated()/1e9:.1f}GB", flush=True)

# Apply PEFT LoRA with PiSSA initialization via HuggingFace PEFT
# (not Unsloth's get_peft_model — that doesn't support pissa_niter_4)
peft_config = LoraConfig(
    r=LORA_R,
    lora_alpha=LORA_ALPHA,
    target_modules=TARGET_MODULES,
    lora_dropout=LORA_DROPOUT,
    bias="none",
    task_type="CAUSAL_LM",
    init_lora_weights=INIT_LORA_WEIGHTS,
)
model = get_peft_model(model, peft_config)
model.gradient_checkpointing_enable()

tr = sum(p.numel() for p in model.parameters() if p.requires_grad)
tt = sum(p.numel() for p in model.parameters())
print(f"Trainable: {tr/1e6:.1f}M / {tt/1e9:.2f}B ({100*tr/tt:.2f}%)", flush=True)

# ── Load dataset from local JSONL files ────────────────────────────────────
print(f"\nLoading {TRAIN_FILE}...", flush=True)
train_ds = load_dataset("json", data_files=TRAIN_FILE, split="train")
print(f"  Train: {len(train_ds)} examples", flush=True)

val_ds = load_dataset("json", data_files=VAL_FILE, split="train")
print(f"  Val: {len(val_ds)} examples", flush=True)

# Hold out a fixed eval split
EVAL_SIZE = min(1000, len(val_ds))
eval_ds = val_ds.select(range(EVAL_SIZE))
print(f"Train: {len(train_ds)} | Eval: {len(eval_ds)}", flush=True)

# Validate format
def is_valid(ex):
    msgs = ex.get("messages", [])
    return len(msgs) >= 3 and all(m.get("content", "").strip() for m in msgs)

train_ds = train_ds.filter(is_valid, num_proc=4)
val_ds = val_ds.filter(is_valid, num_proc=4)
print(f"Valid — Train: {len(train_ds)} | Eval: {len(val_ds)}", flush=True)

# ── Format with chat template ──────────────────────────────────────────────
def fmt_msgs(ex):
    return {"text": tokenizer.apply_chat_template(
        ex["messages"], tokenize=False, add_generation_prompt=False
    )}

print("Formatting with chat template...", flush=True)
train_ds = train_ds.map(fmt_msgs, remove_columns=train_ds.column_names, num_proc=4)
val_ds = val_ds.map(fmt_msgs, remove_columns=val_ds.column_names, num_proc=4)

# ── Train ──────────────────────────────────────────────────────────────────
trainer = SFTTrainer(
    model=model, processing_class=tokenizer,
    train_dataset=train_ds, eval_dataset=val_ds,
    callbacks=[EarlyStoppingCallback(early_stopping_patience=EARLY_STOPPING_PATIENCE)],
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
        save_total_limit=5, bf16=True, optim="adamw_8bit",
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

START_TS=$(date +%s); echo "=== RUNNING TRAINING ==="
set +e
python3 train_capabilities.py
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