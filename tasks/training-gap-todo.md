# Training Gap Closure TODO — REVISED

> **Derived from**: `tasks/training-gap-plan.md` (revised)
> **Date**: 2026-07-22
> **Priority**: Phase 0 is CRITICAL — training has never worked because
>   completion detection is broken. Fix this first.

## Phase 0: Fix Completion Detection (CRITICAL)

### T0a: Fix install script manifest write + upload
- [ ] Change manifest write path to `/workspace/completion.json` (guaranteed local)
- [ ] Add `huggingface-cli upload` command for the manifest to HuggingFace
  - Upload to `jobs/{job_id}/completion-manifest.json` in the model repo
  - Must happen AFTER training completes, BEFORE `exec sleep infinity`
- [ ] Ensure manifest JSON fields match `CompletionManifest` struct
  (`job_id`, `status`, `dataset_sha256`, `adapter`, `finished_at`)
- [ ] Unit test: verify install script content includes manifest upload command
- [ ] `cargo test -p hkask-mcp-training` — pass

### T0b: Fix `status()` to detect completion via HuggingFace manifest
- [ ] Add HuggingFace manifest fetch to `RunpodHost::status()`
  - If pod desiredStatus is `RUNNING`, try fetching manifest from HuggingFace
  - If manifest found + status="success" → return `Completed`
  - If manifest found + status="failed" → return `Failed`
  - If manifest not found (404) → return `Running` (still in progress)
- [ ] Distinguish 404 (not found, training in progress) from other HF API errors (retry)
- [ ] Unit test: mock HF response with manifest (success)
- [ ] Unit test: mock HF response with manifest (failure)
- [ ] Unit test: mock HF response with 404 (not found)
- [ ] Unit test: mock HF response with error (retry needed)
- [ ] `cargo test -p hkask-mcp-training` — pass

### T0c: Implement `completion_metadata()` to parse fetched manifest
- [ ] Change `completion_metadata()` from `Ok(None)` to fetch + parse manifest
- [ ] Map manifest fields to `CompletionMetadata` struct
- [ ] Handle missing/optional fields gracefully (loss may not be in manifest)
- [ ] Unit test: mock manifest JSON → CompletionMetadata
- [ ] `cargo test -p hkask-mcp-training` — pass

### T0d: Fix `adapter_weight_path()` to return HuggingFace adapter path
- [ ] Return the model repository path from job artifacts
- [ ] The adapter is already uploaded by the install script to HuggingFace
- [ ] Unit test: verifies path is returned
- [ ] `cargo test -p hkask-mcp-training` — pass

### T0e: End-to-end smoke test
- [ ] Submit minimal training job: Qwen3-0.5B, 10 samples, 1 epoch, RTX 4090
- [ ] Poll `training_status` every 60s
- [ ] **VERIFY**: status transitions from Queued → Running → **Completed** (not Running forever)
- [ ] Verify adapter is auto-registered in adapter store
- [ ] Verify `completion_metadata` returns loss and duration
- [ ] Document results in `docs/research/training-e2e-smoke-test.md`
- [ ] **Phase 0 checkpoint**: FIRST EVER successful end-to-end training detection

## Phase 1: Template Wiring + Config Pass-Through

### T1a: Wire optimization fields to Axolotl template
- [ ] Add `bf16` to template context from `p.advanced.bf16`
- [ ] Add `gradient_checkpointing` from `p.advanced.gradient_checkpointing`
- [ ] Add `flash_attention` derived from `p.advanced.attn_implementation`
- [ ] Add `sample_packing` from `p.sequence.sample_packing`
- [ ] Update `axolotl-lora.j2`: use variables instead of hardcoded values
- [ ] Unit test: all 4 fields appear in rendered YAML when set
- [ ] Unit test: defaults when unset (bf16=true, gc=true, fa=false, sp=false)
- [ ] `cargo test -p hkask-mcp-training` — pass

