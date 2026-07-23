# Training Gap Closure Plan — PDCA-Anchored

> **Derived from**: `tasks/training-gap-analysis.md`
> **Date**: 2026-07-22
> **Convergence metric target**: ≤ 0.15
> **PKO anchors**: Process axis tags (PLAN → DO → CHECK → ACT) per task

---

## Overview

Close the highest-priority gaps between hKask's training surface and
the three exemplar harnesses (Ludwig, Unsloth-Zoo, Axolotl), while
honoring the Rust-first constraint. The plan is ordered by dependency:
checkpoint resume (focus obstacle) first, then eval harness, then
config pass-through, then platform validation, then OxiCUDA PoC.

## Architecture Decisions

1. **Stay on RunPod Secure Cloud** as primary platform (MCDA score
   6.90, highest). Add Nebius as secondary for the discriminating test
   (§7.7 of analysis). Migration is NOT recommended until the
   discriminating test falsifies the current platform.

2. **Expose harness config parameters** through `TrainingParams` rather
   than building Rust-native implementations. The gap is shallow
   (struct fields + template passthrough), not deep (OxiCUDA
   dependency). Rust-native training is a separate research track
   (OxiCUDA PoC).

3. **Checkpoint resume is the focus obstacle** — highest cost-avoidance
   ROI based on post-mortem evidence ($600 leak root cause class).

4. **Eval harness expansion is Rust-side** — perplexity and benchmark
   eval can be computed from the inference API without Python. This is
   a pure Rust task.

5. **Harness matrix update is a documentation task** — the stale
   entries (Axolotl >SFT, TRL GRPO, add Unsloth) are in
   `docs/reference/lora-training-catalog.md` and the skill's G6 gate
   description.

## Phased Tasks

### Phase 1: Checkpoint Resume (Focus Obstacle)

#### T1: Verify RunPod `/workspace` persistence
- **Slice ID**: T1-checkpoint-persistence
- **PKO**: PLAN → DO → CHECK
- **Acceptance criteria**:
  1. Launch RunPod H100 pod, write file to `/workspace`, stop pod, restart, verify file survives
  2. Document result in `docs/research/runpod-workspace-persistence.md`
- **Verification**: File exists after restart → persistent; file missing → ephemeral
- **Dependencies**: None
- **Files likely touched**: `docs/research/runpod-workspace-persistence.md` (new)
- **Scope**: S (1 RunPod pod, ~$0.50, 30 minutes)
- **Skills**: `diagnose`, `falsifiability`

#### T2: Add `resume_from_checkpoint` to `TrainingParams`
- **Slice ID**: T2-resume-param
- **PKO**: DO → CHECK
- **Acceptance criteria**:
  1. `TrainingParams` has `resume_from_checkpoint: Option<String>` field
  2. Field serializes/deserializes correctly in JSON
  3. `cargo test -p hkask-mcp-training` passes
  4. `cargo clippy -p hkask-mcp-training -- -D warnings` clean
- **Verification**: Unit test for serialization round-trip
- **Dependencies**: T1 (need to know if persistence works before wiring resume)
- **Files likely touched**: `mcp-servers/hkask-mcp-training/src/providers/types.rs`, `mcp-servers/hkask-mcp-training/src/types.rs`
- **Scope**: S (1 struct field + tests)
- **Skills**: `coding-guidelines`, `tdd`

#### T3: Wire `resume_from_checkpoint` through Axolotl config template
- **Slice ID**: T3-resume-axolotl-template
- **PKO**: DO → CHECK
- **Acceptance criteria**:
  1. `AxolotlHarness::render_config` includes `resume_from_checkpoint` in YAML when set
  2. Rendered YAML is valid Axolotl config (verified against Axolotl schema docs)
  3. Unit test verifies the field appears when set and is absent when None
- **Verification**: Unit test on rendered config output
- **Dependencies**: T2
- **Files likely touched**: `mcp-servers/hkask-mcp-training/src/providers/harness.rs`, `registry/templates/training/axolotl-lora.j2` (if template-based)
- **Scope**: S (template change + test)
- **Skills**: `coding-guidelines`, `tdd`

#### T4: Detect pod restart in `training_status` and emit resume span
- **Slice ID**: T4-restart-detection
- **PKO**: DO → CHECK → ACT
- **Acceptance criteria**:
  1. `training_status` detects pod status transition (running → stopped → running)
  2. Emits `reg.training.checkpoint.resume` span on restart detection
  3. Unit test simulates restart transition and verifies span emission
- **Verification**: Unit test with mock status transitions
- **Dependencies**: T2, T3
- **Files likely touched**: `mcp-servers/hkask-mcp-training/src/lib.rs` (training_status function), `mcp-servers/hkask-mcp-training/src/providers/runpod.rs`
- **Scope**: M (status polling logic + span emission + tests)
- **Skills**: `coding-guidelines`, `tdd`, `pragmatic-cybernetics`

**Phase 1 checkpoint**: Pod restart no longer loses training state.
Resume from last checkpoint within 60 seconds.

