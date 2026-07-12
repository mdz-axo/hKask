#!/bin/bash
# ============================================================================
# Rust Datasets Linked Dataset Builder (Sidecar Metadata)
# ============================================================================
# Processes Strandset-Rust-v1 and introspector/rust-analyser into clean
# training data + sidecar metadata with PKO + Dublin Core + 5W1H annotations.
#
# Produces:
#   strandset-train.jsonl      — clean ChatML training data
#   strandset-metadata.jsonl   — ontology sidecar with extracted PKO steps
#   introspector-train.jsonl   — clean ChatML training data
#   introspector-metadata.jsonl — ontology sidecar with extracted PKO steps
#
# Usage: bash scripts/build_rust_linked.sh
# ============================================================================
set -euo pipefail

HF_TOKEN=$(grep '^HF_TOKEN=' .env | cut -d= -f2-)
export HF_TOKEN
OUTPUT_DIR="/tmp/rust-linked"
HF_REPO="Axolotl-Partners/rust-datasets-linked"

mkdir -p "$OUTPUT_DIR"

python3 << 'PYEOF'
import json, os, sys, re, ast, time
from datasets import load_dataset
from pathlib import Path

HF_TOKEN = os.environ.get("HF_TOKEN", "")
OUTPUT_DIR = Path(os.environ.get("OUTPUT_DIR", "/tmp/rust-linked"))
OUTPUT_DIR.mkdir(parents=True, exist_ok=True)

# ── Shared ontology helpers ───────────────────────────────────────────────

DOMAIN_TO_SUBJECT = {
    "code_generation": "code generation",
    "bug_detection": "bug detection",
    "code_review": "code review",
    "docstring_generation": "documentation",
    "comment_generation": "commenting",
    "code_summarization": "summarization",
    "code_explanation": "explanation",
    "function_naming": "naming",
    "variable_naming": "naming",
    "code_completion": "completion",
    "code_refactoring": "refactoring",
    "code_optimization": "optimization",
    "code_search": "search",
    "test_generation": "testing",
    "api_usage_prediction": "API prediction",
    "name_resolution": "symbol resolution",
    "type_inference": "type inference",
    "parsing": "parsing",
}

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

def extract_code_blocks(text):
    """Extract code blocks from markdown-formatted text."""
    blocks = re.findall(r"```(?:rust)?\s*\n(.*?)```", text, re.DOTALL)
    return [b.strip() for b in blocks]

def classify_rust_step(text, category):
    """Classify what kind of Rust development step this is."""
    text_lower = text.lower()
    if "bug" in text_lower or "fix" in text_lower or "error" in text_lower:
        return "pko:IssueOccurrence", "bug_detection"
    if "review" in text_lower or "improve" in text_lower or "suggest" in text_lower:
        return "pko:StepVerification", "review"
    if "refactor" in text_lower or "readab" in text_lower:
        return "pko:Step", "refactor"
    if "optim" in text_lower or "performance" in text_lower or "faster" in text_lower:
        return "pko:Step", "optimize"
    if "test" in text_lower or "assert" in text_lower:
        return "pko:StepVerification", "test"
    if "doc" in text_lower or "comment" in text_lower:
        return "pko:Step", "document"
    if "name" in text_lower or "placeholder" in text_lower:
        return "pko:Step", "naming"
    if "complete" in text_lower or "fill" in text_lower or "prefix" in text_lower:
        return "pko:Step", "complete"
    if "search" in text_lower or "find" in text_lower or "query" in text_lower:
        return "pko:Action", "search"
    return "pko:Step", "generate"

# ── Strandset-Rust-v1 formatter ───────────────────────────────────────────

SYSTEM_CODING = "You are a Rust programming expert. Provide idiomatic, correct, and well-structured Rust code."

def format_strandset_train(ex):
    """Convert a Strandset record to clean ChatML (no ontology noise)."""
    cat = ex.get("task_category", "")
    inp = parse_field(ex.get("input_data"))
    out = parse_field(ex.get("output_data"))

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
        user = json.dumps(inp, indent=2)
        assistant = json.dumps(out, indent=2)

    return {"messages": [
        {"role": "system", "content": SYSTEM_CODING},
        {"role": "user", "content": user},
        {"role": "assistant", "content": assistant},
    ]}


