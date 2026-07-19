# Post-Mortem — Training Provider Cleanup (2026-07-19)

## Summary

~$1,000 spent on training providers (RunPod $600, inference/design $400) with zero useful adapter output. Root cause: a constellation of fabricated API contracts, in-memory-only pod tracking, non-propagated lessons, and untested speculative tools — all symptoms of agent-generated code without end-to-end verification.

## Cost breakdown

| Provider | Spend | Root cause |
|---|---|---|
| RunPod | ~$600 | In-memory `jobs` map (`Arc<Mutex<HashMap>>`) lost pod_ids on process restart; orphaned pods kept billing at H100 hourly rate with no way to cancel. The `cancel` method correctly called `podTerminate` — the leak was persistence, not the cancel logic. |
| Inference + design | ~$400 | Iterating on RunPod pod configurations, inference testing, and agent-driven design thrashing (rewriting the same handoff doc 4+ times without making progress). |

## Findings (5 HIGH, fixed)

### H1: Tinker inference URL fabricated
- **Bug**: `crates/hkask-adapter/src/adapter_router/tinker.rs:62` built `https://api.tinker.ai/v1/openai/{model_name}` — this domain does not exist. The real Tinker OpenAI-compatible endpoint is `https://tinker.thinkingmachines.dev/services/tinker-prod/oai/api/v1` with `tinker://<run_id>/sampler_weights/<step>` as the model path (not appended to the URL).
- **Additional**: `AdapterSource` had only a `HuggingFace { repo }` variant — no `Tinker { checkpoint_path }` variant to carry the `tinker://` URI. The backend was a pass-through abstraction (P4 violation) that could never have worked.
- **Fix**: Deleted the Tinker inference adapter backend entirely. Tinker remains a training host; inference of Tinker-trained adapters goes through download → HuggingFace upload → Together/Runpod inference. Regression test `tinker_inference_backend_is_not_registered` added to prevent re-introduction.

### H2: RunPod training pods not persisted — $600 billing leak
- **Bug**: `RunpodHost.jobs` was `Arc<Mutex<HashMap<String, String>>>` (in-memory only). Process restarts lost all pod_ids; pods kept billing on RunPod with no way to cancel them.
- **Fix**: Added JSON file persistence (`data/training-pods.json`, configurable via `HKASK_PODS_FILE`). Pod IDs are loaded on startup and persisted atomically (temp + rename) after every `submit` and `cancel`. Added `drain_all_pods()` method for graceful shutdown — terminates all known pods via GraphQL `podTerminate`.

### H3: Together AI poll always timed out + wrong JSON field
- **Bug 1**: `poll_until_complete` had a 5-minute ceiling (30 attempts × 10s). Real fine-tune jobs take 26-55h. Every real upload returned `did not complete within 30 attempts`.
- **Bug 2**: Code read `json["model_name"]` / `json["output_name"]`; the real field is `model_output_name` (per Together AI OpenAPI schema). Every successful fine-tune recorded `model_name: "unknown"`, breaking subsequent inference.
- **Fix**: Made timeout configurable via `TOGETHER_POLL_MAX_ATTEMPTS` (default 720 = 2h) and `TOGETHER_POLL_INTERVAL_SECS` (default 30). Fixed field reading to `model_output_name` with fallbacks.

### H4: Tinker training script gradient accumulation broken
- **Bug (Rust harness)**: `harness.rs:541-560` — outer loop stepped by `BATCH_SIZE * GRAD_ACCUM` but only sliced `examples[i:i+BATCH_SIZE]`, skipping `(GRAD_ACCUM-1) * BATCH_SIZE` examples per iteration. Only `1/GRAD_ACCUM` of the dataset was touched per epoch.
- **Bug (Jinja template)**: `tinker-sft.j2:111-135` — loop structure was correct (no skipped examples) but called `forward_backward` GRAD_ACCUM times then `optim_step` once. Per Tinker docs, `optim_step` uses gradients from the last `forward_backward` only — so GRAD_ACCUM-1 calls were wasted compute.
- **Fix**: Tinker's `forward_backward(data: List[Datum])` accepts a list. Both the Rust harness and the Jinja template now pass the full `BATCH_SIZE * GRAD_ACCUM` batch to a single `forward_backward` call, then one `optim_step`. No accumulation loop needed.

### H5: PiSSA lesson documented but never propagated
- **Bug**: `docs/how-to/axolotl-pissa-runpod-guide.md:267-283` documented that PiSSA (`pissa_niter_4`) is non-portable across transformers/torch library versions (residual base mismatch → garbage output; SVD conversion ~40-50% relative error/layer). Yet the manifest, the axolotl config, and the Jinja template all still defaulted to PiSSA. Only one config file (`axolotl-pissa-pod.yaml`) was updated to EVA.
- **Fix**: EVA is now canonical. Deleted `axolotl-pissa-config.yaml` (PiSSA). Renamed `axolotl-pissa-pod.yaml` → `axolotl-lora.yaml` (EVA, clean name). Updated manifest (`init: eva`, deleted corrupt `results:` block, `val_set_size: 0.05`). Updated `axolotl-lora.j2` template (removed PiSSA default, EVA canonical). Deleted `corpus/env/pipeline.env` (stale config with wrong hyperparameters: rank=64, patience=5, grad_accum=4 — all explicitly wrong per the lessons).

## Additional fixes

- **RunPod inference teardown**: `crates/hkask-adapter/src/adapter_router/runpod.rs` teardown was HTTP-DELETEing the OpenAI inference URL — wrong (serverless endpoints scale to zero automatically). Fixed to no-op with documentation pointing to `podTerminate` for future dedicated-pod support.
- **Thrashed handoff doc**: `docs/handoffs/runpod_axolotl_design.md` restated "Adapter weights MISSING. Retrain blocked" 4+ times — classic agent thrashing signature. Deleted.

## What would have prevented this

1. **Contract tests at provider HTTP seams.** 18 of 21 MCP tools had no contract tests. The 3 tested tools were all read-only. Every state-mutating tool was untested. A contract test that simply asserted `resolve_backend(ProviderId::Tinker)` returns `Ok(_)` would have caught H1 immediately (the backend was registered but non-functional). A test that mocked the Together API response and checked the parsed `model_name` would have caught H3.
2. **Pod ID persistence.** The adapter router already persisted endpoints to SQLite (`active_endpoints` table). The training host should have followed the same pattern from day one. The in-memory `HashMap` was a shortcut that cost $600.
3. **Lesson propagation discipline.** The PiSSA portability failure was discovered, documented, and then ignored. The manifest, config, and template were never updated. A checklist item after every "lessons learned" section: "update all config sources that reference this lesson."
4. **End-to-end verification before spending.** No training run was ever verified end-to-end (submit → status → adapter registration → deploy → infer → teardown) before spending real money. Each provider path should have been verified with a $5 test run before committing to $600+ of compute.

## Verification

- `cargo clippy -p hkask-adapter -- -D warnings`: clean
- `cargo test -p hkask-adapter`: 47 passed, 0 failed, 7 ignored (live tests)
- `cargo clippy -p hkask-mcp-training -- -D warnings`: clean
- `cargo test -p hkask-mcp-training`: 16 passed, 0 failed

## Remaining work

- Semantic-graph-audit on the 21-tool MCP surface → cut to 14 tools (delete 7 speculative/inference/dup tools)
- Add CNS spans (`cns.training.provider.{runpod,tinker,together}.*`) for observability
- Write contract tests for the 14 surviving tools (starting with `submit`, `status`, `deploy`, `teardown`)
- Update `mcp-servers/hkask-mcp-training/README.md` after tool simplification