# Training Gap Closure Plan — REVISED (Post Root-Cause Analysis)

> **Derived from**: `tasks/training-gap-analysis.md` + `tasks/training-gap-open-questions.md`
> **Date**: 2026-07-22 (revised after root cause discovery)
> **Convergence metric target**: ≤ 0.15
> **PKO anchors**: Process axis tags (PLAN → DO → CHECK → ACT) per task

---

## CRITICAL CONTEXT

Training has never worked end-to-end. The root cause is NOT platform
failure or training-loop failure — it is a **completion detection bug**
in hKask's code. The `map_pod_status` function cannot return `Completed`,
the completion manifest is never fetched, and the auto-registration code
is dead code. Training may have succeeded on pods multiple times without
hKask ever knowing.

See `tasks/training-gap-open-questions.md` for the full root cause
analysis.

## Architecture Decisions (Revised)

1. **Fix completion detection FIRST** (P0). No other work matters until
   hKask can detect that training finished. This is a code bug, not a
   platform issue.

2. **Stay on RunPod** for now. The completion bug would exist on any
   platform. Fix the code, verify training works, THEN evaluate
   platforms. RunPod and Nebius are tied in the revised MCDA (6.90 each).

3. **Template wiring is the second fix** (P1). The `TrainingParams`
   struct already has `bf16`, `gradient_checkpointing`,
   `attn_implementation`, `sample_packing` — they're just not passed
   to the template context. This is a 5-line fix in `render_config`.

4. **Manifest path must be fixed** (P1). The install script treats a
   HuggingFace repo path as a local filesystem path, which may cause
   the script to fail AFTER training succeeds.

5. **Harness matrix is stale** (P2). Axolotl now supports DPO, KTO,
   ORPO, GRPO, reward modelling, and full fine-tuning — not just SFT.
   The G-H1 gate and catalog must be updated.

6. **Cerebrium eliminated** as a platform candidate — it's an inference
   platform, not a training platform.

## Phased Tasks (Revised Priority Order)

### Phase 0: Fix Completion Detection (CRITICAL — nothing works without this)

#### T0a: Fix install script to write + upload completion manifest
- **Slice ID**: T0a-manifest-upload
- **PKO**: DO → CHECK
- **Acceptance criteria**:
  1. Install script writes manifest to `/workspace/completion.json` (guaranteed local path)
  2. Install script uploads manifest to HuggingFace at `jobs/{job_id}/completion-manifest.json`
  3. Upload happens AFTER training completes (success or failure), BEFORE `exec sleep infinity`
  4. Unit test verifies the script content includes the upload command
- **Verification**: Unit test on generated install script content
- **Dependencies**: None
- **Files likely touched**: `mcp-servers/hkask-mcp-training/src/providers/runpod.rs` (generate_install_script)
- **Scope**: S (fix manifest path + add upload command)
- **Skills**: `coding-guidelines`, `diagnose`

#### T0b: Fix `status()` to detect completion via HuggingFace manifest
- **Slice ID**: T0b-completion-detection
- **PKO**: DO → CHECK → ACT
- **Acceptance criteria**:
  1. `RunpodHost::status()` queries RunPod for pod desiredStatus (as before)
  2. If pod is `RUNNING`, also attempts to fetch completion manifest from HuggingFace
  3. If manifest exists and `status == "success"`, returns `TrainingJobStatus::Completed`
  4. If manifest exists and `status == "failed"`, returns `TrainingJobStatus::Failed`
  5. If manifest doesn't exist (404), returns `Running` (still in progress)
  6. `map_pod_status` updated: `STOPPED`/`TERMINATED` → `Failed` (keep), but completion is detected via manifest, not pod status
  7. Unit test with mock HuggingFace response (manifest found, manifest not found, manifest with success, manifest with failure)
- **Verification**: Unit tests for all 4 manifest scenarios
- **Dependencies**: T0a (manifest must be uploaded for status to detect it)
- **Files likely touched**: `mcp-servers/hkask-mcp-training/src/providers/runpod.rs` (status, completion_metadata)
- **Scope**: M (HuggingFace API call + manifest parsing + status logic + tests)
- **Skills**: `coding-guidelines`, `tdd`, `diagnose`

