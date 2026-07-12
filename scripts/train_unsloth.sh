#!/bin/bash
# ============================================================================
# Qwen3.6-27B Unsloth Training — runs on RunPod pod
# ============================================================================
# Image: runpod/pytorch:2.4.0-py3.11-cuda12.4.1-devel-ubuntu22.04
# Usage: curl -sL <this-script-url> | bash
# ============================================================================
set -euo pipefail
exec > /workspace/training.log 2>&1

# Direct RunPod SSH sessions omit custom pod variables; recover only the two
# credentials needed by this script from the container's initial environment.
if [ -r /proc/1/environ ]; then
    while IFS= read -r -d '' entry; do
        case "$entry" in
            HF_TOKEN=*|RUNPOD_API_KEY=*) export "${entry?}" ;;
        esac
    done < /proc/1/environ
fi


echo "============================================"
echo "Qwen3.6 Unsloth Training | $(date -u)"
echo "GPU: $(nvidia-smi --query-gpu=name --format=csv,noheader 2>/dev/null || echo 'unknown')"
echo "CUDA: $(nvidia-smi | grep 'CUDA Version' | awk '{print $9}' 2>/dev/null || echo 'unknown')"
echo "Torch: $(python3 -c 'import torch; print(torch.__version__)' 2>/dev/null || echo '?')"
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

# Keep the datasets runtime within the version range tested by this pipeline.
echo "datasets>=3.4.1,<5.0" > /tmp/pip-constraints.txt
export PIP_CONSTRAINT=/tmp/pip-constraints.txt

echo "=== INSTALLING UNSLOTH ==="
pip install --cache-dir "$PIP_CACHE_DIR" -q unsloth unsloth_zoo

echo "=== INSTALLING CORE ==="
pip install --cache-dir "$PIP_CACHE_DIR" -q \
    "transformers>=5.2" \
    datasets trl peft accelerate requests huggingface_hub

# Qwen3.6 uses gated-deltanet layers. FLA supplies the optimized kernels;
# TileLang is required for correct Hopper backward passes with Triton >=3.4.
echo "=== INSTALLING QWEN KERNELS ==="
pip install --cache-dir "$PIP_CACHE_DIR" -q \
    "flash-linear-attention[cuda]" tilelang

unset PIP_CONSTRAINT

python3 -c "
import torch
import fla, tilelang
fa = torch.backends.cuda.flash_sdp_enabled() if hasattr(torch.backends.cuda, 'flash_sdp_enabled') else False
print(f'SDPA-FlashAttn: {fa} | BF16: {torch.cuda.is_bf16_supported()}')
"
echo "Install complete."
cd /workspace

cat > train_unsloth.py << 'PYEOF'
import os, sys, json, time, random, requests
from pathlib import Path
import torch
from datasets import load_dataset, concatenate_datasets, Dataset
from unsloth import FastLanguageModel
from unsloth.chat_templates import standardize_sharegpt
from trl import SFTTrainer, SFTConfig
from transformers import EarlyStoppingCallback

MODEL_ID = "unsloth/Qwen3.6-27B"
DATA_REVISION = "916c1ecbcab81d34e0d23b1c1e7ea56e34977608"
MAX_SEQ_LENGTH = 6144
OUTPUT_DIR = os.environ.get("OUTPUT_DIR", "/workspace/outputs")
HF_TOKEN = os.environ.get("HF_TOKEN", "")
HF_REPO = "Axolotl-Partners/qwen36-distill-opus-dsv4-lora"
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
REASONING_RATIO = 0.75

HYPERPARAMS = {
    "model": MODEL_ID, "max_seq_length": MAX_SEQ_LENGTH,
    "lora_r": LORA_R, "lora_alpha": LORA_ALPHA, "lora_dropout": LORA_DROPOUT,
    "target_modules": TARGET_MODULES,
    "learning_rate": LEARNING_RATE, "num_epochs": NUM_EPOCHS,
    "batch_size": PER_DEVICE_BATCH, "grad_accum": GRAD_ACCUM,
    "warmup_steps": WARMUP_STEPS, "max_grad_norm": MAX_GRAD_NORM,
    "weight_decay": WEIGHT_DECAY, "reasoning_ratio": REASONING_RATIO,
    "seed": SEED,
}
random.seed(SEED)

