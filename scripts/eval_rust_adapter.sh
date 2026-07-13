#!/bin/bash
# ============================================================================
# Rust Adapter Evaluation — Strandset-Rust-v1 test split
# ============================================================================
# Image: runpod/pytorch:2.4.0-py3.11-cuda12.4.1-devel-ubuntu22.04
# Usage: curl -sL <this-script-url> | bash
#
# Evaluates a Rust coding adapter against the Strandset-Rust-v1 test split
# (225 examples, 15 per category). Runs baseline + adapter comparison.
#
# Env vars:
#   ADAPTER_DIR  — path to adapter (default: /workspace/adapter)
#   MODEL_ID     — base model (default: unsloth/Qwen3.6-27B)
# ============================================================================
set -euo pipefail
exec > /workspace/rust_eval.log 2>&1

if [ -r /proc/1/environ ]; then
    while IFS= read -r -d '' entry; do
        case "$entry" in HF_TOKEN=*) export "${entry?}" ;; esac
    done < /proc/1/environ
fi

echo "============================================"
echo "Rust Adapter Evaluation | $(date -u)"
echo "============================================"

export HF_HOME=/workspace/.cache/huggingface
export PIP_CACHE_DIR=/workspace/.cache/pip
mkdir -p "$HF_HOME" "$PIP_CACHE_DIR"

echo "=== INSTALLING DEPS ==="
pip install --cache-dir "$PIP_CACHE_DIR" -q unsloth unsloth_zoo 2>&1 | tail -3
pip install --cache-dir "$PIP_CACHE_DIR" -q datasets peft tqdm 2>&1 | tail -3

echo "=== INSTALLING QWEN KERNELS ==="
pip install --cache-dir "$PIP_CACHE_DIR" -q "flash-linear-attention[cuda]" tilelang 2>&1 | tail -3
# causal-conv1d needs --no-build-isolation to use the installed PyTorch's CUDA
# headers instead of the system CUDA toolkit (which may mismatch).
pip install --cache-dir "$PIP_CACHE_DIR" -q causal-conv1d --no-build-isolation 2>&1 | tail -3

cat > /workspace/eval_rust.py << 'PYEOF'
import os, sys, json, re, time, ast
import torch
from datasets import load_dataset
from tqdm import tqdm
from unsloth import FastLanguageModel
from peft import PeftModel

MODEL_ID = os.environ.get("MODEL_ID", "unsloth/Qwen3.6-27B")
ADAPTER_DIR = os.environ.get("ADAPTER_DIR", "/workspace/adapter")
MAX_SEQ = 6144
MAX_NEW_TOKENS = 2048

SYSTEM = "You are a Rust programming expert. Provide idiomatic, correct, and well-structured Rust code."

def parse_field(val):
    """Parse input_data (Python dict repr) or output_data (JSON)."""
    if val is None:
        return {}
    if isinstance(val, dict):
        return val
    if isinstance(val, str):
        try:
            return json.loads(val)
        except (json.JSONDecodeError, TypeError):
            pass
        try:
            return ast.literal_eval(val)
        except (ValueError, SyntaxError):
            return {}
    return {}