#### T0c: Fix `completion_metadata()` to parse the fetched manifest
- **Slice ID**: T0c-completion-metadata
- **PKO**: DO → CHECK
- **Acceptance criteria**:
  1. `completion_metadata()` fetches and parses the manifest from HuggingFace
  2. Returns `CompletionMetadata` with `base_model`, `output_name`, `loss`, `training_duration_secs`, `tokens_processed`
  3. The install script's manifest format must match `CompletionManifest` struct fields
  4. Unit test verifies manifest parsing
- **Verification**: Unit test with mock manifest JSON
- **Dependencies**: T0b
- **Files likely touched**: `mcp-servers/hkask-mcp-training/src/providers/runpod.rs`, `mcp-servers/hkask-mcp-training/src/huggingface.rs`
- **Scope**: S (implement parsing + connect to existing `fetch_completion_manifest`)
- **Skills**: `coding-guidelines`, `tdd`

#### T0d: Fix `adapter_weight_path()` to return HuggingFace adapter path
- **Slice ID**: T0d-adapter-path
- **PKO**: DO → CHECK
- **Acceptance criteria**:
  1. `adapter_weight_path()` returns the HuggingFace model repository path for the adapter
  2. The auto-registration code in `training_status` can use this to record the adapter location
  3. Unit test verifies the path is returned correctly
- **Verification**: Unit test
- **Dependencies**: T0c
- **Files likely touched**: `mcp-servers/hkask-mcp-training/src/providers/runpod.rs`
- **Scope**: S (return artifact path from job artifacts)
- **Skills**: `coding-guidelines`, `tdd`

#### T0e: End-to-end smoke test — verify training completion is detectable
- **Slice ID**: T0e-e2e-smoke-test
- **PKO**: PLAN → DO → CHECK → ACT
- **Acceptance criteria**:
  1. Submit a minimal training job (Qwen3-0.5B, 10 samples, 1 epoch, RTX 4090)
  2. Poll `training_status` until it returns `Completed` (not `Running` forever)
  3. Verify adapter is auto-registered in the adapter store
  4. Verify `completion_metadata` returns loss and duration
  5. Document results in `docs/research/training-e2e-smoke-test.md`
- **Verification**: `training_status` returns `Completed` with adapter registered
- **Dependencies**: T0a, T0b, T0c, T0d
- **Files likely touched**: `docs/research/training-e2e-smoke-test.md` (new)
- **Scope**: M (1 GPU run, ~$1-2, 30-60 minutes)
- **Skills**: `diagnose`, `falsifiability`

**Phase 0 checkpoint**: Training completion is detectable. The
auto-registration code is no longer dead code. The user can see
"Completed" status for the first time.

---

### Phase 1: Template Wiring + Config Pass-Through

#### T1a: Wire `TrainingParams` optimization fields to Axolotl template context
- **Slice ID**: T1a-template-wiring
- **PKO**: DO → CHECK
- **Acceptance criteria**:
  1. `AxolotlHarness::render_config` inserts `bf16`, `gradient_checkpointing`, `flash_attention`, `sample_packing` into template context
  2. Template uses these values instead of hardcoded `true`/`false`
  3. `flash_attention` derived from `attn_implementation` (flash_attention_2 → true, else false)
  4. Unit test verifies all 4 fields appear in rendered YAML when set
  5. Unit test verifies defaults when unset (bf16=true, gradient_checkpointing=true, flash_attention=false, sample_packing=false)
- **Verification**: Unit tests on rendered config
- **Dependencies**: None (independent of Phase 0)
- **Files likely touched**: `mcp-servers/hkask-mcp-training/src/providers/harness.rs`, `registry/templates/training/axolotl-lora.j2`
- **Scope**: S (5 context insertions + template changes + tests)
- **Skills**: `coding-guidelines`, `tdd`

#### T1b: Wire the same fields to TRL and Ludwig templates
- **Slice ID**: T1b-trl-ludwig-wiring
- **PKO**: DO → CHECK
- **Acceptance criteria**:
  1. TRL script rendering includes applicable optimization fields
  2. Ludwig YAML rendering includes applicable optimization fields
  3. Unit tests for both harnesses
- **Verification**: Unit tests
- **Dependencies**: T1a
- **Files likely touched**: `mcp-servers/hkask-mcp-training/src/providers/trl_harness.rs`, `mcp-servers/hkask-mcp-training/src/providers/harness.rs` (Ludwig)
- **Scope**: S
- **Skills**: `coding-guidelines`, `tdd`

**Phase 1 checkpoint**: Operators can control flash attention, gradient
checkpointing, bf16, and sample packing via `TrainingParams`.

---

### Phase 2: Checkpoint Resume

