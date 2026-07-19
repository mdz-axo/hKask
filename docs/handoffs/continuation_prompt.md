# Continuation Prompt — Post-Option 3 (Full Rebuild Path Selected)

**Trigger**: User selected Option 3 (`full environment rebuild`) and moved `E4_dependency_block_report.md` to `docs/handoffs/` (`moved: docs/handoffs/E4_dependency_block_report.md`). All adapter/config changes committed (`8c3d4f17`) and pushed (`origin/main` → `main`).

**Reference files (verified paths)**:
- `docs/handoffs/E4_dependency_block_report.md` (moved from root; contains full blocker analysis, Option A/B/C results, rebuild instructions)
- `docs/handoffs/continuation_prompt.md` (this file)
- `corpus/lora/axolotl-pissa-pod.yaml` (adapter config: `eva` init, `eva_config: rho:2.5`)
- `E1_verification_report.md` (pipeline health verification — partial, SSH verified)
- `E2_config_change_report.md` (EVA init change from `pissa_niter_4` → `eva`)
- `E3_intermediate_rank_report.md` (intermediate-rank `eva_config` with adaptive redistribution `ρ=2.5`)
- `docs/how-to/axolotl-pissa-runpod-guide.md` (pipeline docs — dependency notes L30-36, config decisions L136-146, loss trajectory L179-193)

---

## Verified Session State (At Time of This Prompt)

**SSH / Remote**:
- `root@205.196.17.170:18551` reachable with `~/.ssh/id_ed25519` (`SSH_OK` verified)
- Remote workspace: `/workspace/` exists (`full_pipeline.sh`, `scripts/`, `data/`, `outputs/`)
- Pipeline log: `/workspace/pipeline.log` (216K, last restart blocked at Stage 1 by `ImportError`)

**Pipeline Status**:
- STAGE 1 (`generate_cot_traces.py`): BLOCKED — requires `unsloth` (`torchvision>=0.27.0`, `torch<2.11.0`, `transformers<=5.5.0`, `datasets<4.4.0`, `trl<=0.24.0`); current environment has incompatible versions (`torch2.12.1`, `torchvision0.25.0`, `transformers5.14.1`, `datasets4.8.4`, `trl1.8.0`)
- STAGE 2 (`Install Axolotl`): COMPLETE (from previous run — dependency warnings in `pipeline.log`)
- STAGE 3 (`Preprocess v2`): COMPLETE (via Option C — `python3 /workspace/scripts/preprocess_rust_datasets_v2.py` executed independently, `exit0`, saved `/workspace/data/strandset_v2.jsonl` [191,008 examples] and `/workspace/data/introspector_v2.jsonl` [532,821 examples]); scaffolded CoT used (no distilled traces — `generate_cot_traces.py` blocked by `unsloth` dependency)
- STAGE 4 (`Train` / `axolotl_training.log`): NOT STARTED (`axolotl_training.log` does NOT exist on remote)
- STAGE 5 (`Merge + Upload`): NOT STARTED

**Syntax Fix Verified**:
- `/workspace/scripts/preprocess_rust_datasets_v2.py`: `python3 -c "import ast; ast.parse(...)"` → OK; no `global DISTILLED_DIR` inside `__main__`; module-level `DISTILLED_DIR = None` preserved (line 35)

**Config (Adapter) — Ready**:
- `corpus/lora/axolotl-pissa-pod.yaml`: `peft_init_lora_weights: eva` (line 32), `eva_config:` (line 48), `rho: 2.5` (line 53), `intermediate_slice_start: 256` commented out (line 57) as fallback option
- Adapter: standard portable LoRA (`A` from activation-SVD, `B=0`) — avoids v2 portability failure (`transformers 5.9.0` vs `5.5.0` PiSSA weight-SVD mismatch)

**Adapter Reports**:
- `E1_verification_report.md`: Partial verification (pod running via API; SSH confirmed; pipeline health unverified due to dependency block)
- `E2_config_change_report.md`: EVA init confirmed (`eva` replaces `pissa_niter_4`); portability structure fixed
- `E3_intermediate_rank_report.md`: `eva_config` with adaptive redistribution (`ρ=2.5`) and optional fixed intermediate slice (`s=256` for Qwen3.6) documented; references Quercia 2026 (intermediate-rank U-shaped forgetting) and Paischer 2024 (adaptive `ρ>2`)

