# Training Gap Closure TODO — REVISED

> **Derived from**: `tasks/training-gap-plan.md` (revised)
> **Date**: 2026-07-22
> **Priority**: Phase 0 is CRITICAL — training has never worked because
>   completion detection is broken. Fix this first.

## Phase 0: Fix Completion Detection (CRITICAL) — ✅ CODE COMPLETE

### T0a: Fix install script manifest write + upload — ✅
- [x] Change manifest write path to `/workspace/completion.json` (guaranteed local)
- [x] Add `huggingface-cli upload` command for the manifest to HuggingFace
- [x] Manifest JSON fields match `CompletionManifest` struct
- [x] Unit test: verify install script content includes manifest upload command

### T0b: Fix `status()` to detect completion via HuggingFace manifest — ✅
- [x] Add `check_completion_manifest` helper to `TrainingServer`
- [x] If pod is `Running`, fetch manifest from HuggingFace via `fetch_completion_manifest`
- [x] If manifest found + status="success" → return `Completed`
- [x] If manifest found + status="failed" → return `Failed`
- [x] If manifest not found → return `Running` (still in progress)

### T0c: Implement `completion_metadata()` to parse fetched manifest — ✅
- [x] `training_status` uses manifest data directly for adapter metadata
- [x] Manifest fields (base_model, loss, training_duration_secs) flow to adapter registration

### T0d: Fix `adapter_weight_path()` to return HuggingFace adapter path — ✅
- [x] Adapter path comes from manifest's `adapter.repository` field
- [x] `build_trained_adapter` receives the HF repo path as weight_path
- [x] Old `resolve_adapter_path` (which called broken `adapter_weight_path`) removed

### T0e: End-to-end smoke test — ⏳ REQUIRES GPU + HF CREDENTIALS
- [ ] Submit minimal training job: Qwen3-0.5B, 10 samples, 1 epoch, RTX 4090
- [ ] Poll `training_status` every 60s
- [ ] **VERIFY**: status transitions from Queued → Running → **Completed**
- [ ] Verify adapter is auto-registered in adapter store
- [ ] Document results in `docs/research/training-e2e-smoke-test.md`

**Phase 0 code checkpoint**: Completion detection pipeline is fixed.
`map_pod_status` limitation is worked around by manifest-based detection.
All code changes build, pass clippy, and pass 136 tests.

## Phase 1: Template Wiring + Config Pass-Through — ✅ CODE COMPLETE

### T1a: Wire optimization fields to Axolotl template — ✅
- [x] Add `bf16` to template context from `p.advanced.bf16`
- [x] Add `gradient_checkpointing` from `p.advanced.gradient_checkpointing`
- [x] Add `flash_attention` derived from `p.advanced.attn_implementation`
- [x] Add `sample_packing` from `p.sequence.sample_packing`
- [x] Update `axolotl-lora.j2`: use variables instead of hardcoded values
- [x] Unit test: `axolotl_harness_wires_optimization_fields` verifies all 4 fields

### T1b: Wire to TRL and Ludwig templates — ⏳ NOT YET DONE
- [ ] Wire applicable fields to TRL script rendering
- [ ] Wire applicable fields to Ludwig YAML rendering
- [ ] Unit tests for both

## Phase 2: Checkpoint Resume — ⏳ NOT YET DONE

### T2a: Add resume config to Axolotl template
- [ ] Add `auto_resume_from_checkpoints: true` to template (default)
- [ ] Add optional `resume_from_checkpoint` to `TrainingParams`
- [ ] Unit test: resume config in rendered YAML

### T2b: Detect pod restart + emit span
- [ ] Track pod uptime seconds across status polls
- [ ] Detect uptime reset (pod restarted)
- [ ] Emit `reg.training.checkpoint.resume` span
- [ ] Unit test: mock uptime sequence

## Phase 3: Harness Matrix Update — ⏳ NOT YET DONE

### T3a: Update stale matrix
- [ ] Update `docs/reference/lora-training-catalog.md`: Axolotl supports DPO/KTO/ORPO/GRPO/RM/Full FT
- [ ] Update `lora-training` SKILL.md G6 description
- [ ] Update `lora_validation.rs` G-H1 gate
- [ ] `cargo test -p hkask-mcp-training` — pass

## Phase 4: Eval Harness — ⏳ NOT YET DONE

### T4a: Add benchmark eval (MMLU-style)
- [ ] Add `benchmark: Option<String>` to eval request
- [ ] Implement MMLU-style multiple-choice prompt formatting
- [ ] Implement scoring (exact match on answer letter)
- [ ] Unit test with 5-sample mock

## Phase 5: Platform Validation — ⏳ BLOCKED ON T0e

### T5a: RunPod validation (5 runs)
- [ ] Submit 5x Qwen3-0.5B LoRA on RunPod Secure Cloud
- [ ] All 5 detected as Completed by `training_status`

### T5b: Nebius comparison (5 runs)
- [ ] Run same 5 jobs on Nebius H100
- [ ] Compare: completion rate, cost, time

## Phase 6: OxiCUDA PoC (Research Track) — ⏳ NOT YET DONE

### T6a: Verify OxiCUDA repo
- [ ] Clone/inspect github.com/cool-japan/oxicuda
- [ ] Verify claimed crates exist

### T6b: Build PoC (only if T6a passes)
- [ ] Create `crates/hkask-training/`
- [ ] Load Qwen3-0.5B on GPU
- [ ] Run 1 training step
- [ ] Save adapter