def build_prompt(tokenizer, cat, inp):
    """Build the same prompt as the training script for each category."""
    if cat == "code_generation":
        title = inp.get("title", "")
        desc = inp.get("description", "")
        ctx = inp.get("code_context", "")
        user = f"Generate Rust code for the following task.\n\nTitle: {title}\nDescription: {desc}"
        if ctx:
            user += f"\n\nContext:\n```rust\n{ctx}\n```"
    elif cat == "bug_detection":
        buggy = inp.get("buggy_code", "")
        ctx = inp.get("code_context", "")
        user = f"Find and fix the bug in this Rust code.\n\n"
        if ctx:
            user += f"Context:\n```rust\n{ctx}\n```\n\n"
        user += f"Buggy code:\n```rust\n{buggy}\n```"
    elif cat == "code_review":
        before = inp.get("code_before", "")
        ctx = inp.get("code_context", "")
        user = f"Review this Rust code and suggest improvements.\n\n"
        if ctx:
            user += f"Context:\n```rust\n{ctx}\n```\n\n"
        user += f"Code:\n```rust\n{before}\n```"
    elif cat == "docstring_generation":
        code = inp.get("code", "")
        user = f"Generate a Rust docstring for this code.\n\n```rust\n{code}\n```"
    elif cat == "comment_generation":
        code = inp.get("code", "")
        user = f"Add meaningful inline comments to this Rust code.\n\n```rust\n{code}\n```"
    elif cat == "code_summarization":
        code = inp.get("code", "")
        user = f"Summarize what this Rust code does.\n\n```rust\n{code}\n```"
    elif cat == "code_explanation":
        code = inp.get("code", "")
        user = f"Explain this Rust code.\n\n```rust\n{code}\n```"
    elif cat == "function_naming":
        code = inp.get("code", "")
        user = f"Suggest an idiomatic Rust function name for the placeholder in this code.\n\n```rust\n{code}\n```"
    elif cat == "variable_naming":
        code = inp.get("code", "")
        user = f"Suggest an idiomatic Rust variable name for the placeholder in this code.\n\n```rust\n{code}\n```"
    elif cat == "code_completion":
        prefix = inp.get("prefix", "")
        suffix = inp.get("suffix", "")
        user = f"Complete this Rust code. Fill in the missing section between the prefix and suffix.\n\n"
        user += f"Prefix:\n```rust\n{prefix}\n```\n\n"
        user += f"Suffix:\n```rust\n{suffix}\n```"
    elif cat == "code_refactoring":
        before = inp.get("code_before", "")
        user = f"Refactor this Rust code to improve readability while preserving logic.\n\n```rust\n{before}\n```"
    elif cat == "code_optimization":
        before = inp.get("code_before", "")
        user = f"Optimize this Rust code.\n\n```rust\n{before}\n```"
    elif cat == "code_search":
        query = inp.get("query", "")
        ctx = inp.get("code_context", "")
        user = f"Find Rust code relevant to this query: {query}"
        if ctx:
            user += f"\n\nContext:\n```rust\n{ctx}\n```"
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
    elif cat == "api_usage_prediction":
        code = inp.get("code", "")
        ctx = inp.get("code_context", "")
        user = f"Predict the next API call or usage pattern in this Rust context.\n\n"
        if ctx:
            user += f"Context:\n```rust\n{ctx}\n```\n\n"
        user += f"Code:\n```rust\n{code}\n```"
    else:
        user = json.dumps(inp, indent=2)

    msgs = [{"role": "system", "content": SYSTEM}, {"role": "user", "content": user}]
    return tokenizer.apply_chat_template(msgs, tokenize=False, add_generation_prompt=True)

def extract_code(text):
    """Extract code from a ```rust ... ``` block, or return the text."""
    m = re.search(r"```rust\s*\n(.*?)```", text, re.DOTALL)
    if m:
        return m.group(1).strip()
    m = re.search(r"```\s*\n(.*?)```", text, re.DOTALL)
    if m:
        return m.group(1).strip()
    return text.strip()

def extract_expected(cat, out):
    """Extract the expected answer from output_data based on category."""
    if cat == "code_generation":
        return out.get("code", "")
    elif cat == "bug_detection":
        return out.get("fixed_code", "")
    elif cat == "code_review":
        return out.get("code_after", "")
    elif cat == "docstring_generation":
        return out.get("docstring", "")
    elif cat == "comment_generation":
        return out.get("commented_code", "")
    elif cat == "code_summarization":
        return out.get("summary", "")
    elif cat == "code_explanation":
        return out.get("explanation", "")
    elif cat == "function_naming":
        return out.get("function_name", "")
    elif cat == "variable_naming":
        return out.get("variable_name", "")
    elif cat == "code_completion":
        return out.get("completion", "")
    elif cat == "code_refactoring":
        return out.get("code_after", "")
    elif cat == "code_optimization":
        return out.get("code_after", "")
    elif cat == "code_search":
        return out.get("code_snippet", "")
    elif cat == "test_generation":
        return out.get("test_cases", "")
    elif cat == "api_usage_prediction":
        return out.get("next_api_call", "")
    return ""

