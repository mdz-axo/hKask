#!/bin/bash
# ============================================================================
# Qwen3.6-27B Evaluation — runs on RunPod pod
# ============================================================================
# Image: runpod/pytorch:2.4.0-py3.11-cuda12.4.1-devel-ubuntu22.04
# Usage: curl -sL <this-script-url> | bash
# ============================================================================
set -euo pipefail
exec > /workspace/eval.log 2>&1

if [ -r /proc/1/environ ]; then
    while IFS= read -r -d '' entry; do
        case "$entry" in HF_TOKEN=*) export "${entry?}" ;; esac
    done < /proc/1/environ
fi

echo "============================================"
echo "Qwen3.6 Eval | $(date -u)"
echo "GPU: $(nvidia-smi --query-gpu=name --format=csv,noheader 2>/dev/null || echo 'unknown')"
echo "============================================"

export HF_HOME=/workspace/.cache/huggingface
export PIP_CACHE_DIR=/workspace/.cache/pip
mkdir -p "$HF_HOME" "$PIP_CACHE_DIR"

echo "" && echo "=== INSTALLING DEPS ==="
pip install --cache-dir "$PIP_CACHE_DIR" -q unsloth unsloth_zoo 2>&1 | tail -3
pip install --cache-dir "$PIP_CACHE_DIR" -q datasets peft tqdm 2>&1 | tail -3
python3 -c "
import unsloth, torch
print(f'Unsloth {unsloth.__version__} | Torch {torch.__version__}')
print(f'GPU: {torch.cuda.get_device_name(0)} | BF16: {torch.cuda.is_bf16_supported()}')
"