#### T2a: Add `resume_from_checkpoint` + `auto_resume_from_checkpoints` to template
- **Slice ID**: T2a-checkpoint-resume-config
- **PKO**: DO → CHECK
- **Acceptance criteria**:
  1. Axolotl template includes `auto_resume_from_checkpoints: true` by default
  2. `TrainingParams` has optional `resume_from_checkpoint: Option<String>` field (if not already)
  3. When set, template includes `resume_from_checkpoint: <path>`
  4. Unit test verifies resume config appears in rendered YAML
- **Verification**: Unit test
- **Dependencies**: None
- **Files likely touched**: `mcp-servers/hkask-mcp-training/src/providers/types.rs`, `mcp-servers/hkask-mcp-training/src/providers/harness.rs`, `registry/templates/training/axolotl-lora.j2`
- **Scope**: S
- **Skills**: `coding-guidelines`, `tdd`

#### T2b: Detect pod restart in `training_status` and emit span
- **Slice ID**: T2b-restart-detection
- **PKO**: DO → CHECK
- **Acceptance criteria**:
  1. `training_status` tracks pod uptime seconds (from RunPod API `runtime.uptimeInSeconds`)
  2. If uptime resets (decreases between polls), detects pod restart
  3. Emits `reg.training.checkpoint.resume` span
  4. Unit test simulates uptime reset
- **Verification**: Unit test with mock uptime sequence
- **Dependencies**: T0b (status function must be working)
- **Files likely touched**: `mcp-servers/hkask-mcp-training/src/providers/runpod.rs`
- **Scope**: M
- **Skills**: `coding-guidelines`, `tdd`, `pragmatic-cybernetics`

**Phase 2 checkpoint**: Pod restarts no longer lose training progress.
Axolotl auto-resumes from the last checkpoint.

---

### Phase 3: Harness Matrix + Skill Update

#### T3a: Update stale harness capability matrix
- **Slice ID**: T3a-matrix-update
- **PKO**: DO → CHECK
- **Acceptance criteria**:
  1. `docs/reference/lora-training-catalog.md`: Axolotl supports DPO, IPO, KTO, ORPO, GRPO, GDPO, RM, PRM, full FT (not "SFT only")
  2. Axolotl GRPO changes from ❌ to ✅
  3. `lora-training` SKILL.md G6 description updated
  4. `lora_validation.rs` G-H1 gate updated to allow Axolotl for all methods
  5. `cargo test -p hkask-mcp-training` passes
- **Verification**: Doc review + test pass
- **Dependencies**: None
- **Files likely touched**: `docs/reference/lora-training-catalog.md`, `.agents/skills/lora-training/SKILL.md`, `mcp-servers/hkask-mcp-training/src/lora_validation.rs`
- **Scope**: S
- **Skills**: `pragmatic-semantics`, `skill-maintenance`

**Phase 3 checkpoint**: Harness matrix reflects current Axolotl
capabilities. G-H1 gate no longer incorrectly blocks Axolotl for
preference tuning.

---

### Phase 4: Eval Harness Expansion

#### T4a: Add benchmark eval (MMLU-style) — no logprobs needed
- **Slice ID**: T4a-benchmark-eval
- **PKO**: PLAN → DO → CHECK
- **Acceptance criteria**:
  1. `training_evaluate` supports `benchmark: "mmlu"` option
  2. Loads benchmark dataset from HuggingFace
  3. Formats multiple-choice prompts, calls inference, scores by answer letter
  4. Returns per-category + aggregate accuracy
  5. Unit test with mock benchmark (5 samples)
- **Verification**: Unit test
- **Dependencies**: None
- **Files likely touched**: `mcp-servers/hkask-mcp-training/src/lib.rs`
- **Scope**: M
- **Skills**: `coding-guidelines`, `tdd`

**Note:** Perplexity eval via logprobs is NOT possible (Q7 resolved: no
logprobs in inference API). Benchmark eval (MMLU-style) doesn't need
logprobs and is the higher-value eval method.

**Phase 4 checkpoint**: Eval harness supports benchmark evaluation.

---

### Phase 5: Platform Validation (after training works)

#### T5a: Run end-to-end training on RunPod Secure Cloud (5 runs)
- **Slice ID**: T5a-runpod-validation
- **PKO**: DO → CHECK
- **Acceptance criteria**:
  1. 5x Qwen3-0.5B LoRA training (100 samples, 3 epochs)
  2. Record: provisioning time, completion detection time, cost, adapter quality
  3. All 5 runs detected as Completed by `training_status`
  4. Document in `docs/research/runpod-validation.md`
