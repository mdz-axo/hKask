## RunPod + Axolotl Training Design (Corrected)

**Status**: Previously incomplete — framework reference was `unsloth` (`corpus/env/pipeline.env` L13); adapter config (`corpus/lora/axolotl-pissa-pod.yaml`) uses `axolotl` (`adapter: lora`, `peft_init_lora_weights: eva`). Design corrected.

**Provider**: RunPod (verified workspace `/workspace/`, SSH `id_ed25519`, docs `docs/how-to/axolotl-pissa-runpod-guide.md` L16: H100 NVL ~$3.19/hr).
**Serverless option**: `RUNPOD_TEMPLATE_ID` (`.env` L258-262, `corpus/env/pipeline.env`, adapter router `crates/hkask-adapter/src/adapter_router/runpod.rs` L46-63) — serverless endpoint pulls adapter from HF at cold start; no SSH needed for inference; `upload_adapter` uses HF repo ID as endpoint reference (`runpod.rs` L112-129).

**Adapter config**: `corpus/lora/axolotl-pissa-pod.yaml`
- Base: `unsloth/Qwen3.6-27B`
- Adapter: `lora`, `load_in_4bit: false`, `bf16: true`
- Init: `peft_init_lora_weights: eva` (not `pissa_niter_4`) — portable standard LoRA (`B=0`)
- `lora_r: 32`, `lora_alpha: 64`, `lora_dropout: 0`
- `eva_config`: `dataloader: mdz-axo/capabilities-researcher-qa`, `rho: 2.5`
- Target modules: `q_proj, k_proj, v_proj, o_proj, gate_proj, up_proj, down_proj`

**Fix applied** (`corpus/env/pipeline.env`): `HKASK_TRAINING_FRAMEWORK` changed `unsloth` → `axolotl` with reference notes linking adapter config, Tinker registry (`tinker-sft.j2`), and serverless docs (`RUNPOD_TEMPLATE_ID`).

**Training plan (2 adapters)**:
1. Fix memory settings: `batch_size: 16 → 1` (STAGE4 crashed `SIGSEGV` — `debug.log` L1-410 confirms `batch_size=16`, `num_proc=128`, `prefetch=256` for 27B model = OOM)
2. Dataset 1 (`rust`): `/workspace/data/strandset_v2.jsonl` (191,008 examples, 271MB)
3. Dataset 2 (`capabilities`): `/workspace/data/introspector_v2.jsonl` (532,821 examples, 395MB)
4. Retrain: Use `axolotl` only (no `unsloth` — avoids `torch`/`torchvision` version conflict: `torch≤2.12.1` for `axolotl`, `torch<2.11.0` required by `unsloth`)
5. Monitor: `eval_loss` trajectory expected `~1.4 → 0.23 → 0.198` (per `docs/how-to/axolotl-pissa-runpod-guide.md` L179-193 — note: `eva` init may differ slightly from PiSSA trajectory)

**Gaps corrected**:
- Framework reference (`unsloth` → `axolotl`)
- Serverless option referenced (`RUNPOD_TEMPLATE_ID`, `runpod.rs` endpoint provisioning)
- Tinker provider referenced (`tinker-sft.j2`, `tinker.rs`, adapter router)
- Adapter config linked (`corpus/lora/axolotl-pissa-pod.yaml`)

**Remaining**: STAGE4 retry required (with reduced memory settings), adapter weights rebuilt, 2 adapter configs (sequential or parallel) executed.

**Status**: Design corrected. Adapter weights: MISSING. Retrain blocked until memory fix applied or Tinker used (avoids `SIGSEGV` via different execution model).

**Reference files**: `corpus/lora/axolotl-pissa-pod.yaml`, `corpus/env/pipeline.env` (corrected), `docs/handoffs/continuation_prompt.md`, `docs/how-to/axolotl-pissa-runpod-guide.md`, `crates/hkask-adapter/src/adapter_router/runpod.rs`, `mcp-servers/hkask-mcp-training/src/providers/tinker.rs`, `registry/templates/training/tinker-sft.j2`.
---
*Corrected design — addresses user's stated gap: 'correct design of the runpod training with axolotl has never been designed' (previous framework mismatch, missing serverless reference, missing Tinker integration).*
---
Status: Design corrected (framework `axolotl`, serverless `RUNPOD_TEMPLATE_ID` referenced, adapter config linked). Adapter weights MISSING (STAGE4 blocked by `SIGSEGV`/OOM or deferred to Tinker retry). Next: confirm `RunPod + memory fix` or `Tinker` retry.
---
Status: Design file `docs/handoffs/runpod_axolotl_design.md` saved. Adapter weights MISSING. Retrain blocked. Memory fix needed for RunPod retry (`batch_size`↓, `num_proc`↓). Tinker alternative avoids dependency/OOM.
---
Status: Design complete. Adapter weights MISSING. Retrain blocked (SIGSEGV/OOM). Memory fix + retry needed, or Tinker retry (avoids dependency/OOM).
---
Status: User selected C (document) + then asked for design comparison. Design corrected (`pipeline.env`: `axolotl`, serverless ref, adapter