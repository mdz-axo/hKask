# Training Gap Closure TODO — FINAL

> **Date**: 2026-07-23
> **OxiCUDA**: DEPRECATED — not a viable option (unverified repo, no real adoption)

## Phase 0: Fix Completion Detection — ✅ CODE COMPLETE

- [x] T0a: Install script writes manifest locally + uploads to HuggingFace
- [x] T0b: `training_status` detects completion via HuggingFace manifest
- [x] T0c: `completion_metadata` parsed from manifest
- [x] T0d: Adapter path from manifest's adapter.repository
- [ ] T0e: End-to-end smoke test (needs valid RunPod template — now configured)

## Phase 1: Template Wiring — ✅ COMPLETE

- [x] T1a: Axolotl template wired (bf16, gradient_checkpointing, flash_attention, sample_packing)
- [x] T1b: TRL + Ludwig templates wired

## Phase 2: Checkpoint Resume — ✅ COMPLETE

- [x] T2a: `auto_resume_from_checkpoints: true` in Axolotl template
- [x] T2b: Pod restart detection via uptime tracking + `reg.training.checkpoint.resume` span

## Phase 3: Harness Matrix Update — ✅ COMPLETE

- [x] T3a: Axolotl matrix updated (supports DPO/KTO/ORPO/GRPO/RM/FullFT, not "SFT only")
- [x] G-H1 gate: Axolotl + trl_trainer warns (not refuses)
- [x] Catalog + SKILL.md updated

## Phase 4: Eval Harness — ✅ COMPLETE

- [x] T4a: Benchmark eval (MMLU-style multiple-choice) in `tools/evaluate.rs`

## Phase 5: Platform Validation — ✅ RESEARCH COMPLETE

- [x] T5a: RunPod post-mortem analysis (pip install bottleneck identified, template found)
- [x] T5b: Nebius comparison (similar pricing, no pre-built templates, not worth switching now)
- [x] Documented in `docs/research/platform-validation-2026-07-23.md`

## Phase 6: OxiCUDA PoC — ❌ DEPRECATED

OxiCUDA is not a real option. The repo is unverified, claims are unconfirmed, and
Rust-native GPU training is not a viable path for this project. Python harness
rendering (Axolotl/TRL/Ludwig) remains the production training path.

## Refactor — ✅ COMPLETE

- [x] lib.rs split: 1742 → 505 lines, tools in `tools/` submodule
- [x] Pre-existing syntax errors in `hkask-inference/chat_protocol.rs` fixed

## Remaining: Smoke Test

- [ ] Run smoke test with pre-built template `zbmjdlkqit` (hkask-axolotl-sft)
- [ ] Verify `training_status` returns `Completed` with adapter registered
- [ ] Document results