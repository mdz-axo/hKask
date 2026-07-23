# Training Gap Closure TODO — FINAL

> **Date**: 2026-07-23
> **OxiCUDA**: DEPRECATED — not a viable option (unverified repo, no real adoption)

## Phase 0: Fix Completion Detection — ✅ CODE COMPLETE

- [x] T0a: Install script writes manifest locally + uploads to HuggingFace
- [x] T0b: `training_status` detects completion via HuggingFace manifest
- [x] T0c: `completion_metadata` parsed from manifest
- [x] T0d: Adapter path from manifest's adapter.repository
- [ ] T0e: End-to-end smoke test (DeepInfra out of B200 capacity; Nebius needs CLI auth check)

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

## Phase 5: Three-Host Architecture — ✅ CODE COMPLETE

- [x] T5a: `TrainingHost` trait redesigned — `status()` returns `PodStatus` with SSH, IP, uptime, GPU type, fail_reason
- [x] T5b: `RunpodHost` — `cloudType: SECURE`, SSH info in status, restart detection
- [x] T5c: `DeepInfraHost` — REST API at `api.deepinfra.com/v1/containers`, B200 at $3.69/hr
- [x] T5d: `NebiusHost` — `nebius` CLI, H100 at $2.95/hr, `--parent-id` for resource creation
- [x] T5e: `TrainingHostId` enum extended with `DeepInfra` and `Nebius` variants
- [x] T5f: `create_host()` auto-detects from env vars; `HKASK_TRAINING_HOST` overrides
- [x] T5g: `TrainingHostConfig::default()` consistent with `lib.rs::run()` auto-detection
- [x] T5h: `PodStatus.fail_reason` field added — surfaces provider failure reasons (e.g. "out of capacity")
- [x] T5i: `status` tool surfaces `fail_reason` in response JSON
- [x] T5j: DeepInfra container ID extraction fixed (API returns `container_id` field)
- [x] T5k: Nebius `--parent-id` wired to `project_id` for disk + VM creation
- [x] T5l: Nebius `vm_names` dead field removed
- [x] T5m: Nebius `is_public_ip` conditional on actual IP (was hardcoded `true`)
- [x] T5n: Cost estimates updated (DeepInfra B200 $3.69/hr, Runpod H100 $2.39/hr, Nebius H100 $3.85/hr)
- [x] T5o: `.env` updated with `NEBIUS_PROJECT_ID` and `NEBIUS_SUBNET_ID`
- [x] T5p: Research doc corrected — DeepInfra offers B200 only (not H100 as originally researched)
- [x] T5q: Smoke test updated — auto-detects host, supports all three providers

## Phase 6: OxiCUDA PoC — ❌ DEPRECATED

OxiCUDA is not a real option. The repo is unverified, claims are unconfirmed, and
Rust-native GPU training is not a viable path for this project. Python harness
rendering (Axolotl/TRL/Ludwig) remains the production training path.

## Refactor — ✅ COMPLETE

- [x] lib.rs split: 1742 → 505 lines, tools in `tools/` submodule
- [x] Pre-existing syntax errors in `hkask-inference/chat_protocol.rs` fixed

## Remaining: Live Smoke Tests

- [ ] DeepInfra smoke test — blocked by B200 capacity ("Start failed: out of capacity")
      Retry when capacity is available. Error handling verified: `fail_reason` surfaced.
- [ ] Nebius smoke test — requires verified CLI auth + project/subnet configuration
- [ ] Runpod smoke test — needs valid template `zbmjdlkqit` (hkask-axolotl-sft)
- [ ] Verify `training_status` returns `Completed` with adapter registered
- [ ] Document results

## Build Status — ✅ CLEAN

- [x] `cargo build -p hkask-mcp-training` — no warnings
- [x] `cargo clippy -p hkask-mcp-training --no-deps --tests -- -D warnings` — clean
- [x] `cargo test -p hkask-mcp-training` — 20/20 tests pass