print("=" * 60)
print(f"Qwen3.6 Unsloth Training | {time.strftime('%Y-%m-%d %H:%M:%S')} UTC")
print(f"GPU: {torch.cuda.get_device_name(0) if torch.cuda.is_available() else 'NONE'}")
Path(OUTPUT_DIR).mkdir(parents=True, exist_ok=True)
disk_gb = os.statvfs(OUTPUT_DIR).f_frsize * os.statvfs(OUTPUT_DIR).f_bavail / 1e9
print(f"Disk free: {disk_gb:.1f}GB | GPU free: {torch.cuda.mem_get_info()[0]/1e9:.1f}GB")

print("\n=== EXPERIMENT CONFIG ===")
for k, v in HYPERPARAMS.items():
    print(f"  {k}: {v}")
print(f"  timestamp: {time.strftime('%Y-%m-%dT%H:%M:%SZ', time.gmtime())}")
print(f"  torch: {torch.__version__}")

print("\n--- Validating datasets ---")
print("Checking: Axolotl-Partners/qwen36-distill-opus-dsv4")
data_base = f"hf://datasets/Axolotl-Partners/qwen36-distill-opus-dsv4@{DATA_REVISION}"
data_files = {
    "train": [f"{data_base}/stage{i}/train.jsonl" for i in range(1, 4)],
    "test": [f"{data_base}/stage{i}/test.jsonl" for i in range(1, 4)],
}
distill = load_dataset("json", data_files=data_files)
ds1 = distill["train"]
ev = distill["test"]
s = ds1[0]
assert "messages" in s, f"opus-dsv4 missing messages: {list(s.keys())}"
assert len(s["messages"]) >= 2, "opus-dsv4 <2 messages"
print(f"  train: {len(ds1)} samples, OK")
print(f"  test: {len(ev)} samples")

print("Checking: mlabonne/FineTome-100k")
chat_ds = load_dataset("mlabonne/FineTome-100k", split="train")
assert "conversations" in chat_ds[0], "FineTome missing conversations"
print(f"  {len(chat_ds)} samples, OK")
print("All datasets validated. Loading model.\n")

print(f"Loading {MODEL_ID}...")
model, tokenizer = FastLanguageModel.from_pretrained(
    model_name=MODEL_ID, max_seq_length=MAX_SEQ_LENGTH,
    load_in_4bit=False, load_in_16bit=True, full_finetuning=False,
)
print(f"GPU: {torch.cuda.memory_allocated()/1e9:.1f}GB")
tok = getattr(tokenizer, 'tokenizer', tokenizer)  # Qwen3.6 returns Processor, not Tokenizer

model = FastLanguageModel.get_peft_model(
    model, r=LORA_R, lora_alpha=LORA_ALPHA,
    target_modules=TARGET_MODULES,
    lora_dropout=LORA_DROPOUT, bias="none",
    use_gradient_checkpointing="unsloth", random_state=SEED,
    max_seq_length=MAX_SEQ_LENGTH,
)
tr = sum(p.numel() for p in model.parameters() if p.requires_grad)
tt = sum(p.numel() for p in model.parameters())
print(f"Trainable: {tr/1e6:.1f}M / {tt/1e9:.2f}B ({100*tr/tt:.2f}%)")

def fmt_msgs(ex):
    return {"text": tokenizer.apply_chat_template(ex["messages"], tokenize=False, add_generation_prompt=False)}

def fmt_conv(ex):
    return {"text": tokenizer.apply_chat_template(ex["conversations"], tokenize=False, add_generation_prompt=False)}

def validate(name, ds, fn):
    x = ds.select(range(min(3, len(ds))))
    for i, ex in enumerate(x):
        r = fn(ex)
        assert r["text"] and len(r["text"]) >= 20, f"FATAL: {name}[{i}] short ({len(r.get('text',''))})"
        assert "assistant" in r["text"].lower(), f"FATAL: {name}[{i}] no assistant"
    print(f"  {name}: OK ({len(r['text'])} chars)")

print("\n--- Token length analysis ---")
def measure_lengths(name, ds, fn, n=500):
    sample = ds.select(range(min(n, len(ds))))
    lengths = []
    for ex in sample:
        r = fn(ex)
        lengths.append(len(tok.encode(r["text"])))
    lengths.sort()
    print(f"  {name} (n={len(lengths)}): "
          f"min={lengths[0]} p50={lengths[len(lengths)//2]} "
          f"p95={lengths[int(len(lengths)*0.95)]} max={lengths[-1]}")
    return lengths

opus_lens = measure_lengths("opus-dsv4", ds1, fmt_msgs)

print("\n--- Reasoning data ---")
validate("opus-dsv4", ds1, fmt_msgs)
ds1 = ds1.map(fmt_msgs, remove_columns=ds1.column_names)
n_opus = len(ds1)
print(f"Reasoning: {n_opus} samples (opus-dsv4 only)")