def build_strandset_metadata(ex, idx):
    """Build ontology metadata for a Strandset example."""
    cat = ex.get("task_category", "")
    crate = ex.get("crate_name", "")
    inp = parse_field(ex.get("input_data"))
    out = parse_field(ex.get("output_data"))

    # Extract code blocks from the output for analysis
    output_text = json.dumps(out)
    code_blocks = extract_code_blocks(output_text)

    # Classify the step type
    pko_concept, step_type = classify_rust_step(cat, cat)

    # Build a title from the input
    title = ""
    if cat == "code_generation":
        title = inp.get("title", "")[:200]
    elif cat == "bug_detection":
        title = f"Bug fix in {crate}"[:200]
    elif cat == "code_review":
        title = f"Code review of {crate}"[:200]
    else:
        title = f"{cat} for {crate}"[:200]

    return {
        "id": f"strandset-rust-v1#{idx:06d}",
        "dublin_core": {
            "dcterms:title": title,
            "dcterms:creator": "Fortytwo Swarm Inference",
            "dcterms:source": "Fortytwo-Network/Strandset-Rust-v1",
            "dcterms:subject": DOMAIN_TO_SUBJECT.get(cat, cat),
            "dcterms:type": "dcterms:Dataset",
            "dcterms:identifier": f"strandset-rust-v1#{idx:06d}",
            "dcterms:rights": "Apache-2.0",
            "dcterms:description": f"Rust {cat} example from crate {crate}",
        },
        "pko": {
            "pko:Procedure": cat,
            "pko:hasStep": [{
                "step": 1,
                "pko_concept": pko_concept,
                "type": step_type,
                "category": cat,
                "crate": crate,
                "code_blocks_in_output": len(code_blocks),
            }],
            "pko:Agent": "Fortytwo Swarm Inference",
            "pko:references": f"crates.io/{crate}",
            "pko:StepVerification": {
                "compilation_verified": True,  # Strandset verifies with rustc
                "consensus_score": ">=0.7",
            },
        },
        "five_w1h": {
            "who": "Fortytwo Swarm Inference",
            "what": f"Rust {cat}",
            "domain": cat,
            "source": "Strandset-Rust-v1",
            "crate": crate,
            "why": "to teach Rust programming patterns",
            "how": "multi-model generation with peer-review consensus and rustc verification",
        },
    }


# ── introspector/rust-analyser formatter ──────────────────────────────────

SYSTEM_ANALYSIS = "You are a Rust code analysis expert. Analyze code for symbols, types, and semantic structure as rust-analyzer would."

def format_introspector_train(ex):
    """Convert a rust-analyser record to clean ChatML."""
    phase = ex.get("phase", "unknown")
    snippet = ex.get("source_snippet", "")
    element_type = ex.get("element_type", "")
    element_name = ex.get("element_name", "") or ""
    element_sig = ex.get("element_signature", "") or ""
    symbol_data = parse_field(ex.get("symbol_data"))
    type_data = parse_field(ex.get("type_data"))
    syntax_data = parse_field(ex.get("syntax_data"))
    ctx_before = ex.get("context_before", "") or ""

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


def build_introspector_metadata(ex, idx):
    """Build ontology metadata for a rust-analyser example."""
    phase = ex.get("phase", "unknown")
    element_type = ex.get("element_type", "")
    element_name = ex.get("element_name", "") or ""
    file_path = ex.get("file_path", "")
    line = ex.get("line", 0)
    column = ex.get("column", 0)
    processing_time = ex.get("processing_time_ms", 0)
    rust_version = ex.get("rust_version", "")
    analyzer_version = ex.get("analyzer_version", "")

    # Extract crate name from file path
    crate = ""
    if file_path:
        parts = file_path.split("/")
        for p in parts:
            if p and not p.startswith(".") and "/" not in p:
                crate = p
                break

    # Map phase to PKO
    phase_to_pko = {
        "name_resolution": ("pko:Step", "symbol_resolution"),
        "type_inference": ("pko:Step", "type_inference"),
        "parsing": ("pko:Step", "parsing"),
    }
    pko_concept, step_type = phase_to_pko.get(phase, ("pko:Step", phase))

    return {
        "id": f"rust-analyser#{idx:06d}",
        "dublin_core": {
            "dcterms:title": f"{element_type} {element_name}"[:200] if element_name else f"{element_type} at line {line}"[:200],
            "dcterms:creator": "rust-analyzer",
            "dcterms:source": "introspector/rust-analyser",
            "dcterms:subject": DOMAIN_TO_SUBJECT.get(phase, phase),
            "dcterms:type": "dcterms:Dataset",
            "dcterms:identifier": f"rust-analyser#{idx:06d}",
            "dcterms:rights": "AGPL-3.0",
            "dcterms:description": f"rust-analyzer {phase} of {element_type} in {crate or 'unknown crate'}",
        },
        "pko": {
            "pko:Procedure": f"semantic_analysis:{phase}",
            "pko:hasStep": [{
                "step": 1,
                "pko_concept": pko_concept,
                "type": step_type,
                "phase": phase,
                "element_type": element_type,
                "element_name": element_name,
                "file_path": file_path,
                "line": line,
                "column": column,
                "processing_time_ms": processing_time,
            }],
            "pko:Agent": "rust-analyzer",
            "pko:references": f"rust-analyzer/{crate}" if crate else "rust-analyzer",
            "pko:StepVerification": {
                "rust_version": rust_version,
                "analyzer_version": analyzer_version,
            },
        },
        "five_w1h": {
            "who": "rust-analyzer",
            "what": f"{phase} of {element_type}",
            "domain": phase,
            "source": "introspector/rust-analyser",
            "crate": crate,
            "file": file_path,
            "line": line,
            "why": "to teach semantic code analysis",
            "how": "rust-analyzer processes its own codebase and records analysis events",
        },
    }