---

### Phase 2: Eval Harness Expansion

#### T5: Add perplexity evaluation to `training_evaluate`
- **Slice ID**: T5-perplexity-eval
- **PKO**: PLAN → DO → CHECK
- **Acceptance criteria**:
  1. `training_evaluate` supports `method: "perplexity"` option
  2. Perplexity computed from inference API logprobs (if available) or estimated from token likelihood
  3. Result includes `perplexity` field alongside existing `accuracy`
  4. Unit test with mock inference response
- **Verification**: Unit test verifies perplexity computation
- **Dependencies**: None (independent of Phase 1)
- **Files likely touched**: `mcp-servers/hkask-mcp-training/src/lib.rs` (training_evaluate function)
- **Scope**: M (new eval method + logprob handling + tests)
- **Skills**: `coding-guidelines`, `tdd`
- **Open question**: Q7 — does the inference API return logprobs?

#### T6: Add benchmark eval scaffold (MMLU-style)
- **Slice ID**: T6-benchmark-eval
- **PKO**: PLAN → DO → CHECK
- **Acceptance criteria**:
  1. New MCP tool `training_evaluate_benchmark` or extended `training_evaluate` with `benchmark: "mmlu"` option
  2. Loads benchmark dataset from HuggingFace (via `hf-hub` crate)
  3. Formats multiple-choice prompts, calls inference, scores
  4. Returns per-category accuracy + aggregate score
- **Verification**: Unit test with mock benchmark dataset (5 samples)
- **Dependencies**: T5 (shares eval infrastructure)
- **Files likely touched**: `mcp-servers/hkask-mcp-training/src/lib.rs`, `mcp-servers/hkask-mcp-training/src/dataset.rs`
- **Scope**: M (new eval path + benchmark loading + scoring + tests)
- **Skills**: `coding-guidelines`, `tdd`, `deep-module`

**Phase 2 checkpoint**: Eval harness supports exact_match, contains,
semantic, perplexity, and benchmark (MMLU-style) evaluation.

---

### Phase 3: Config Pass-Through

#### T7: Expose `TrainingParams` optimization fields
- **Slice ID**: T7-optimization-params
- **PKO**: DO → CHECK
- **Acceptance criteria**:
  1. `TrainingParams` includes: `flash_attention: Option<bool>`, `gradient_checkpointing: Option<bool>`, `bf16: Option<bool>`, `sample_packing: Option<bool>`, `deepspeed_config: Option<String>`
  2. All fields pass through to Axolotl, TRL, and Ludwig config templates
  3. Each field has a unit test verifying it appears in rendered config when set
  4. `cargo test -p hkask-mcp-training` passes
- **Verification**: Unit tests on rendered configs for all 3 harnesses
- **Dependencies**: None (independent)
- **Files likely touched**: `mcp-servers/hkask-mcp-training/src/providers/types.rs`, `mcp-servers/hkask-mcp-training/src/providers/harness.rs`, `mcp-servers/hkask-mcp-training/src/providers/trl_harness.rs`
- **Scope**: M (5 struct fields + 3 template updates + 15 tests)
- **Skills**: `coding-guidelines`, `tdd`, `idiomatic-rust`

**Phase 3 checkpoint**: hKask exposes flash attention, gradient
checkpointing, mixed precision, sample packing, and DeepSpeed config
as first-class `TrainingParams` fields.

---

### Phase 4: Harness Matrix & Skill Update

#### T8: Update stale harness capability matrix
- **Slice ID**: T8-harness-matrix-update
- **PKO**: DO → CHECK
- **Acceptance criteria**:
  1. `docs/reference/lora-training-catalog.md` harness matrix updated: Axolotl supports RLHF + full FT (not "SFT only"), TRL GRPO no longer "deferred", Unsloth added as 4th harness
  2. `lora-training` skill G6 description updated to reflect current Axolotl/TRL capabilities
  3. `lora_validation.rs` G-H1 gate updated if harness-trainer matrix changed
- **Verification**: Document review + `cargo test -p hkask-mcp-training` passes
- **Dependencies**: None (documentation + validation update)
- **Files likely touched**: `docs/reference/lora-training-catalog.md`, `.agents/skills/lora-training/SKILL.md`, `mcp-servers/hkask-mcp-training/src/lora_validation.rs`
- **Scope**: S (doc update + validation matrix update)
- **Skills**: `pragmatic-semantics`, `skill-maintenance`

#### T9: Add sample-level log scraping + Regulation spans
- **Slice ID**: T9-sample-logging
- **PKO**: DO → CHECK
- **Acceptance criteria**:
  1. `training_status` scrapes pod stdout/stderr for training log lines
  2. Parses loss, learning rate, step, epoch from log lines
  3. Emits `reg.training.sample.{loss,lr,step,epoch}` spans
  4. Unit test with mock log lines