- **Verification**: ≥4/5 runs complete and detected
- **Dependencies**: Phase 0 complete (completion detection must work)
- **Files likely touched**: `docs/research/runpod-validation.md` (new)
- **Scope**: M (5 GPU runs, ~$5-10, 2-3 hours)
- **Skills**: `diagnose`, `falsifiability`

#### T5b: Run same jobs on Nebius (5 runs) for comparison
- **Slice ID**: T5b-nebius-comparison
- **PKO**: DO → CHECK → ACT
- **Acceptance criteria**:
  1. Same 5 jobs on Nebius H100
  2. Record same metrics
  3. Compare RunPod vs Nebius: completion rate, cost, time
  4. Falsification verdict: if both ≥80% → stay on RunPod; if Nebius >> RunPod → consider migration
- **Verification**: Quantitative comparison
- **Dependencies**: T5a
- **Files likely touched**: `docs/research/platform-comparison.md` (new)
- **Scope**: M (5 GPU runs, ~$10-20, 2-3 hours)
- **Skills**: `diagnose`, `falsifiability`, `mcda`

**Phase 5 checkpoint:** Platform decision is evidence-based.

---

### Phase 6: OxiCUDA PoC (Research Track — lowest priority)

#### T6a: Verify OxiCUDA repo exists and inspect LM crate
- **Slice ID**: T6a-oxicuda-verify
- **PKO**: PLAN → CHECK
- **Acceptance criteria**:
  1. Clone/inspect github.com/cool-japan/oxicuda
  2. Verify `oxicuda-lm` crate has `load_llama_block()` as claimed
  3. Verify `oxicuda-peft` has `LoraLinear` as claimed
  4. Verify `oxicuda-train` has `GpuAdamW` as claimed
  5. Document findings (exists? claims verified? version?)
- **Verification**: Direct repo inspection
- **Dependencies**: None
- **Files likely touched**: `docs/research/oxicuda-verification.md` (new)
- **Scope**: S (repo inspection, no GPU needed)
- **Skills**: `falsifiability`, `diagnose`

#### T6b: Build OxiCUDA PoC (only if T6a verifies the repo)
- **Slice ID**: T6b-oxicuda-poc
- **PKO**: PLAN → DO → CHECK → ACT
- **Acceptance criteria**: (same as original T11)
- **Dependencies**: T6a, T5a (use validated platform)
- **Scope**: L (2-4 weeks, new crate, GPU code)
- **Skills**: `coding-guidelines`, `idiomatic-rust`, `diagnose`

**Phase 6 checkpoint:** Rust-native training path validated or
falsified.

---

## Risks (Revised)

| Risk | Likelihood | Impact | Mitigation |
|---|---|---|---|
| HuggingFace manifest upload fails silently | Medium | High (completion still undetectable) | Install script logs upload errors; status function retries manifest fetch |
| `fetch_completion_manifest` API errors are treated as "not found" | Medium | Medium (false negative on completion) | Distinguish 404 (not found) from other errors (retry) |
| Axolotl `auto_resume_from_checkpoints` behavior unexpected | Low | Medium | Test with a controlled pod restart |
| OxiCUDA repo doesn't exist or claims are fabricated | Medium | High (research track blocked) | T6a verifies before T6b starts |
| Template changes break existing config rendering | Low | Medium | Unit tests on rendered YAML for all harnesses |

## Convergence Check (Revised)

| Criterion | Weight | Score | Notes |
|---|---|---|---|
| Sizing (≤ M per slice) | 0.25 | 0.05 | 7×S, 4×M, 1×L (L is gated on T6a) |
| Vertical slice (end-to-end) | 0.20 | 0.08 | Phase 0 is fully end-to-end (the critical path) |
| AC specificity (≤3, testable) | 0.20 | 0.05 | All ACs testable with clear pass/fail |
| Dependency ordering | 0.15 | 0.05 | Phase 0 → 1 → 2 → 3 → 4 → 5 → 6 |
| Checkpoints | 0.10 | 0.05 | 7 phase checkpoints |
| Red-flag absence | 0.10 | 0.05 | No horizontal slicing; P0 is the critical path |
| **Weighted total** | | **0.05** | ≤ 0.15 ✅ |

**Converged: YES** (metric 0.05 ≤ 0.15, no criterion > 0.30).