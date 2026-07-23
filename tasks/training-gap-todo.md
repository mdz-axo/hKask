# Training Gap Closure TODO — FINAL

> **Date**: 2026-07-23
> **OxiCUDA**: DEPRECATED — not a viable option (unverified repo, no real adoption)

## Phase 0: Fix Completion Detection — ✅ CODE COMPLETE

- [x] T0a: Install script writes manifest locally + uploads to HuggingFace
- [x] T0b: `training_status` detects completion via HuggingFace manifest
- [x] T0c: `completion_metadata` parsed from manifest
- [x] T0d: Adapter path from manifest's adapter.repository
- [ ] T0e: End-to-end smoke test (DeepInfra out of B200 capacity; Nebius verified separately)

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

## Phase 5: Three-Host Architecture — ✅ CODE COMPLETE + VERIFIED

- [x] T5a: `TrainingHost` trait redesigned — `status()` returns `PodStatus` with SSH, IP, uptime, GPU type, fail_reason
- [x] T5b: `RunpodHost` — `cloudType: SECURE`, SSH info in status, restart detection
- [x] T5c: `DeepInfraHost` — REST API, B200 at $3.69/hr, `fail_reason` from API, uptime from `start_ts`
- [x] T5d: `NebiusHost` — `nebius` CLI, H100 at $2.95/hr, `--parent-id` for resource creation
- [x] T5e: `TrainingHostId` enum extended with `DeepInfra` and `Nebius` variants
- [x] T5f: `create_host()` auto-detects from env vars; `HKASK_TRAINING_HOST` overrides
- [x] T5g: `TrainingHostConfig::default()` consistent with `lib.rs::run()` auto-detection
- [x] T5h: `PodStatus.fail_reason` field — surfaces provider failure reasons
- [x] T5i: `status` tool surfaces `fail_reason` in response JSON
- [x] T5j: DeepInfra container ID extraction fixed (API returns `container_id` field)
- [x] T5k: Nebius `--parent-id` wired to `project_id` for disk + VM creation
- [x] T5l: Nebius `vm_names` dead field removed
- [x] T5m: Nebius `is_public_ip` conditional on actual IP
- [x] T5n: Cost estimates updated (DeepInfra B200 $3.69/hr, Runpod H100 $2.39/hr, Nebius H100 $3.85/hr)
- [x] T5o: `.env` updated with `NEBIUS_PROJECT_ID` and `NEBIUS_SUBNET_ID`
- [x] T5p: Research doc corrected — DeepInfra offers B200 only
- [x] T5q: Smoke test updated — auto-detects host, supports all three providers

### Nebius CLI Verification (2026-07-23) — ✅ END-TO-END VERIFIED

- [x] Disk creation from image family `ubuntu24.04-cuda13.0` — verified
- [x] VM creation with `gpu-h100-sxm` platform + `1gpu-16vcpu-200gb` preset — verified
- [x] `--boot-disk-attach-mode read_write` (lowercase) — verified (was `READ_WRITE` bug, fixed)
- [x] Network interfaces JSON format — verified
- [x] `metadata.id` extraction for disk/VM IDs — verified
- [x] `status.state` extraction (nested path, not top-level) — bug found and fixed
- [x] `status.network_interfaces[0].public_ip_address.address` with CIDR stripping — verified
- [x] SSH access to VM — verified (H100 80GB confirmed via nvidia-smi)
- [x] Cloud-init user-data processing — verified
- [x] VM delete (stops all billing) — verified; cancel method updated to use `delete` not `stop`

### B200 + Axolotl Compatibility (Researched 2026-07-23)

- [x] Axolotl officially supports Blackwell (PyTorch 2.9.1 + CUDA 13.0)
- [x] Install script updated to check for pre-installed harness (avoids overwriting GPU PyTorch)
- [ ] DeepInfra `di-cont-ubuntu-torch:latest` image contents unverified (PyTorch/CUDA version unknown)
- [ ] B200 smoke test blocked by DeepInfra capacity ("out of capacity")

## Phase 6: OxiCUDA PoC — ❌ DEPRECATED

OxiCUDA is not a real option. Python harness rendering remains the production path.

## Refactor — ✅ COMPLETE

- [x] lib.rs split: 1742 → 505 lines, tools in `tools/` submodule
- [x] Pre-existing syntax errors in `hkask-inference/chat_protocol.rs` fixed

## Documentation — ✅ COMPLETE

- [x] DIAG-TRAIN-006 class diagram updated (removed TogetherHost/UnslothHarness, added DeepInfraHost/NebiusHost/PodStatus/TrlHarness/LudwigHarness)
- [x] DIAGRAMS_INDEX.md verification date updated
- [x] AGENTS.md activation guide updated (three-host, not RunPod-only)
- [x] Research doc updated with B200 compatibility section
- [x] Nebius CLI verification results documented

## Tests — ✅ COMPLETE

- [x] 145 library tests (including 7 Nebius JSON extraction tests with verified API responses)
- [x] 20 contract tests
- [x] 3 cost estimate tests (Runpod + DeepInfra + Nebius)
- [x] `cargo clippy -p hkask-mcp-training --no-deps --tests -- -D warnings` — clean

## Remaining: Live Smoke Tests

- [ ] DeepInfra B200 smoke test — blocked by capacity. Error handling verified via live API.
- [ ] Nebius H100 smoke test — CLI verified, ready for full training test
- [ ] Runpod H100 smoke test — needs valid template `zbmjdlkqit`
- [ ] Verify DeepInfra image contents (PyTorch version, CUDA version, pre-installed packages)

## Build Status — ✅ CLEAN

- [x] `cargo build -p hkask-mcp-training --tests` — no warnings
- [x] `cargo clippy -p hkask-mcp-training --no-deps --tests -- -D warnings` — clean
- [x] `cargo test -p hkask-mcp-training` — 165 tests pass (145 lib + 20 contract), 9 ignored