def score_response(cat, generated, expected):
    """Score the response. Returns (is_correct, metric_name, detail)."""
    if not expected:
        return False, "no_expected", "no expected answer"

    if cat in ("function_naming", "variable_naming"):
        # Exact match on the name (case-insensitive)
        gen_clean = generated.strip().split("\n")[0].strip().strip("`").strip()
        exp_clean = expected.strip()
        return gen_clean.lower() == exp_clean.lower(), "exact_match", f"gen={gen_clean!r} exp={exp_clean!r}"

    if cat in ("code_summarization", "code_explanation", "docstring_generation"):
        # Contains match — does the generated text contain key phrases from expected?
        gen_lower = generated.lower()
        exp_lower = expected.lower()
        # Check if >50% of expected words appear in generated
        exp_words = set(exp_lower.split())
        gen_words = set(gen_lower.split())
        if not exp_words:
            return False, "empty_expected", ""
        overlap = len(exp_words & gen_words) / len(exp_words)
        return overlap >= 0.3, "word_overlap", f"overlap={overlap:.2f}"

    if cat in ("code_generation", "bug_detection", "code_review", "code_completion",
               "code_refactoring", "code_optimization", "code_search", "test_generation",
               "comment_generation", "api_usage_prediction"):
        # Code comparison — extract code blocks and compare
        gen_code = extract_code(generated)
        exp_code = expected.strip()
        # Normalize whitespace
        gen_norm = re.sub(r"\s+", " ", gen_code).strip()
        exp_norm = re.sub(r"\s+", " ", exp_code).strip()
        if not gen_norm or not exp_norm:
            return False, "empty_code", "generated or expected code is empty"
        # Exact match after normalization
        if gen_norm == exp_norm:
            return True, "exact_code", ""
        # Token overlap (rough similarity)
        gen_tokens = set(gen_norm.split())
        exp_tokens = set(exp_norm.split())
        if not exp_tokens:
            return False, "empty_tokens", ""
        overlap = len(gen_tokens & exp_tokens) / len(exp_tokens)
        return overlap >= 0.5, "token_overlap", f"overlap={overlap:.2f}"

    return False, "unknown_category", ""

def generate(model, tokenizer, prompt, max_new_tokens=MAX_NEW_TOKENS):
    try:
        inputs = tokenizer(text=prompt, return_tensors="pt").to(model.device)
    except TypeError:
        inputs = tokenizer(prompt, return_tensors="pt").to(model.device)
    with torch.no_grad():
        out = model.generate(
            **inputs, max_new_tokens=max_new_tokens,
            do_sample=False, temperature=1.0,
            pad_token_id=tokenizer.eos_token_id,
        )
    return tokenizer.decode(out[0][inputs.input_ids.shape[1]:], skip_special_tokens=True)

