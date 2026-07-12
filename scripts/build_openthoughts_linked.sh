#!/bin/bash
# ============================================================================
# OpenThoughts-114k Linked Dataset Builder (Sidecar Metadata)
# ============================================================================
# Downloads OpenThoughts-114k and produces two files:
#
#   train.jsonl     — clean ChatML training data (original system + conversations)
#   metadata.jsonl  — ontology annotations sidecar (PKO + Dublin Core + 5W1H)
#                     with real reasoning steps extracted from the thought trace
#
# Line N in metadata.jsonl corresponds to line N in train.jsonl.
#
# Usage: bash scripts/build_openthoughts_linked.sh
# ============================================================================
set -euo pipefail

HF_TOKEN=$(grep '^HF_TOKEN=' .env | cut -d= -f2-)
export HF_TOKEN
OUTPUT_DIR="/tmp/openthoughts-linked"
HF_REPO="Axolotl-Partners/openthoughts-114k-linked"

mkdir -p "$OUTPUT_DIR"

python3 << 'PYEOF'
import json, os, sys, re, time
from datasets import load_dataset
from pathlib import Path

HF_TOKEN = os.environ.get("HF_TOKEN", "")
OUTPUT_DIR = Path(os.environ.get("OUTPUT_DIR", "/tmp/openthoughts-linked"))
OUTPUT_DIR.mkdir(parents=True, exist_ok=True)

# ── Thought section parser ────────────────────────────────────────────────
# Extracts reasoning steps from the <|begin_of_thought|> section by splitting
# on paragraph boundaries (\n\n). Each paragraph is a pko:Step.

THOUGHT_START = "<|begin_of_thought|>"
THOUGHT_END = "<|end_of_thought|>"
SOLUTION_START = "<|begin_of_solution|>"
SOLUTION_END = "<|end_of_solution|>"

# Heuristics for classifying what kind of reasoning step a paragraph represents.
# These map to PKO concepts.
STEP_PATTERNS = [
    # (regex, pko concept, step type label)
    (r"^(okay|let me|so i need|i need to|first|let's|alright)", "pko:Step", "understand"),
    (r"^(wait|but|however|actually|no,|that's wrong|let me reconsider)", "pko:Step", "reassess"),
    (r"^(so if|therefore|thus|hence|this means|which means)", "pko:Step", "infer"),
    (r"^(let me check|let me verify|to verify|checking|if.*correct)", "pko:StepVerification", "verify"),
    (r"^(now|next|then|after|step)", "pko:Step", "proceed"),
    (r"^(i remember|i know|from|according to|based on)", "pko:Step", "recall"),
    (r"^(what if|alternatively|another approach|maybe|perhaps)", "pko:Step", "explore"),
]

def classify_step(paragraph):
    """Classify a reasoning paragraph into a PKO step type."""
    first_words = paragraph[:200].lower()
    for pattern, pko_concept, label in STEP_PATTERNS:
        if re.match(pattern, first_words):
            return pko_concept, label
    return "pko:Step", "reason"

def extract_reasoning_steps(assistant_text):
    """Extract PKO steps from the thought section of an assistant response."""
    if THOUGHT_START not in assistant_text or THOUGHT_END not in assistant_text:
        return [], None

    thought_start = assistant_text.find(THOUGHT_START) + len(THOUGHT_START)
    thought_end = assistant_text.find(THOUGHT_END)
    thought = assistant_text[thought_start:thought_end].strip()

    # Solution section
    solution = None
    if SOLUTION_START in assistant_text and SOLUTION_END in assistant_text:
        sol_start = assistant_text.find(SOLUTION_START) + len(SOLUTION_START)
        sol_end = assistant_text.find(SOLUTION_END)
        solution = assistant_text[sol_start:sol_end].strip()

    # Split into paragraphs — each paragraph is a reasoning step
    paragraphs = [p.strip() for p in thought.split("\n\n") if p.strip()]

    steps = []
    for i, para in enumerate(paragraphs):
        pko_concept, step_type = classify_step(para)
        steps.append({
            "step": i + 1,
            "pko_concept": pko_concept,
            "type": step_type,
            "length": len(para),
            "preview": para[:200],
        })

    return steps, solution

# ── Domain → PKO action mapping ───────────────────────────────────────────

DOMAIN_TO_SUBJECT = {
    "math": "mathematics",
    "science": "natural science",
    "code": "computer science",
    "puzzle": "logic puzzles",
}

# ── Build metadata for a single example ────────────────────────────────────

