# Training Gap Closure TODO

> **Derived from**: `tasks/training-gap-plan.md`
> **Date**: 2026-07-22

## Phase 1: Checkpoint Resume (Focus Obstacle)

### T1: Verify RunPod `/workspace` persistence
- [ ] Launch RunPod H100 pod (Secure Cloud, ~$0.50 for 30 min)
- [ ] Write test file to `/workspace/persistence-test.txt`
- [ ] Stop pod (not terminate)
- [ ] Restart pod
- [ ] Check if `/workspace/persistence-test.txt` survives
- [ ] Document result in `docs/research/runpod-workspace-persistence.md`
- [ ] **Checkpoint**: persistence verified or refuted

### T2: Add `resume_from_checkpoint` to `TrainingParams`
- [ ] Add `resume_from_checkpoint: Option<String>` to `TrainingParams` in `types.rs`
- [ ] Add serde serialize/deserialize (derive or impl)
- [ ] Write unit test for JSON round-trip
- [ ] `cargo test -p hkask-mcp-training` — pass
- [ ] `cargo clippy -p hkask-mcp-training -- -D warnings` — clean

### T3: Wire through Axolotl config template
- [ ] Update `AxolotlHarness::render_config` to include `resume_from_checkpoint` when set
- [ ] Update `LudwigHarness::render_config` similarly
- [ ] Update TRL script rendering if applicable
- [ ] Write unit test: field present when set, absent when None
- [ ] `cargo test -p hkask-mcp-training` — pass

### T4: Detect pod restart + emit resume span
- [ ] Add restart detection logic to `training_status` (status transition tracking)
- [ ] Emit `reg.training.checkpoint.resume` tracing span on detected restart
- [ ] Write unit test with mock status sequence (running → stopped → running)
- [ ] `cargo test -p hkask-mcp-training` — pass
- [ ] `cargo clippy -p hkask-mcp-training -- -D warnings` — clean
- [ ] **Phase 1 checkpoint**: pod restart no longer loses training state

## Phase 2: Eval Harness Expansion

### T5: Add perplexity evaluation
- [ ] Check if inference API returns logprobs (resolve Q7)
- [ ] Add `method: "perplexity"` option to `training_evaluate`
- [ ] Implement perplexity computation from logprobs (or estimate)
- [ ] Add `perplexity` field to eval result JSON
- [ ] Write unit test with mock inference response
- [ ] `cargo test -p hkask-mcp-training` — pass

### T6: Add benchmark eval scaffold (MMLU-style)
- [ ] Add `benchmark: Option<String>` to `TrainEvaluateRequest`
- [ ] Implement MMLU-style multiple-choice prompt formatting
- [ ] Implement scoring (exact match on answer letter)
- [ ] Return per-category + aggregate accuracy
- [ ] Write unit test with 5-sample mock benchmark
- [ ] `cargo test -p hkask-mcp-training` — pass
- [ ] **Phase 2 checkpoint**: 5 eval methods supported

## Phase 3: Config Pass-Through

### T7: Expose optimization fields in `TrainingParams`
- [ ] Add `flash_attention: Option<bool>` to `TrainingParams`
- [ ] Add `gradient_checkpointing: Option<bool>`
- [ ] Add `bf16: Option<bool>`
- [ ] Add `sample_packing: Option<bool>`
- [ ] Add `deepspeed_config: Option<String>`
- [ ] Wire all 5 through `AxolotlHarness::render_config`
- [ ] Wire all 5 through `LudwigHarness::render_config`
- [ ] Wire applicable fields through `TrlHarness::render_config`
- [ ] Write 15 unit tests (5 fields × 3 harnesses)
- [ ] `cargo test -p hkask-mcp-training` — pass
- [ ] `cargo clippy -p hkask-mcp-training -- -D warnings` — clean
- [ ] **Phase 3 checkpoint**: optimization params exposed as first-class fields

## Phase 4: Harness Matrix & Skill Update

### T8: Update stale harness capability matrix
- [ ] Update `docs/reference/lora-training-catalog.md`: Axolotl >SFT
- [ ] Update: TRL GRPO no longer "deferred"
- [ ] Add Unsloth as 4th harness in matrix
- [ ] Update `lora-training` SKILL.md G6 description
- [ ] Update `lora_validation.rs` G-H1 if matrix changed
- [ ] `cargo test -p hkask-mcp-training` — pass

### T9: Add sample-level log scraping + spans
- [ ] Add log line parser (loss, lr, step, epoch regex)
- [ ] Integrate into `training_status` poll loop
- [ ] Emit `reg.training.sample.{loss,lr,step,epoch}` spans
- [ ] Write unit test with mock Axolotl log lines
- [ ] `cargo test -p hkask-mcp-training` — pass
- [ ] **Phase 4 checkpoint**: matrix current + sample observability

## Phase 5: Platform Validation

### T10: RunPod vs Nebius discriminating test
- [ ] Prepare test config: Qwen3-0.5B, 100 samples, 3 epochs, LoRA r=16
- [ ] Run 5× on RunPod Secure Cloud H100
- [ ] Run 5× on Nebius on-demand H100
- [ ] Record: provisioning time, completion rate, cost/success, checkpoint integrity
- [ ] Document in `docs/research/platform-discriminating-test.md`
- [ ] Falsification verdict: ≥80% RunPod completion → stay; <60% → migrate
- [ ] **Phase 5 checkpoint**: platform decision evidence-based

## Phase 6: OxiCUDA PoC (Research Track)

### T11: Build OxiCUDA proof-of-concept
- [ ] Create `crates/hkask-training/` with `Cargo.toml`
- [ ] Add OxiCUDA as git dependency
- [ ] Implement Qwen3 weight loader (~100 lines, based on LLaMA loader)
- [ ] Load Qwen3-0.5B on GPU via `oxicuda-driver`
- [ ] Run GEMM via `oxicuda-blas` — verify CUDA on H100
- [ ] Wrap Linear with `oxicuda-peft::LoraLinear`
- [ ] Run 1 training step via `oxicuda-train::GpuAdamW`
- [ ] Save adapter via `oxicuda-peft::io::AdapterPayload`
- [ ] Document in `docs/research/oxicuda-poc-results.md`
- [ ] `cargo build -p hkask-training` — pass
- [ ] `cargo clippy -p hkask-training -- -D warnings` — clean
- [ ] **Phase 6 checkpoint**: Rust-native training validated or falsified