### T1b: Wire to TRL and Ludwig templates
- [ ] Wire applicable fields to TRL script rendering
- [ ] Wire applicable fields to Ludwig YAML rendering
- [ ] Unit tests for both
- [ ] `cargo test -p hkask-mcp-training` — pass
- [ ] **Phase 1 checkpoint**: operators can control optimization settings

## Phase 2: Checkpoint Resume

### T2a: Add resume config to Axolotl template
- [ ] Add `auto_resume_from_checkpoints: true` to template (default)
- [ ] Add optional `resume_from_checkpoint` to `TrainingParams` (if not present)
- [ ] Unit test: resume config in rendered YAML
- [ ] `cargo test -p hkask-mcp-training` — pass

### T2b: Detect pod restart + emit span
- [ ] Track pod uptime seconds across status polls
- [ ] Detect uptime reset (pod restarted)
- [ ] Emit `reg.training.checkpoint.resume` span
- [ ] Unit test: mock uptime sequence (100s → 5s = restart detected)
- [ ] `cargo test -p hkask-mcp-training` — pass
- [ ] **Phase 2 checkpoint**: pod restarts auto-resume training

## Phase 3: Harness Matrix Update

### T3a: Update stale matrix
- [ ] Update `docs/reference/lora-training-catalog.md`:
  - Axolotl: SFT ✅, DPO ✅, KTO ✅, ORPO ✅, GRPO ✅, Reward ✅, Full FT ✅
  - Remove "SFT only" label
- [ ] Update `lora-training` SKILL.md G6 description
- [ ] Update `lora_validation.rs` G-H1 gate: Axolotl supports all methods
- [ ] `cargo test -p hkask-mcp-training` — pass
- [ ] **Phase 3 checkpoint**: matrix reflects current Axolotl capabilities

## Phase 4: Eval Harness

### T4a: Add benchmark eval (MMLU-style)
- [ ] Add `benchmark: Option<String>` to eval request
- [ ] Implement MMLU-style multiple-choice prompt formatting
- [ ] Implement scoring (exact match on answer letter)
- [ ] Return per-category + aggregate accuracy
- [ ] Unit test with 5-sample mock
- [ ] `cargo test -p hkask-mcp-training` — pass
- [ ] **Phase 4 checkpoint**: benchmark eval available

## Phase 5: Platform Validation (after training works)

### T5a: RunPod validation (5 runs)
- [ ] Submit 5x Qwen3-0.5B LoRA (100 samples, 3 epochs) on RunPod Secure Cloud
- [ ] Record: provisioning time, completion detection time, cost, adapter quality
- [ ] All 5 detected as Completed by `training_status`
- [ ] Document in `docs/research/runpod-validation.md`

### T5b: Nebius comparison (5 runs)
- [ ] Run same 5 jobs on Nebius H100
- [ ] Record same metrics
- [ ] Compare: completion rate, cost, time
- [ ] Falsification verdict in `docs/research/platform-comparison.md`
- [ ] **Phase 5 checkpoint**: platform decision evidence-based

## Phase 6: OxiCUDA PoC (Research Track)

### T6a: Verify OxiCUDA repo
- [ ] Clone/inspect github.com/cool-japan/oxicuda
- [ ] Verify oxicuda-lm has load_llama_block()
- [ ] Verify oxicuda-peft has LoraLinear
- [ ] Verify oxicuda-train has GpuAdamW
- [ ] Document in `docs/research/oxicuda-verification.md`
- [ ] **If repo doesn't exist or claims are false: STOP. Document and exit.**

### T6b: Build PoC (only if T6a passes)
- [ ] Create `crates/hkask-training/`
- [ ] Add OxiCUDA as git dependency
- [ ] Implement Qwen3 weight loader
- [ ] Load Qwen3-0.5B on GPU
- [ ] Run GEMM → verify CUDA
- [ ] Wrap with LoRA
- [ ] Run 1 training step
- [ ] Save adapter
- [ ] Document in `docs/research/oxicuda-poc-results.md`
- [ ] **Phase 6 checkpoint**: Rust-native training validated or falsified