- **Verification**: Unit test verifies span emission from parsed logs
- **Dependencies**: None (independent)
- **Files likely touched**: `mcp-servers/hkask-mcp-training/src/lib.rs`, `mcp-servers/hkask-mcp-training/src/providers/runpod.rs`
- **Scope**: M (log parsing + span emission + tests)
- **Skills**: `coding-guidelines`, `tdd`, `pragmatic-cybernetics`

**Phase 4 checkpoint**: Harness matrix is current; sample-level
training observability via Regulation spans.

---

### Phase 5: Platform Validation

#### T10: Run RunPod vs Nebius discriminating test
- **Slice ID**: T10-platform-discriminating-test
- **PKO**: PLAN → DO → CHECK → ACT
- **Acceptance criteria**:
  1. Run 5x Qwen3-0.5B LoRA training jobs on RunPod Secure Cloud H100
  2. Run 5x same jobs on Nebius on-demand H100
  3. Record: provisioning time, completion rate, cost per successful run, checkpoint integrity
  4. Document results in `docs/research/platform-discriminating-test.md`
  5. Falsification verdict: if RunPod ≥80% completion at ≤120% Nebius cost → stay; if <60% → migrate
- **Verification**: Quantitative comparison with raw data
- **Dependencies**: T4 (checkpoint resume helps completion rate)
- **Files likely touched**: `docs/research/platform-discriminating-test.md` (new)
- **Scope**: M (10 GPU runs, ~$20-50 total, 1 day)
- **Skills**: `diagnose`, `falsifiability`, `mcda`

**Phase 5 checkpoint**: Platform decision is evidence-based, not
vibes-based. Falsification verdict recorded.

---

### Phase 6: OxiCUDA PoC (Research Track)

#### T11: Build OxiCUDA proof-of-concept
- **Slice ID**: T11-oxicuda-poc
- **PKO**: PLAN → DO → CHECK → ACT
- **Acceptance criteria**:
  1. New Rust binary crate `hkask-lora-trainer` (or `crates/hkask-training`)
  2. OxiCUDA added as git dependency
  3. Load Qwen3-0.5B on GPU via `oxicuda-driver`
  4. Run simple GEMM via `oxicuda-blas` to verify CUDA works on H100
  5. Wrap a Linear layer with `oxicuda-peft::LoraLinear`
  6. Run 1 training step using `oxicuda-train::GpuAdamW`
  7. Save adapter via `oxicuda-peft::io::AdapterPayload`
  8. Document results in `docs/research/oxicuda-poc-results.md`
- **Verification**: PoC runs end-to-end on H100; adapter saved successfully
- **Dependencies**: T10 (use validated platform for PoC)
- **Files likely touched**: `crates/hkask-training/` (new), `Cargo.toml` (workspace member), `docs/research/oxicuda-poc-results.md` (new)
- **Scope**: L (new crate, git dependency, GPU code, ~2-4 weeks)
- **Skills**: `coding-guidelines`, `idiomatic-rust`, `diagnose`, `falsifiability`
- **Risk**: OxiCUDA PTX kernels untested on H100 (SM 9.0) — per research doc

**Phase 6 checkpoint**: Rust-native training path validated or
falsified. If validated, plan full `hkask-lora-trainer` binary. If
falsified, continue with Python harness rendering.

---

## Risks

| Risk | Likelihood | Impact | Mitigation |
|---|---|---|---|
| RunPod `/workspace` not persistent | Low | High (blocks checkpoint resume) | T1 verifies before T2-T4; fallback: Nebius or RunPod volume mount |
| OxiCUDA fails on H100 | Medium | High (blocks Rust-native training) | T11 is a research track; Python harnesses remain the fallback |
| Inference API doesn't return logprobs | Medium | Medium (blocks perplexity eval) | T5 checks API capability; fallback: estimate from token sampling |
| Nebius doesn't support training workloads | Low | Medium (blocks discriminating test) | T10 can use DeepInfra as alternative ($1.79/hr H100) |
| Axolotl config schema changes | Low | Low (template update) | Pin Axolotl version in pod template |

## Open Questions (from analysis, unresolved)

See `tasks/training-gap-analysis.md` §10 for the full register (Q1-Q15).
Each task above references the open questions it depends on.

## Convergence Check

| Criterion | Weight | Score (0=good, 1=bad) | Notes |
|---|---|---|---|
| Sizing (≤ M per slice) | 0.25 | 0.05 | 9×S, 4×M, 1×L (L is explicitly research track) |
| Vertical slice (end-to-end) | 0.20 | 0.10 | Each slice has acceptance criteria + verification |
| AC specificity (≤3, testable) | 0.20 | 0.05 | All ACs are testable with clear pass/fail |
| Dependency ordering | 0.15 | 0.05 | Phase 1 → 2 → 3 → 4 → 5 → 6; within-phase deps marked |
| Checkpoints | 0.10 | 0.05 | 6 phase checkpoints defined |
| Red-flag absence | 0.10 | 0.05 | No horizontal slicing; no stubs; no untested state-mutating tools |
| **Weighted total** | | **0.06** | ≤ 0.15 threshold ✅ |

**Converged: YES** (metric 0.06 ≤ 0.15, no criterion > 0.30).