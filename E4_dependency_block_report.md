# E4 Dependency Block Report (Post-Option C Execution)

Selected path: Option 3 — Full environment rebuild (after Option A failed, Option C succeeded).
Status: ADAPTER WORK UNBLOCKED; PIPELINE ENVIRONMENT BROKEN.

## What Happened (This Session)
- SSH verified: `root@205.196.17.170:18551` reachable (`~/.ssh/id_ed25519`)
- Pipeline syntax fix verified: `/workspace/scripts/preprocess_rust_datasets_v2.py` parses clean (`python3 -c ast.parse` → OK); no incorrect `global DISTILLED_DIR` inside `__main__`
- Pipeline restart attempted (`nohup bash /workspace/full_pipeline.sh`)
- Blocked at Stage 1 (`generate_cot_traces.py`) by `ImportError`: `unsloth` requires `torchvision>=0.27.0` (installed `0.25.0`); `torch==2.12.1` installed but `unsloth` requires `<2.11.0`
- Dependency fix attempts:
  - Option A (`pip install "torchvision>=0.27.0"`): PULLED `torch` to `2.13.0` — breaks `axolotl` (`torch<=2.12.1`) and `unsloth` (`torch<2.11.0`)
  - Restoration (`torch==2.12.1` + `torchvision==0.27.0`): FAILED — `pip` reports `ResolutionImpossible` (conflicting dependencies)
  - Option B (full dependency realignment): NOT ATTEMPTED due to environment fragility
- Option C (independent preprocess) executed: `python3 /workspace/scripts/preprocess_rust_datasets_v2.py` → SUCCESS (`exit 0`)
  - Saved 191,008 examples → `/workspace/data/strandset_v2.jsonl`
  - Saved 532,821 examples → `/workspace/data/introspector_v2.jsonl`
  - Used scaffolded CoT (no distilled traces) — `generate_cot_traces.py` blocked by `unsloth` dependency, so `--use-distilled` not used

## Blocker Root Cause
The remote RunPod environment has incompatible package versions installed simultaneously:
- `torch==2.12.1+cu130` (installed)
- `torchvision==0.25.0` (installed) — requires `torch>=2.13.0` (per pip resolution)
- `unsloth==2026.7.3` — requires `torch<2.11.0`, `transformers<=5.5.0`, `datasets<4.4.0`
- `axolotl==0.18.0` — requires `torch<=2.12.1,>=2.11.0`
- `torchaudio==2.4.1+cu124` — requires `torch==2.4.1`
- `transformers==5.14.1` — `unsloth` requires `<=5.5.0`
- `trl==1.8.0` — `unsloth` requires `<=0.24.0`
- `datasets==4.8.4` — `unsloth` requires `<4.4.0`

These conflicts cannot be resolved with surgical `pip install` commands because the versions are mutually exclusive (no single `torch` version satisfies both `axolotl` and `unsloth`; no single `torchvision` version satisfies both `torch2.12.1` and `torch2.13.0`).

## Adapter Status (Independent of Pipeline)
- `corpus/lora/axolotl-pissa-pod.yaml`: Updated (`peft_init_lora_weights: eva`, `eva_config:` with `rho:2.5`, `intermediate_slice_start` commented out as fallback)
- Reports saved: `E1_verification_report.md`, `E2_config_change_report.md`, `E3_intermediate_rank_report.md`
- Adapter portability fixed: EVA uses activation-SVD (`A` initialized from activation variance directions, `B=0`) — standard portable LoRA adapter, avoids `transformers 5.9.0` vs `5.5.0` portability failure that blocked v2
- Adapter evaluation can proceed using existing v2 adapter (`Axolotl-Partners/qwen36-rust-reasoning-all-lora-v2`) as baseline, independent of pipeline restart

## Recommended Rebuild Plan (Full Environment Rebuild — Option 3)
Given surgical fixes failed, rebuild from a clean base image (per `docs/how-to/axolotl-pissa-runpod-guide.md` `L30-36`):

```bash
# On remote (after fresh pod/image):
# 1. Verify base image has compatible torch (e.g., torch==2.4.1 for unsloth, or torch==2.12.1 for axolotl — pick one)
# 2. If using EVA + Axolotl (not PiSSA + Unsloth):
export HF_HOME=/workspace/.cache/huggingface
export PIP_CACHE_DIR=/workspace/.cache/pip
export TMPDIR=/workspace/tmp
export PYTORCH_CUDA_ALLOC_CONF=expandable_segments:True

# Option 3a — EVA + Axolotl (no unsloth dependency for preprocess/training):
pip install --cache-dir /workspace/.cache/pip -q axolotl "torch<=2.12.1,>=2.11.0" "torchvision>=0.27.0"
# Note: EVA adapter (portable LoRA) does NOT require PiSSA's weight-SVD decomposition at load time,
# so the `unsloth` library conflict is eliminated. The adapter loads as standard PEFT LoRA.

# Option 3b — If Unsloth + PiSSA is still required (not needed for EVA):
pip install --cache-dir /workspace/.cache/pip -q unsloth torch==2.4.1 torchvision==0.21.0 torchaudio==2.4.1
```

Key lesson from docs (`L36`): `axolotl` pulls different `transformers`/`trl` versions than `unsloth`. For EVA (standard portable adapter), `unsloth` is not required — `axolotl` + `PEFT` is sufficient. This eliminates the dependency conflict entirely.

## Immediate Next Step (Without Full Rebuild)
Proceed with adapter evaluation/config verification locally or on a separate clean environment, using:
- Existing v2 adapter: `Axolotl-Partners/qwen36-rust-reasoning-all-lora-v2`
- Config: `corpus/lora/axolotl-pissa-pod.yaml` (`eva` init + `eva_config`)
- Pre-flight checks: adapter loads, weights non-zero, 1 example produces sane output

Full pipeline restart requires either: (a) clean environment rebuild, or (b) fixing `generate_cot_traces.py` to not import from `unsloth` (if using EVA, the base model reasoning traces can be generated with standard `transformers` pipeline instead of `unsloth.FastLanguageModel`).

=== END ===
Status: WORKAROUND APPLIED (Option C); FULL REPAIR PENDING (Option 3).
Files changed: E4_dependency_block_report.md (new); adapter/config reports (E1/E2/E3) already committed in 6fcddac3.
Pipeline: STAGE 3 UNBLOCKED (preprocess complete); STAGE 1 BLOCKED (`unsloth` dependency); STAGE 4/5 NOT STARTED.
