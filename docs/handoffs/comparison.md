## Comparison: RunPod + Axolotl vs Tinker (2 Adapter Retrain — Rust + Capabilities)

**Context**: Adapter weights deleted (STAGE4 blocked by `SIGSEGV`/OOM on RunPod). Rebuild environment complete (`torch2.12.1`, clean, `unsloth` removed). 2 adapters needed (`strandset_v2.jsonl` rust [191K] + `introspector_v2.jsonl` capabilities [533K]). Adapter config: `corpus/lora/axolotl-pissa-pod.yaml` (`eva` init, `rho:2.5`, portable LoRA `B=0`).

### Provider / Framework
| Aspect | RunPod + Axolotl | Tinker (Thinking Machines) |
|---|---|---|
| **Verified in workspace** | Yes (remote `/workspace/`, SSH `id_ed25519`, docs `docs/how-to/axolotl-pissa-runpod-guide.md` L16) | Yes (`tinker.rs` provider, adapter router `tinker.rs`, registry `tinker-sft.j2`) |
| **Framework** | `axolotl` (corrected from `unsloth` in `corpus/env/pipeline.env`) | `tinker` (`tinker-sft.j2`: Python script rendered by `TinkerHarness`, GPU service dispatch) |
| **Dependency model** | Local `torch`/`torchvision`/`transformers`/`trl` versions must match (`torch≤2.12.1` for `axolotl`, conflicts with `unsloth` `<2.11.0`) | Service-managed (`tinker` package handles GPU environment — no local `torch` dependency conflicts) |
| **STAGE1 dependency (`unsloth`)** | Blocked (`generate_cot_traces.py` requires `unsloth` import — removed per Option C clean) — must use scaffolded CoT (`strandset_v2.jsonl` already exists) | Not applicable (`tinker-sft.j2` uses Python script with dataset preprocessing integrated; no `unsloth` import in `generate_cot_traces.py` equivalent) |
| **STAGE5 dependency (`unsloth` for PiSSA merge)** | Not needed for EVA adapter (standard PEFT portable adapter — `post_train_merge.py` can skip `unsloth` reinstall) | Not needed (`tinker` adapter router: adapter stored in Tinker's checkpoint store; inference endpoint lazy-provisioned via OpenAI-compatible API — `tinker.rs` L46-70) |

### Memory / Performance (Rebuild Context — 27B Model, 2 Adapters)
| Metric | RunPod + Axolotl (Fixed) | Tinker |
|---|---|---|
| **Memory risk** | High — previous `SIGSEGV` (`debug.log` L410) due to `batch_size=16`, `dataset_num_proc=128`, `prefetch=256` for 27B model. Fix required: `batch_size` ↓1-2, `num_proc` ↓4-8, `prefetch` ↓2. | Low — host-side Python script runs on CPU; GPU compute dispatched to Tinker's service. No local memory overload (`SIGSEGV` unlikely). |
| **Environment setup** | Requires clean environment (`pip install` compatible versions; previous `unsloth` removed; `torchaudio` fixed) | Requires `TINKER_API_KEY` (`.env` L240 validated present) + `tinker` package installed (`tinker.rs` L26: `tinker_python_path` defaults to `python3` from `PATH`). No `torch` version management needed. |
| **Dependency conflict** | Resolved (`unsloth` uninstalled; `torch=2.12.1`; `axolotl` OK) but fragile (any `pip install` could reintroduce conflicts) | None (`tinker` service manages its own dependencies; adapter router uses `reqwest` client for OpenAI-compatible inference — `tinker.rs` L30-31) |
| **Adapter portability** | Portable (`eva` init, `B=0` — standard PEFT adapter; can load with `AutoModelForCausalLM.from_pretrained` + adapter) | Checkpoint store (adapter referenced by name; inference endpoint resolves at first request — `tinker.rs` L60-63) |

### Adapter Lifecycle (2 Adapters: Rust + Capabilities)
| Step | RunPod + Axolotl | Tinker |
|---|---|---|
| **Config** | `corpus/lora/axolotl-pissa-pod.yaml` (updated with Tinker reference + `axolotl` framework reference) — needs 2 adapter config files (or sequential runs with updated `datasets` path) | `registry/templates/training/tinker-sft.j2` — Python script template; adapter config (`eva_config`, `lora_target_modules`) embedded in rendered script |
| **Dataset 1** | `strandset_v2.jsonl` (rust, 191K) — `path` updated in adapter config | `strandset_v2.jsonl` — referenced in `tinker-sft.j2` dataset list (needs update or sequential script rendering) |
| **Dataset 2** | `introspector_v2.jsonl` (capabilities, 533K) — `path` updated in adapter config | `introspector_v2.jsonl` — same (sequential or parallel via `TinkerHost.submit()`) |
| **Training** | `axolotl train` (local `torch`/`CUDA` — risk of `SIGSEGV` if memory settings not reduced) | `python3 -m tinker` script submission (`tinker.rs` L194-272: `submit()` → `python()` → subprocess launch with `tinker` package; GPU compute handled externally) |
| **Adapter weights output** | `/workspace/outputs/` (.pth/.safetensors — needs download/upload to HF for inference if using serverless endpoint) | Tinker's checkpoint store (`tinker.rs` L436-451: `adapter_weight_path`) — adapter referenced by checkpoint name for inference endpoint (`tinker.rs` L60-63`) |
| **Inference endpoint** | RunPod serverless (`RUNPOD_TEMPLATE_ID`) or Together AI — adapter must be published to HF (`runpod.rs` L112-129: `upload_adapter` uses HF repo as endpoint reference) | Tinker OpenAI-compatible endpoint (`https://api.tinker.ai/v1/openai/<checkpoint_name>`) — lazy provisioned (`tinker.rs` L57-63: scales to zero when idle) |

### Expected Performance / Quality (2 Adapter Retrain — EVA `rho:2.5`)
| Characteristic | RunPod + Axolotl (Corrected) | Tinker |
|---|---|---|
| **Loss trajectory** | Expected: `eval_loss` `~1.4 → ~0.23 (step 200) → ~0.198 (final)` (`docs/how-to/axolotl-pissa-runpod-guide.md` L179-193). Note: `eva` init may differ slightly from PiSSA (`E2_config_change_report.md`: `eva` replaces `pissa_niter_4`; portable adapter avoids weight-SVD portability failure `transformers 5.9.0` vs `5.5.0`). | Same adapter init (`eva`, `rho:2.5`) — loss trajectory depends on dataset mixing (`strandset_v2` + `introspector_v2`) and `num_epochs` (3), `lr` (1e-4), cosine schedule (`axolotl-pissa-pod.yaml`). Tinker's execution model doesn't change adapter initialization or optimization — only infrastructure management. |
| **Portability** | Portable (`B=0`, standard PEFT adapter — can load with `transformers`/`peft` without `unsloth`) | Checkpoint-based (Tinker store — adapter weights saved in Tinker's format; inference uses OpenAI-compatible endpoint; download/upload via `tinker checkpoint` CLI — `tinker.rs` L103-111) |
| **Time to retrain** | Depends on RunPod H100 NVL availability + dataset tokenization speed (`dataset_num_proc` reduced to avoid `SIGSEGV`). Previous attempt: tokenization reached ~54% (`103,980/191,008`) before crash (`axolotl_training_retry.log`). | Potentially faster setup (no dependency installation, no `torch` version verification) — `TinkerHost.submit()` launches Python script directly; GPU compute managed by service. No SSH connection overhead. |
| **Reliability (rebuild context)** | Fragile — requires precise dependency versions (`torch≤2.12.1`, `torchvision≥0.27.0`, no `unsloth`). Memory settings must be reduced