def run_eval(model, tokenizer, label):
    """Run eval on the Strandset test split."""
    print(f"\n=== {label} ===", flush=True)
    ds = load_dataset("Fortytwo-Network/Strandset-Rust-v1", split="test")
    print(f"  Test split: {len(ds)} examples", flush=True)

    results_by_cat = {}
    all_results = []

    for ex in tqdm(ds, desc=label):
        cat = ex.get("task_category", "")
        inp = parse_field(ex.get("input_data"))
        out = parse_field(ex.get("output_data"))
        expected = extract_expected(cat, out)

        prompt = build_prompt(tokenizer, cat, inp)
        response = generate(model, tokenizer, prompt)
        is_correct, metric, detail = score_response(cat, response, expected)

        if cat not in results_by_cat:
            results_by_cat[cat] = {"correct": 0, "total": 0}
        results_by_cat[cat]["total"] += 1
        if is_correct:
            results_by_cat[cat]["correct"] += 1

        all_results.append({
            "category": cat,
            "crate": ex.get("crate_name", ""),
            "correct": is_correct,
            "metric": metric,
            "detail": detail,
        })

    # Print per-category results
    print(f"\n  --- Per-category accuracy ({label}) ---", flush=True)
    total_correct = 0
    total_total = 0
    for cat in sorted(results_by_cat.keys()):
        r = results_by_cat[cat]
        acc = r["correct"] / r["total"] if r["total"] > 0 else 0
        print(f"    {cat:25s}: {acc:.4f} ({r['correct']}/{r['total']})", flush=True)
        total_correct += r["correct"]
        total_total += r["total"]

    overall = total_correct / total_total if total_total > 0 else 0
    print(f"    {'OVERALL':25s}: {overall:.4f} ({total_correct}/{total_total})", flush=True)

    return {"overall": overall, "by_category": {k: v["correct"]/v["total"] for k, v in results_by_cat.items()}, "per_example": all_results}

# ── Main ──────────────────────────────────────────────────────────────────

print(f"Loading base model {MODEL_ID} via Unsloth...", flush=True)
model, tokenizer = FastLanguageModel.from_pretrained(
    model_name=MODEL_ID, max_seq_length=MAX_SEQ,
    load_in_4bit=True,
)
FastLanguageModel.for_inference(model)

# Baseline eval
baseline_results = run_eval(model, tokenizer, "BASELINE")

# Load adapter
print(f"\nLoading adapter from {ADAPTER_DIR}...", flush=True)
model = PeftModel.from_pretrained(model, ADAPTER_DIR)
model.eval()

# Verify adapter
lora_nonzero = sum((p != 0).sum().item() for n, p in model.named_parameters() if "lora_B" in n and "default" in n)
lora_total = sum(p.numel() for n, p in model.named_parameters() if "lora_B" in n and "default" in n)
print(f"  LoRA B weights: {lora_nonzero}/{lora_total} non-zero ({100*lora_nonzero/max(lora_total,1):.1f}%)", flush=True)
if lora_nonzero == 0:
    print("FATAL: adapter has all-zero LoRA B weights", flush=True)
    sys.exit(1)

# Adapter eval
adapter_results = run_eval(model, tokenizer, "ADAPTER")

# Summary
print("\n=== SUMMARY ===", flush=True)
print(f"  Baseline: {baseline_results['overall']:.4f}", flush=True)
print(f"  Adapter:  {adapter_results['overall']:.4f}", flush=True)
print(f"  Delta:    {adapter_results['overall'] - baseline_results['overall']:+.4f}", flush=True)

print("\n  --- Per-category delta ---", flush=True)
for cat in sorted(baseline_results["by_category"].keys()):
    b = baseline_results["by_category"].get(cat, 0)
    a = adapter_results["by_category"].get(cat, 0)
    print(f"    {cat:25s}: {a - b:+.4f} (base={b:.4f} adapter={a:.4f})", flush=True)

# Save
ts = time.strftime("%Y%m%d-%H%M%S")
outfile = f"/workspace/eval_results/rust_eval_{ts}.json"
os.makedirs("/workspace/eval_results", exist_ok=True)
with open(outfile, "w") as f:
    json.dump({"baseline": baseline_results, "adapter": adapter_results,
               "model": MODEL_ID, "adapter_dir": ADAPTER_DIR}, f, indent=2)
print(f"\nSaved to {outfile}", flush=True)
print("\nDONE", flush=True)
PYEOF

echo "" && echo "=== RUNNING RUST EVALUATION ==="
python3 /workspace/eval_rust.py

echo "" && echo "=== EVAL COMPLETE: $(date -u) ==="
ls -lh /workspace/eval_results/