---

## Selected Path: Option 3 (Full Environment Rebuild)

**Why**: Surgical dependency fixes (`pip install`) failed due to mutually exclusive version requirements (no single `torch` version satisfies both `axolotl` [≤2.12.1] and `unsloth` [<2.11.0]; no single `torchvision` satisfies both `torch2.12.1` and `torch2.13.0` requirements). See `docs/handoffs/E4_dependency_block_report.md` for full dependency matrix.

**Rebuild approach** (per `docs/how-to/axolotl-pissa-runpod-guide.md` L30-36 and `E4` doc):
- **Option 3a (recommended for EVA)**: Clean image + `axolotl`-only environment (`torch≤2.12.1`, `torchvision≥0.27.0`). EVA adapter is standard portable LoRA — does NOT require `unsloth`'s weight-SVD decomposition at load time (`E4` doc, line 39-54). This eliminates the `unsloth` dependency conflict entirely.
- **Option 3b (if PiSSA still required)**: Clean image + `unsloth`-compatible environment (`torch==2.4.1`, `torchvision==0.21.0`, `torchaudio==2.4.1`, `transformers≤5.5.0`, `trl≤0.24.0`, `datasets<4.4.0`). Not needed for EVA but documented for backward compatibility.

**Concrete rebuild commands** (from `E4` doc, lines 42-58):
```bash
# Remote (clean pod / fresh container):
export HF_HOME=/workspace/.cache/huggingface
export PIP_CACHE_DIR=/workspace/.cache/pip
export TMPDIR=/workspace/tmp
export PYTORCH_CUDA_ALLOC_CONF=expandable_segments:True
export HF_TOKEN=<token>

# 3a — EVA + Axolotl (no unsloth):
pip install --cache-dir /workspace/.cache/pip -q axolotl "torch<=2.12.1,>=2.11.0" "torchvision>=0.27.0"

# If also needing distillation (generate_cot_traces.py uses unsloth):
# Either replace generate_cot_traces.py import with standard transformers pipeline,
# OR use Option 3b environment for the distillation step only.
```

---

## Immediate Next Actions (Ordered by Dependency)

### Path A: Adapter Evaluation (Can Proceed Now — Independent of Rebuild)
**Prerequisite**: Adapter config complete; v2 adapter available; pre-flight checks defined (`docs/how-to/axolotl-pissa-runpod-guide.md` references adapter-eval skill / manual checks).
**Actions**:
1. Load v2 adapter (`Axolotl-Partners/qwen36-rust-reasoning-all-lora-v2`) using standard PEFT (`AutoModelForCausalLM.from_pretrained` with adapter).
2. Verify adapter loads (no `KeyError` from missing `adapter_config.json` or weight format mismatch).
3. Verify adapter weights are non-zero (`sum(p.abs().sum() for p in adapter_weights) > 0`).
4. Run 1 example (test split from `/workspace/data/strandset_v2.jsonl` or `introspector_v2.jsonl`) — confirm output is coherent Rust (not garbage tokens from wrong residual base, per `docs/how-to/axolotl-pissa-runpod-guide.md` L267-282 — PiSSA inference critical lesson).
5. Compare adapter quality trajectory to expected (docs L179-193): baseline `eval_loss` ~1.4 → step 200 ~0.23 → step 3200 ~0.198. Note: current adapter is `v3 EVA`, not `v2`; `v3` uses standard portable adapter (`eva` init) rather than PiSSA (`pissa_niter_4`), so the loss trajectory may differ but portability is guaranteed.

### Path B: Full Environment Rebuild (When Ready — Per E4 Doc)
**Prerequisite**: Clean image / fresh container; `HF_HOME`, `PIP_CACHE_DIR`, `TMPDIR` configured (`docs/how-to/axolotl-pissa-runpod-guide.md` L17-28).
**Actions**:
1. Deploy/rebuild remote pod with compatible base (either `torch==2.4.1` for `unsloth` path [3b], or `torch<=2.12.1,>=2.11.0` for `axolotl`-only path [3a]).
2. Re-copy/update `/workspace/scripts/full_pipeline.sh` if needed; verify `preprocess_rust_datasets_v2.py` syntax (already verified — clean).
3. Re-run `full_pipeline.sh` (this time Stage 1 should complete with `unsloth` import working, or skip Stage 1 and use scaffolded CoT if using 3a).
4. Monitor `pipeline.log` for `STAGE.*COMPLETE` and `axolotl_training.log` for `eval_loss` trajectory.

