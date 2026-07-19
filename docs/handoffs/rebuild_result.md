## Rebuild Result — Option C (Document Failure)

**Status**: Rebuild complete; adapter retrain blocked; adapter weights MISSING.

**Rebuild (3a — EVA + Axolotl, no unsloth)**: COMPLETE
- `torch`: 2.12.1+cu130 (✓)
- `axolotl`: import OK (✓)
- `unsloth`/`unsloth-zoo`: uninstalled (✓)
- Conflicts: resolved (✓)
- Adapter config: preserved (`corpus/lora/axolotl-pissa-pod.yaml`: `eva`, `rho: 2.5`)

**Adapter weights**: DELETED / MISSING (`/workspace/outputs/` empty before; only `debug.log` after retry attempts).

**STAGE4 retrain attempts**: 2 failures
- `PID 3427`: `ImportError` (torchaudio mismatch) → FIXED (`torchaudio>=2.7.0` installed)
- `PID 3780`: `SIGSEGV` (signal 11) — memory/OOM crash during tokenization (batch_size=16, num_proc=128, prefetch=256, 27B model)

**Datasets present** (preprocessed):
- `strandset_v2.jsonl` (191,008) — rust/capabilities dataset
- `introspector_v2.jsonl` (532,821) — capabilities/introspector dataset

**2 adapters needed** (per user clarification): rust dataset + capabilities dataset.
**Current config** (`axolotl-pissa-pod.yaml`): single adapter (`mdz-axo/capabilities-researcher-qa`). Needs 2 adapter configs.

**Next to retry**: Fix memory (`batch_size` ↓, `num_proc` ↓, `prefetch` ↓) + create 2 adapter configs (`rust` + `capabilities`) + retry STAGE4.

**Status**: Rebuild environment complete. Adapter weights still MISSING. Retraining blocked by memory/OOM (`SIGSEGV`). Needs memory fix + 2-adapter config before retry.
