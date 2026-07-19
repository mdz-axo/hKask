---
name: adapter-eval
visibility: public
description: "Pre-flight validation and evaluation discipline for LoRA adapters. Enforces 3 mandatory checks (load, weights, sanity) before any full eval run. Prevents wasted compute from broken adapters, PiSSA conversion issues, or shape mismatches. Pairs with the training_preflight_check MCP tool."
---


# Adapter Evaluation

Pre-flight validation and evaluation discipline for LoRA adapters. Catches broken adapters before they waste hours of GPU compute on a full eval run.

## When to Use

- Before running a full evaluation on a trained adapter (225+ examples, hours of GPU time)
- After converting a PiSSA adapter to standard LoRA (verify the conversion didn't corrupt weights)
- After downloading an adapter from HuggingFace (verify it's not empty or malformed)
- Before deploying an adapter to a production inference endpoint
- When an adapter eval shows catastrophic regression (run pre-flight to diagnose root cause)

## The Problem This Skill Solves

A full eval on 225 examples takes 2-5 hours on an H100 ($6-16). Running it on a broken adapter wastes that entire budget. The 3 most common failure modes are:

1. **PiSSA double-counting**: A PiSSA-trained adapter loaded with `init_lora_weights: true` on the original base model. The principal components are applied twice (once in the base, once in the adapter), causing catastrophic regression (0.56 to 0.07). Fix: convert PiSSA to standard LoRA first.

2. **Shape mismatch**: A/B matrices swapped during conversion. The adapter loads but produces garbage. Fix: verify lora_A has shape [r, in_features] and lora_B has shape [out_features, r].

3. **All-zero weights**: The adapter file is present but the weights are zero (failed training, corrupted download, wrong file). Fix: verify non-zero weight count.

All three are caught in under 30 seconds by the pre-flight checks. Running a full eval without pre-flight is negligence.

## Instructions

### Phase 1: Verify (pre-flight checks)

Before running any full eval, call `training_preflight_check` or run these 3 checks manually:

1. **Load check**: Verify `adapter_config.json` exists, parses as JSON, and `init_lora_weights` is either `true` (standard LoRA) or `pissa_niter_N` (PiSSA). If the adapter was PiSSA-trained and you are loading in 4-bit, you MUST convert to standard LoRA first (see Phase 0 below).

2. **Weights check**: Verify `adapter_model.safetensors` exists and is over 1KB. Load it and verify LoRA B weights are non-zero (more than 0 percent of elements are non-zero). All-zero weights mean the adapter is broken.

3. **Sanity check**: Generate output on 1 test example. Verify the response is over 50 characters and looks like Rust code (not garbage, not empty, not an error message). This catches shape mismatches, PiSSA double-counting, and other subtle loading issues that do not throw errors.

If any check fails, STOP. Do not run the full eval. Diagnose and fix the adapter first.

### Phase 0 (if needed): Convert PiSSA to standard LoRA

If the adapter was trained with PiSSA (`init_lora_weights: pissa_niter_4`) and you need to load it in 4-bit (where PiSSA decomposition fails), convert it first:

1. Load the base model in bf16 (not 4-bit)
2. Load the PiSSA adapter (pissa_niter_4) — this properly decomposes the base model
3. Merge the adapter into the base model (`model.merge_and_unload()`)
4. Compute the delta between merged and original weights
5. SVD decompose the delta to rank-r to get new (A, B) matrices
6. Save with `init_lora_weights: true` — this is now a standard LoRA that works in any backend

**Critical**: lora_A must have shape [r, in_features] and lora_B must have shape [out_features, r]. If you swap them, the adapter loads but produces garbage. Always verify shapes after conversion.

### Phase 2: Baseline (once, then hardcode)

Run the baseline eval (base model, no adapter) ONCE. The baseline is deterministic — same model, same prompts, same scoring. It does not change between runs. Hardcode the results and never re-run the baseline.

If you change the eval script (different prompts, different scoring, different system messages), THEN re-run the baseline. Otherwise, use the hardcoded values.

### Phase 3: Full adapter eval

Only after pre-flight checks pass and the baseline is known:

1. Load the base model (4-bit via Unsloth, or bf16 for PiSSA)
2. Load the adapter via `PeftModel.from_pretrained`
3. Run inference on all test examples
4. Score per-category with the same logic as the baseline
5. Report per-category accuracy and delta vs baseline

### Phase 4: Diagnosis (if regression)

If the adapter shows regression vs baseline (delta is negative):

- **Catastrophic regression (delta below -0.3)**: Almost certainly a loading issue. Re-run pre-flight checks. Check for PiSSA double-counting, shape mismatches, or all-zero weights.
- **Moderate regression (delta below -0.1)**: The adapter may be overfitting or the training data may be wrong. Check the training loss curve and eval_loss progression.
- **Small regression (delta below -0.03)**: Within noise range of the eval set. May not be significant.

## MCP Tool

The `training_preflight_check` tool in `hkask-mcp-training` implements Phase 1:

```json
{
  "adapter_path": "/workspace/adapter",
  "model": "unsloth/Qwen3.6-27B",
  "test_prompt": "Generate a simple Rust function that adds two numbers.",
  "min_response_chars": 50
}
```

Returns:
```json
{
  "all_pass": true,
  "checks": [
    {"check": "load", "status": "pass", "init_lora_weights": true, "r": 32, "lora_alpha": 64},
    {"check": "weights", "status": "pass", "size_bytes": 305000000},
    {"check": "sanity", "status": "pass", "response_chars": 245, "response_preview": "fn add(a: i32, b: i32) -> i32 {"}
  ]
}
```

If any check fails, `all_pass` is `false` and `failed_at` indicates which check failed.

## Cost Discipline

| Phase | Time | Cost (H100 at $3.19/hr) |
|-------|------|------------------------|
| Pre-flight (3 checks) | ~30 sec | $0.03 |
| PiSSA conversion (if needed) | ~10 min | $0.53 |
| Baseline (once, then hardcode) | ~2-3h | $6-10 |
| Full adapter eval | ~2-3h | $6-10 |
| **Total with pre-flight** | ~5h | **~$16** |
| **Total without pre-flight (wasted runs)** | 15-20h | **~$50-64** |

Pre-flight checks save $34-48 per eval cycle by preventing wasted full runs on broken adapters.