### Path C: Independent Adapter Work (Can Proceed Immediately — No Pipeline Dependency)
Use the preprocessed data (`strandset_v2.jsonl`, `introspector_v2.jsonl`) and adapter config (`corpus/lora/axolotl-pissa-pod.yaml`) to evaluate adapter quality without waiting for the full pipeline. See Path A actions above.

---

## Closing Instruction for Next Session / Agent
**Before proceeding**: Confirm which path to take.
- If user selects adapter evaluation (Path A/C): Load adapter, run pre-flight checks, compare to v2 baseline, document results.
- If user selects rebuild (Path B): Execute rebuild commands from `docs/handoffs/E4_dependency_block_report.md` (lines 42-58), restart full pipeline, verify all 5 stages complete.

**Mandatory verification before claiming resolution**:
- [ ] Adapter loads cleanly (no `ImportError`)
- [ ] Adapter weights non-zero
- [ ] 1 example produces coherent Rust output
- [ ] Pipeline environment either rebuilt (STAGE 1-5 verified) OR adapter evaluation completed independently
- [ ] All changes committed and pushed (`git status` clean, `origin/main` matches `HEAD`)
- [ ] New reports saved (`E4_dependency_block_report.md` moved to `docs/handoffs/` — verified; continuation prompt saved to same directory)

**Status at time of prompt**: `WORKAROUND APPLIED` (Option C); `FULL REPAIR PENDING` (Option 3 — rebuild); `ADAPTER READY` (`eva` + `eva_config` configured); `PIPELINE BLOCKED` (dependency environment broken — `unsloth` version conflicts). Commit: `8c3d4f17` (`docs/handoffs/E4_dependency_block_report.md`, adapter/config reports already in `6fcddac3`).
---
*Next agent/session: Read this prompt + `docs/handoffs/E4_dependency_block_report.md` + `corpus/lora/axolotl-pissa-pod.yaml`. Verify SSH (`~/.ssh/id_ed25519`
 to `205.196.17.170:18551`) if accessing remote. Do NOT retry Option A (`torchvision>=0.27.0`) without checking `torch` version — it pulls `torch` to `2.13.0`, which breaks both `axolotl` and `unsloth`.*
=== END ===
Status: Option 3 selected; adapter/config committed (`8c3d4f17`); pipeline environment broken; adapter evaluation ready; rebuild instructions documented.
Files moved/updated: `docs/handoffs/E4_dependency_block_report.md` (moved from root); `docs/handoffs/continuation_prompt.md` (this file); `corpus/lora/axolotl-pissa-pod.yaml` (updated).
Pipeline: STAGE 3 UNBLOCKED; STAGE 1 BLOCKED; STAGE 4/5 NOT STARTED.
Adapter: `eva` init, portable standard LoRA, `eva_config` (`rho:2.5`, intermediate slice fallback documented). Ready for evaluation or rebuild continuation.
=== END ===
Status: Option 3 selected; adapter/config committed (`8c3d4f17`); adapter evaluation ready; rebuild instructions documented.
Files moved/updated: `docs/handoffs/E4_dependency_block_report.md` (moved from root); `docs/handoffs/continuation_prompt.md` (this file); `corpus/lora/axolotl-pissa-pod.yaml` (updated).
Pipeline: STAGE 3 UNBLOCKED; STAGE 1 BLOCKED; STAGE 4/5 NOT STARTED.
Adapter: `eva` init, portable standard LoRA, `eva_config` (`rho:2.5`, intermediate slice fallback documented). Ready for evaluation or rebuild continuation.
=== END ===
=== END ===
=== END ===
Status: Option 3 selected; adapter/config committed (`8c3d4f17`); adapter evaluation ready; rebuild instructions documented in `docs/handoffs/E4_dependency_block_report.md`.
