#!/usr/bin/env python3
"""Preprocess raw Rust datasets to ChatML JSONL for Axolotl."""
import json, ast, sys
from datasets import load_dataset

SYSTEM_CODING = "You are a Rust programming expert. Provide idiomatic, correct, and well-structured Rust code."
SYSTEM_ANALYSIS = "You are a Rust code analysis expert. Analyze code for symbols, types, and semantic structure as rust-analyzer would."

def parse_json_field(val):
    if val is None: return {}
    if isinstance(val, dict): return val
    if isinstance(val, str):
        try: return json.loads(val)
        except: pass
        try: return ast.literal_eval(val)
        except: return {"_raw": val}
    return {}

def format_strandset(example):
    cat = example.get("task_category", "")
    inp = parse_json_field(example.get("input_data"))
    out = parse_json_field(example.get("output_data"))
    if cat == "code_generation":
        ctx = inp.get("code_context", "")
        user = f"Generate Rust code for the following task.\n\nTitle: {inp.get('title','')}\nDescription: {inp.get('description','')}"
        if ctx: user += f"\n\nContext:\n```rust\n{ctx}\n```"
        assistant = f"```rust\n{out.get('code', '')}\n```"
    elif cat == "bug_detection":
        ctx = inp.get("code_context", "")
        user = f"Find and fix the bug in this Rust code.\n\n"
        if ctx: user += f"Context:\n```rust\n{ctx}\n```\n\n"
        user += f"Buggy code:\n```rust\n{inp.get('buggy_code','')}\n```"
        assistant = f"**Bug:** {out.get('bug_description','')}\n\n**Fixed code:**\n```rust\n{out.get('fixed_code','')}\n```"
    elif cat == "code_review":
        ctx = inp.get("code_context", "")
        user = f"Review this Rust code and suggest improvements.\n\n"
        if ctx: user += f"Context:\n```rust\n{ctx}\n```\n\n"
        user += f"Code:\n```rust\n{inp.get('code_before','')}\n```"
        assistant = f"**Review:** {out.get('review_comment','')}\n\n**Improved code:**\n```rust\n{out.get('code_after','')}\n```"
    elif cat == "docstring_generation":
        user = f"Generate a Rust docstring for this code.\n\n```rust\n{inp.get('code','')}\n```"
        assistant = out.get("docstring", "")
    elif cat == "comment_generation":
        user = f"Add meaningful inline comments to this Rust code.\n\n```rust\n{inp.get('code','')}\n```"
        assistant = f"```rust\n{out.get('commented_code', '')}\n```"
    elif cat == "code_summarization":
        user = f"Summarize what this Rust code does.\n\n```rust\n{inp.get('code','')}\n```"
        assistant = out.get("summary", "")
    elif cat == "code_explanation":
        user = f"Explain this Rust code.\n\n```rust\n{inp.get('code','')}\n```"
        assistant = out.get("explanation", "")
    elif cat == "function_naming":
        user = f"Suggest an idiomatic Rust function name for the placeholder in this code.\n\n```rust\n{inp.get('code','')}\n```"
        assistant = out.get("function_name", "")
    elif cat == "variable_naming":
        user = f"Suggest an idiomatic Rust variable name for the placeholder in this code.\n\n```rust\n{inp.get('code','')}\n```"
        assistant = out.get("variable_name", "")
    elif cat == "code_completion":
        user = f"Complete this Rust code. Fill in the missing section between the prefix and suffix.\n\n"
        user += f"Prefix:\n```rust\n{inp.get('prefix','')}\n```\n\nSuffix:\n```rust\n{inp.get('suffix','')}\n```"
        assistant = f"```rust\n{out.get('completion', '')}\n```"
    elif cat == "code_refactoring":
        user = f"Refactor this Rust code to improve readability while preserving logic.\n\n```rust\n{inp.get('code_before','')}\n```"
        assistant = f"**Rationale:** {out.get('rationale','')}\n\n**Refactored code:**\n```rust\n{out.get('code_after','')}\n```"
    elif cat == "code_optimization":
        user = f"Optimize this Rust code.\n\n```rust\n{inp.get('code_before','')}\n```"
        assistant = f"**Rationale:** {out.get('rationale','')}\n\n**Optimized code:**\n```rust\n{out.get('code_after','')}\n```"
    elif cat == "code_search":
        ctx = inp.get("code_context", "")
        user = f"Find Rust code relevant to this query: {inp.get('query','')}"
        if ctx: user += f"\n\nContext:\n```rust\n{ctx}\n```"
        assistant = f"```rust\n{out.get('code_snippet', '')}\n```"
    elif cat == "test_generation":
        ctx = inp.get("code_context", "")
        user = f"Generate unit tests for this Rust code.\n\n"
        if ctx: user += f"Context:\n```rust\n{ctx}\n```\n\n"
        user += f"Code to test:\n```rust\n{inp.get('code_to_test','')}\n```"
        test_ctx = inp.get("test_context", "")
        if test_ctx: user += f"\n\nTest context:\n```rust\n{test_ctx}\n```"
        assistant = f"```rust\n{out.get('test_cases', '')}\n```"
    elif cat == "api_usage_prediction":
        ctx = inp.get("code_context", "")
        user = f"Predict the next API call or usage pattern in this Rust context.\n\n"
        if ctx: user += f"Context:\n```rust\n{ctx}\n```\n\n"
        user += f"Code:\n```rust\n{inp.get('code','')}\n```"
        assistant = out.get("next_api_call", "")
    else:
        user = json.dumps(inp, indent=2)
        assistant = json.dumps(out, indent=2)
    return {"messages": [
        {"role": "system", "content": SYSTEM_CODING},
        {"role": "user", "content": user},
        {"role": "assistant", "content": assistant},
    ]}