# ── Process datasets ───────────────────────────────────────────────────────

def process_dataset(ds, train_formatter, meta_builder, name, train_path, meta_path):
    """Process a dataset into clean train + sidecar metadata."""
    print(f"\nProcessing {name} ({len(ds)} examples)...", flush=True)
    written = 0
    skipped = 0

    with open(train_path, "w") as tf, open(meta_path, "w") as mf:
        for i, ex in enumerate(ds):
            # Build clean training data
            train_ex = train_formatter(ex)
            msgs = train_ex.get("messages", [])
            if len(msgs) < 3 or not all(m.get("content", "").strip() for m in msgs):
                skipped += 1
                continue

            tf.write(json.dumps(train_ex) + "\n")

            # Build sidecar metadata
            metadata = meta_builder(ex, i)
            mf.write(json.dumps(metadata) + "\n")

            written += 1
            if (i + 1) % 20000 == 0:
                print(f"  Processed {i+1}/{len(ds)} ({written} written, {skipped} skipped)", flush=True)

    print(f"  Done: {written} written, {skipped} skipped", flush=True)
    print(f"  Train: {train_path} ({train_path.stat().st_size / 1e6:.1f} MB)", flush=True)
    print(f"  Meta:  {meta_path} ({meta_path.stat().st_size / 1e6:.1f} MB)", flush=True)
    return written


# ── Load and process Strandset-Rust-v1 ────────────────────────────────────

print("Loading Strandset-Rust-v1...", flush=True)
strandset = load_dataset("Fortytwo-Network/Strandset-Rust-v1", split="train")
strand_train = OUTPUT_DIR / "strandset-train.jsonl"
strand_meta = OUTPUT_DIR / "strandset-metadata.jsonl"
strand_count = process_dataset(
    strandset, format_strandset_train, build_strandset_metadata,
    "Strandset-Rust-v1", strand_train, strand_meta
)

# ── Load and process introspector/rust-analyser ──────────────────────────

print("\nLoading introspector/rust-analyser...", flush=True)
introspector = load_dataset("introspector/rust-analyser", split="train")
intro_train = OUTPUT_DIR / "introspector-train.jsonl"
intro_meta = OUTPUT_DIR / "introspector-metadata.jsonl"
intro_count = process_dataset(
    introspector, format_introspector_train, build_introspector_metadata,
    "introspector/rust-analyser", intro_train, intro_meta
)

# ── Summary ────────────────────────────────────────────────────────────────

print(f"\n=== SUMMARY ===", flush=True)
print(f"Strandset:    {strand_count} examples", flush=True)
print(f"Introspector: {intro_count} examples", flush=True)
print(f"Total:        {strand_count + intro_count} examples", flush=True)

# ── Upload to HuggingFace ─────────────────────────────────────────────────

if HF_TOKEN:
    repo_id = os.environ.get("HF_REPO", "Axolotl-Partners/rust-datasets-linked")
    print(f"\nUploading to {repo_id}...", flush=True)
    from huggingface_hub import HfApi
    api = HfApi(token=HF_TOKEN)
    api.create_repo(repo_id=repo_id, repo_type="dataset", exist_ok=True)

    for fname in ["strandset-train.jsonl", "strandset-metadata.jsonl",
                  "introspector-train.jsonl", "introspector-metadata.jsonl"]:
        fpath = OUTPUT_DIR / fname
        print(f"  Uploading {fname} ({fpath.stat().st_size / 1e6:.1f} MB)...", flush=True)
        api.upload_file(
            path_or_fileobj=str(fpath),
            path_in_repo=fname,
            repo_id=repo_id,
            repo_type="dataset",
            commit_message=f"Rust datasets with sidecar ontology metadata (PKO + Dublin Core + 5W1H)",
        )
        print(f"    Done", flush=True)
    print(f"\nUploaded to {repo_id}", flush=True)
else:
    print("No HF_TOKEN - files saved locally only.", flush=True)
PYEOF

echo "=== DONE ==="
ls -lh "$OUTPUT_DIR/"