print("\n--- Chat data (25%) ---")
nc = int(n_opus * (1 - REASONING_RATIO) / REASONING_RATIO)
nc = min(nc, len(chat_ds))
chat = chat_ds.select(random.sample(range(len(chat_ds)), nc))
chat = standardize_sharegpt(chat)
validate("finetome", chat, fmt_conv)
chat = chat.map(fmt_conv, remove_columns=chat.column_names)
print(f"Chat: {len(chat)} ({100*len(chat)/(n_opus+len(chat)):.0f}%)")

train_ds = concatenate_datasets([ds1, chat]).shuffle(seed=SEED)
print(f"\nTraining: {len(train_ds)} total")
print(f"Data composition: {n_opus} reasoning + {len(chat)} chat = {len(train_ds)}")
eval_ds = ev.map(fmt_msgs, remove_columns=ev.column_names)
print(f"Eval: {len(eval_ds)}")

p95_tokens = opus_lens[int(len(opus_lens) * 0.95)]
if p95_tokens < MAX_SEQ_LENGTH * 0.5:
    print(f"P95={p95_tokens} vs max_seq={MAX_SEQ_LENGTH}")
    adjusted = int(p95_tokens * 1.2)
    print(f"MAX_SEQ adjusted: {MAX_SEQ_LENGTH} -> {adjusted} (P95={p95_tokens})")
    MAX_SEQ_LENGTH = adjusted

trainer = SFTTrainer(
    model=model, processing_class=tokenizer,
    train_dataset=train_ds, eval_dataset=eval_ds,
    callbacks=[EarlyStoppingCallback(early_stopping_patience=10)],
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
print("\n=== STARTING TRAINING ===")
trainer.train()
print("=== TRAINING COMPLETE ===")
print(f"Finished: {time.strftime('%Y-%m-%dT%H:%M:%SZ', time.gmtime())}")

model.save_pretrained(OUTPUT_DIR)
tokenizer.save_pretrained(OUTPUT_DIR)
print(f"Saved to {OUTPUT_DIR}")

if HF_TOKEN:
    print("Uploading to HuggingFace...")
    from huggingface_hub import HfApi
    api = HfApi(token=HF_TOKEN)
    api.create_repo(repo_id=HF_REPO, repo_type="model", exist_ok=True)
    api.upload_folder(
        folder_path=OUTPUT_DIR, path_in_repo=".",
        repo_id=HF_REPO, repo_type="model",
    )
    print("Uploaded")
else:
    print("No HF_TOKEN. Model saved locally.")
PYEOF

START_TS=$(date +%s); echo "=== RUNNING TRAINING ==="
set +e
python3 train_unsloth.py
EXIT_CODE=$?
set -e
ELAPSED=$(( $(date +%s) - START_TS ))
COST=$(python3 -c "print(f'{${ELAPSED}/3600*1.39:.2f}')")
echo "=== FINISHED (exit=${EXIT_CODE}, ~\$${COST}) ==="
if [ "${EXIT_CODE}" -eq 0 ]; then
    # Discover pod ID via hostname (RunPod sets hostname = pod ID)
    POD_ID=$(hostname 2>/dev/null || echo "")
    if [ -n "${POD_ID}" ] && [ -n "${RUNPOD_API_KEY:-}" ]; then
        echo "Training OK. Terminating pod ${POD_ID} in 60s (Ctrl-C to cancel)..."
        trap 'echo "Auto-terminate cancelled. Pod will stay alive."; exit 0' INT
        sleep 60
        trap - INT
        TERM_RESP=$(curl -s --max-time 30 -X POST \
            "https://api.runpod.io/graphql?api_key=${RUNPOD_API_KEY}" \
            -H "Content-Type: application/json" \
            -d "{\"query\":\"mutation { podTerminate(input: {podId: \\\"${POD_ID}\\\"}) }\"}")
        if echo "$TERM_RESP" | python3 -c "import sys,json; r=json.load(sys.stdin); raise SystemExit(1 if r.get('errors') else 0)"; then
            echo "Pod terminated."
        else
            echo "WARNING: Pod termination failed. Check dashboard: https://www.runpod.io/console/pods/${POD_ID}"
        fi
    else
        echo "Pod kept alive (no pod ID discovered). Check RunPod dashboard."
    fi
else
    echo "Training FAILED (exit=${EXIT_CODE}). Pod kept alive for debugging."
fi