def format_introspector(example):
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
        if ctx_before: user += f"Context before:\n```rust\n{ctx_before}\n```\n\n"
        user += f"Code:\n```rust\n{snippet}\n```"
        parts = [f"Element type: {element_type}"]
        if element_name: parts.append(f"Name: {element_name}")
        if element_sig: parts.append(f"Signature: {element_sig}")
        if symbol_data: parts.append(f"Symbol data: {json.dumps(symbol_data, indent=2)}")
        assistant = "\n".join(parts)
    elif phase == "type_inference":
        user = f"What is the type information for this Rust code?\n\n```rust\n{snippet}\n```"
        parts = []
        if element_name: parts.append(f"Element: {element_name}")
        if element_type: parts.append(f"Type: {element_type}")
        if type_data: parts.append(f"Type data: {json.dumps(type_data, indent=2)}")
        assistant = "\n".join(parts) if parts else "No type information available."
    elif phase == "parsing":
        user = f"Parse this Rust code and describe the syntax structure.\n\n```rust\n{snippet}\n```"
        parts = [f"Element type: {element_type}"]
        if element_name: parts.append(f"Name: {element_name}")
        if syntax_data: parts.append(f"Syntax data: {json.dumps(syntax_data, indent=2)}")
        assistant = "\n".join(parts)
    else:
        user = f"Analyze this Rust code.\n\n```rust\n{snippet}\n```"
        parts = [f"Phase: {phase}", f"Element type: {element_type}"]
        if element_name: parts.append(f"Name: {element_name}")
        if element_sig: parts.append(f"Signature: {element_sig}")
        assistant = "\n".join(parts)
    return {"messages": [
        {"role": "system", "content": SYSTEM_ANALYSIS},
        {"role": "user", "content": user},
        {"role": "assistant", "content": assistant},
    ]}

def is_valid(ex):
    msgs = ex.get("messages", [])
    if len(msgs) < 3: return False
    return all(m.get("content", "").strip() for m in msgs)

def save_jsonl(ds, path):
    with open(path, "w") as f:
        for ex in ds:
            f.write(json.dumps(ex) + "\n")

if __name__ == "__main__":
    import os
    outdir = "/workspace/data"
    os.makedirs(outdir, exist_ok=True)

    print("Loading and formatting Strandset-Rust-v1...", flush=True)
    ds = load_dataset("Fortytwo-Network/Strandset-Rust-v1", split="train")
    ds = ds.map(format_strandset, remove_columns=ds.column_names, num_proc=8)
    ds = ds.filter(is_valid, num_proc=8)
    save_jsonl(ds, f"{outdir}/strandset.jsonl")
    print(f"  Saved {len(ds)} examples to {outdir}/strandset.jsonl", flush=True)

    print("Loading and formatting introspector/rust-analyser...", flush=True)
    ds = load_dataset("introspector/rust-analyser", split="train")
    ds = ds.map(format_introspector, remove_columns=ds.column_names, num_proc=8)
    ds = ds.filter(is_valid, num_proc=8)
    save_jsonl(ds, f"{outdir}/introspector.jsonl")
    print(f"  Saved {len(ds)} examples to {outdir}/introspector.jsonl", flush=True)

    print("Done! All datasets preprocessed.", flush=True)