def build_metadata(ex, meta, idx):
    """Build structured ontology metadata for one example."""
    domain = meta.get("domain", "unknown") if meta else "unknown"
    source = meta.get("source", "open-thoughts") if meta else "open-thoughts"
    problem = meta.get("problem", "") if meta else ""
    has_ground_truth = bool(meta.get("ground_truth_solution")) if meta else False
    has_test_cases = bool(meta.get("test_cases")) if meta else False

    # Extract reasoning steps from the assistant response
    convs = ex.get("conversations", [])
    assistant_text = ""
    for c in convs:
        if c.get("from") == "assistant":
            assistant_text = c.get("value", "")
            break

    reasoning_steps, solution = extract_reasoning_steps(assistant_text)

    # Count step types for summary
    step_types = {}
    for s in reasoning_steps:
        step_types[s["type"]] = step_types.get(s["type"], 0) + 1

    # Build the metadata object
    return {
        "id": f"openthoughts-114k#{idx:06d}",
        "dublin_core": {
            "dcterms:title": problem[:200] if problem else "",
            "dcterms:creator": "DeepSeek-R1",
            "dcterms:source": source,
            "dcterms:subject": DOMAIN_TO_SUBJECT.get(domain, domain),
            "dcterms:type": "dcterms:Dataset",
            "dcterms:identifier": f"openthoughts-114k#{idx:06d}",
            "dcterms:rights": "Apache-2.0",
            "dcterms:description": f"Reasoning trace for {domain} problem from {source}",
        },
        "pko": {
            "pko:Procedure": "reasoning",
            "pko:hasStep": reasoning_steps,
            "pko:ProcedureExecution": {
                "total_steps": len(reasoning_steps),
                "step_type_distribution": step_types,
                "has_solution": solution is not None,
                "solution_length": len(solution) if solution else 0,
            },
            "pko:Agent": "DeepSeek-R1",
            "pko:StepVerification": {
                "has_ground_truth": has_ground_truth,
                "has_test_cases": has_test_cases,
            },
            "pko:references": source,
        },
        "five_w1h": {
            "who": "DeepSeek-R1",
            "what": f"{domain} reasoning trace",
            "domain": domain,
            "source": source,
            "why": "to teach systematic reasoning to student models",
            "how": "DeepSeek-R1 generates reasoning traces, verified against ground truth",
        },
    }

# ── Load datasets ──────────────────────────────────────────────────────────

print("Loading OpenThoughts-114k...", flush=True)
ds = load_dataset("open-thoughts/OpenThoughts-114k", split="train")
ds_meta = load_dataset("open-thoughts/OpenThoughts-114k", "metadata", split="train")
print(f"  Loaded {len(ds)} examples + {len(ds_meta)} metadata rows", flush=True)

# ── Write clean training data + sidecar metadata ──────────────────────────

train_file = OUTPUT_DIR / "train.jsonl"
meta_file = OUTPUT_DIR / "metadata.jsonl"

print(f"Writing clean training data to {train_file}...", flush=True)
print(f"Writing ontology metadata to {meta_file}...", flush=True)

written = 0
skipped = 0
step_counts = []

with open(train_file, "w") as tf, open(meta_file, "w") as mf:
    for i, (ex, meta) in enumerate(zip(ds, ds_meta)):
        convs = ex.get("conversations", [])
        if len(convs) < 2:
            skipped += 1
            continue

        # Clean training data: original system + conversations as ChatML
        messages = []
        system = ex.get("system", "")
        if system:
            messages.append({"role": "system", "content": system})
        for c in convs:
            role = "user" if c.get("from") == "user" else "assistant"
            messages.append({"role": role, "content": c.get("value", "")})

        if len(messages) < 3:
            skipped += 1
            continue

        tf.write(json.dumps({"messages": messages}) + "\n")

        # Sidecar metadata with extracted reasoning steps
        metadata = build_metadata(ex, meta, i)
        mf.write(json.dumps(metadata) + "\n")

        step_counts.append(len(metadata["pko"]["pko:hasStep"]))
        written += 1

        if (i + 1) % 10000 == 0:
            avg_steps = sum(step_counts) / len(step_counts) if step_counts else 0
            print(f"  Processed {i+1}/{len(ds)} ({written} written, {skipped} skipped, avg {avg_steps:.1f} steps/example)", flush=True)

avg_steps = sum(step_counts) / len(step_counts) if step_counts else 0
print(f"\nDone: {written} examples written, {skipped} skipped", flush=True)
print(f"Average reasoning steps per example: {avg_steps:.1f}", flush=True)
print(f"Training data: {train_file} ({train_file.stat().st_size / 1e6:.1f} MB)", flush=True)
print(f"Metadata: {meta_file} ({meta_file.stat().st_size / 1e6:.1f} MB)", flush=True)

# ── Verify uniqueness of metadata ──────────────────────────────────────────

print("\nVerifying metadata uniqueness...", flush=True)
unique_meta = set()
with open(meta_file) as f:
    for line in f:
        m = json.loads(line)
        # The step distribution is the discriminative part
        unique_meta.add(json.dumps(m["pko"]["pko:ProcedureExecution"]["step_type_distribution"], sort_keys=True))

print(f"Unique step type distributions: {len(unique_meta)} / {written} ({100*len(unique_meta)/max(written,1):.1f}%)", flush=True)

# ── Upload to HuggingFace ─────────────────────────────────────────────────

if HF_TOKEN:
    repo_id = os.environ.get("HF_REPO", "Axolotl-Partners/openthoughts-114k-linked")
    print(f"\nUploading to {repo_id}...", flush=True)
    from huggingface_hub import HfApi
    api = HfApi(token=HF_TOKEN)
    api.create_repo(repo_id=repo_id, repo_type="dataset", exist_ok=True)

    # Upload both files in one commit
    api.upload_file(
        path_or_fileobj=str(train_file),
        path_in_repo="train.jsonl",
        repo_id=repo_id,
        repo_type="dataset",
        commit_message=f"Clean training data ({written} examples)",
    )
    api.upload_file(
        path_or_fileobj=str(meta_file),
        path_in_repo="metadata.jsonl",
        repo_id=repo_id,
        repo_type="dataset",
        commit_message=f"Sidecar metadata with extracted PKO reasoning steps ({written} examples, avg {avg_steps:.1f} steps/example)",
    )
    print(f"Uploaded to {repo_id}", flush=True)
else:
    print("No HF_TOKEN — files saved locally only.", flush=True)
PYEOF

echo "=== DONE ==="
ls -lh "$OUTPUT_DIR/"