echo "" && echo "=== DOWNLOADING ADAPTER ==="
ADAPTER_DIR="/workspace/adapter"
CHECKPOINT="${CHECKPOINT:-400}"
python3 -c "
from huggingface_hub import snapshot_download
snapshot_download(
    repo_id='Axolotl-Partners/qwen36-distill-opus-dsv4-lora',
    repo_type='model',
    allow_patterns=[f'checkpoint-${CHECKPOINT}/*'],
    local_dir='${ADAPTER_DIR}',
)
"
if [ -d "${ADAPTER_DIR}/checkpoint-${CHECKPOINT}" ]; then
    cp "${ADAPTER_DIR}/checkpoint-${CHECKPOINT}"/* "${ADAPTER_DIR}/"
fi
ls -lh "${ADAPTER_DIR}/adapter_model.safetensors"
ls -lh "${ADAPTER_DIR}/adapter_config.json"

cat > /workspace/eval_qwen36.py << 'PYEOF'
import argparse, json, re, time, random, os
from pathlib import Path
import torch
from datasets import load_dataset
from tqdm import tqdm

def parse_mc_answer(text, n_opts=4):
    # Extract answer after </thinking> if present
    if "</thinking>" in text:
        text = text.rsplit("</thinking>", 1)[-1].strip()
    else:
        text = text.strip()
    # Try explicit "answer is X" / "Answer: X" patterns first
    for pattern in [
        r"(?:answer is|Answer:|answer:|correct answer is)\s*\(?([A-J])\)?",
        r"\b([A-J])\b\s*$",
    ]:
        m = re.search(pattern, text, re.IGNORECASE)
        if m:
            return m.group(1).upper()
    # Fallback: last standalone letter A-J
    matches = list(re.finditer(r"\b([A-J])\b", text))
    if matches:
        return matches[-1].group(1)
    return None

def parse_math_answer(text):
    if "</thinking>" in text:
        text = text.rsplit("</thinking>", 1)[-1].strip()
    else:
        text = text.strip()
    m = re.search(r"\\boxed\{([^}]+)\}", text)
    if m:
        return re.sub(r"\s+", "", m.group(1).strip()).lower()
    nums = re.findall(r"-?\d+\.?\d*", text)
    return nums[-1].lower() if nums else None

def load_model_with_adapter(model_id, lora_path, max_seq=6144):
    from unsloth import FastLanguageModel
    model, tokenizer = FastLanguageModel.from_pretrained(
        model_name=model_id, max_seq_length=max_seq,
        load_in_4bit=True,
    )
    FastLanguageModel.for_inference(model)
    if lora_path:
        from peft import PeftModel
        model = PeftModel.from_pretrained(model, lora_path)
        # Verify adapter is applied: check LoRA B weights are non-zero.
        # inference_mode=true freezes weights (requires_grad=False), so
        # checking trainable params is a false alarm — check weight values instead.
        lora_b_nonzero = sum((p != 0).sum().item() for n, p in model.named_parameters() if "lora_B" in n and "default" in n)
        lora_b_total = sum(p.numel() for n, p in model.named_parameters() if "lora_B" in n and "default" in n)
        print(f"  LoRA B weights: {lora_b_nonzero}/{lora_b_total} non-zero ({100*lora_b_nonzero/max(lora_b_total,1):.1f}%)", flush=True)
        if lora_b_nonzero == 0:
            raise RuntimeError("Adapter has all-zero LoRA B weights — adapter not applied")
    model.eval()
    return model, tokenizer

def generate(model, tokenizer, prompt, max_tokens=2048):
    # Qwen3.5 is a multimodal processor — must pass text= kwarg to avoid
    # the processor trying to parse the prompt string as an image.
    try:
        inputs = tokenizer(text=prompt, return_tensors="pt").to(model.device)
    except TypeError:
        inputs = tokenizer(prompt, return_tensors="pt").to(model.device)
    with torch.no_grad():
        out = model.generate(
            **inputs, max_new_tokens=max_tokens,
            temperature=1.0, do_sample=False,
            pad_token_id=tokenizer.eos_token_id,
        )
    return tokenizer.decode(out[0][inputs.input_ids.shape[1]:], skip_special_tokens=True)

def build_mcq_prompt(tokenizer, question, choices, labels):
    opts = "\n".join(f"{l}. {c}" for l, c in zip(labels, choices))
    user_msg = (
        f"Answer the following multiple choice question. "
        f"Respond with only the letter of the correct answer.\n\n"
        f"Question: {question}\n\n{opts}\n\n"
        f"Answer with just the letter."
    )
    msgs = [{"role": "user", "content": user_msg}]
    return tokenizer.apply_chat_template(msgs, tokenize=False, add_generation_prompt=True)

def run_gpqa(model, tokenizer, hf_token=""):
    print("\n=== GPQA Diamond ===", flush=True)
    try:
        ds = load_dataset("Idavidrein/gpqa", "gpqa_diamond", split="train", token=hf_token or None)
    except Exception as e:
        print(f"  Dataset load FAILED: {e}")
        return {"gpqa_diamond": None, "gpqa_n": 0}
    correct, total = 0, 0
    for ex in tqdm(ds, desc="GPQA"):
        choices = [ex["Correct Answer"], ex["Incorrect Answer 1"],
                   ex["Incorrect Answer 2"], ex["Incorrect Answer 3"]]
        random.shuffle(choices)
        labels = ["A", "B", "C", "D"]
        idx = {v: k for k, v in zip(labels, choices)}
        correct_label = idx[ex["Correct Answer"]]
        prompt = build_mcq_prompt(tokenizer, ex["Question"], choices, labels)
        response = generate(model, tokenizer, prompt, max_tokens=2048)
        pred = parse_mc_answer(response, 4)
        if pred == correct_label:
            correct += 1
        total += 1
    acc = correct / total if total else 0
    print(f"  Accuracy: {acc:.4f} ({correct}/{total})")
    return {"gpqa_diamond": acc, "gpqa_n": total}

def run_math500(model, tokenizer, n=100):
    print(f"\n=== MATH-500 ({n} samples) ===", flush=True)
    try:
        ds = load_dataset("HuggingFaceH4/MATH-500", split="test")
    except Exception as e:
        print(f"  Dataset load FAILED: {e}")
        return {"math_500": None, "math_n": 0}
    ds = ds.select(range(min(n, len(ds))))
    correct, total = 0, 0
    for ex in tqdm(ds, desc="MATH-500"):
        user_msg = (
            f"Solve the following math problem. "
            f"Put your final answer in \\boxed{{}}.\n\n"
            f"Problem: {ex['problem']}"
        )
        msgs = [{"role": "user", "content": user_msg}]
        prompt = tokenizer.apply_chat_template(msgs, tokenize=False, add_generation_prompt=True)
        response = generate(model, tokenizer, prompt, max_tokens=4096)
        pred = parse_math_answer(response)
        true_ans = re.sub(r"\s+", "", ex["answer"].strip()).lower()
        if pred is not None and pred == true_ans:
            correct += 1
        total += 1
    acc = correct / total if total else 0
    print(f"  Exact Match: {acc:.4f} ({correct}/{total})")
    return {"math_500": acc, "math_n": total}

def run_mmlu_pro(model, tokenizer, n=200):
    print(f"\n=== MMLU-Pro ({n} samples) ===", flush=True)
    try:
        ds = load_dataset("TIGER-Lab/MMLU-Pro", split="test")
    except Exception as e:
        print(f"  Dataset load FAILED: {e}")
        return {"mmlu_pro": None, "mmlu_n": 0}
    indices = random.sample(range(len(ds)), min(n, len(ds)))
    ds = ds.select(indices)
    correct, total = 0, 0
    n_opts = len(ds[0]["options"])
    labels = [chr(ord("A") + i) for i in range(min(n_opts, 10))]
    for ex in tqdm(ds, desc="MMLU-Pro"):
        true_label = labels[ex["answer_index"]]
        prompt = build_mcq_prompt(tokenizer, ex["question"], ex["options"], labels)
        response = generate(model, tokenizer, prompt, max_tokens=2048)
        pred = parse_mc_answer(response, n_opts)
        if pred == true_label:
            correct += 1
        total += 1
    acc = correct / total if total else 0
    print(f"  Accuracy: {acc:.4f} ({correct}/{total})")
    return {"mmlu_pro": acc, "mmlu_n": total}

def main():
    parser = argparse.ArgumentParser()
    parser.add_argument("--model", default="unsloth/Qwen3.6-27B")
    parser.add_argument("--lora", default=None)
    parser.add_argument("--base", action="store_true")
    parser.add_argument("--output-dir", default="./eval_results")
    parser.add_argument("--math-n", type=int, default=100)
    parser.add_argument("--mmlu-n", type=int, default=200)
    args = parser.parse_args()

    output_dir = Path(args.output_dir)
    output_dir.mkdir(parents=True, exist_ok=True)
    random.seed(42)
    timestamp = time.strftime("%Y%m%d-%H%M%S")

    hf_token = os.environ.get("HF_TOKEN", "")
    if not hf_token:
        try:
            with open("/proc/1/environ", "rb") as f:
                for entry in f.read().split(b"\0"):
                    if entry.startswith(b"HF_TOKEN="):
                        hf_token = entry.split(b"=", 1)[1].decode()
        except Exception:
            pass

    all_results = {}

    if args.lora:
        print(f"Loading adapter model: {args.model} + {args.lora}", flush=True)
        model, tok = load_model_with_adapter(args.model, args.lora)
        print(f"GPU: {torch.cuda.memory_allocated()/1e9:.1f}GB", flush=True)
        r = {"model": args.model, "lora": args.lora, "timestamp": timestamp}
        r.update(run_gpqa(model, tok, hf_token))
        r.update(run_math500(model, tok, args.math_n))
        r.update(run_mmlu_pro(model, tok, args.mmlu_n))
        all_results["adapter"] = r
        del model, tok
        torch.cuda.empty_cache()

    if args.base:
        print(f"\nLoading base model: {args.model}", flush=True)
        model, tok = load_model_with_adapter(args.model, lora_path="")
        print(f"GPU: {torch.cuda.memory_allocated()/1e9:.1f}GB", flush=True)
        r = {"model": args.model, "lora": None, "timestamp": timestamp}
        r.update(run_gpqa(model, tok, hf_token))
        r.update(run_math500(model, tok, args.math_n))
        r.update(run_mmlu_pro(model, tok, args.mmlu_n))
        all_results["base"] = r

    outfile = output_dir / f"eval_{timestamp}.json"
    with open(outfile, "w") as f:
        json.dump(all_results, f, indent=2)
    print(f"\nSaved to {outfile}")
    for label, r in all_results.items():
        print(f"\n=== {label.upper()} ===")
        for bench in ["gpqa_diamond", "math_500", "mmlu_pro"]:
            v = r.get(bench)
            n_key = bench.replace("diamond", "n").replace("_500", "_n").replace("_pro", "_n")
            if v is not None:
                print(f"  {bench}: {v:.4f} (n={r.get(n_key, '?')})")
            else:
                print(f"  {bench}: FAILED")
    if "adapter" in all_results and "base" in all_results:
        print("\n=== DELTA (adapter - base) ===")
        for bench in ["gpqa_diamond", "math_500", "mmlu_pro"]:
            a = all_results["adapter"].get(bench)
            b = all_results["base"].get(bench)
            if a is not None and b is not None:
                print(f"  {bench}: {a - b:+.4f}")

if __name__ == "__main__":
    main()
PYEOF

echo "" && echo "=== RUNNING EVALUATION ==="
python3 /workspace/eval_qwen36.py \
    --model unsloth/Qwen3.6-27B \
    --lora "${ADAPTER_DIR}" \
    --base \
    --output-dir /workspace/eval_results \
    --math-n 100 \
    --mmlu-n 200

echo "" && echo "=== EVAL COMPLETE: $(date -u) ==="
ls -lh /workspace/eval_results/
