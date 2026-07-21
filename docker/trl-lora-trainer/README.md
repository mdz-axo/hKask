# trl-lora-trainer

Minimal Docker image for LoRA fine-tuning with [TRL](https://github.com/huggingface/trl) on [RunPod](https://runpod.io) H100 pods. Pushed to `docker.io/mdzaxo/trl-lora-trainer:latest`.

**Image size: ~130MB uncompressed.** All heavy packages (PyTorch, TRL, PEFT, transformers) are pip-installed at pod startup — **not** baked into the image. This mirrors the `axolotl-lora-trainer` design (Lesson 11 of the RunPod guide).

## Design

This image is the runtime container for hKask's `RunpodHost` training provider when `harness=trl` is selected. It is intentionally minimal and follows the project's tooling policy (Rust-only; no Python in our code):

| Concern | Owner | Where |
|---|---|---|
| TRL Python script generation | Rust `TrlHarness::render_config()` | `mcp-servers/hkask-mcp-training/src/providers/trl_harness.rs` |
| Pod creation + env var passing | Rust `RunpodHost::submit()` | `mcp-servers/hkask-mcp-training/src/providers/runpod.rs` |
| LoRA method + gate selection | `lora-training` skill | `.agents/skills/lora-training/SKILL.md`, `docs/reference/lora-training-catalog.md` |
| Image + entrypoint | This directory | `Dockerfile`, `entrypoint.sh` |

The image contains **only**: `python:3.11-slim` + `pip` + `bash` + `curl` + `git` + `build-essential`. The entrypoint is a single bash script — no Python config generator, no baked-in CUDA toolkit, no PyTorch base image.

## What the entrypoint does

`entrypoint.sh` runs in this order:

1. `pip install trl==1.8.0 peft==0.19.0 transformers==5.9.0 bitsandbytes accelerate liger-kernel huggingface_hub` (pinned versions — Lesson 12)
2. Writes `HKASK_TRL_SCRIPT` env var to `/workspace/train.py`
3. `huggingface-cli login` if `HF_TOKEN` is set
4. `python /workspace/train.py` (TRL pulls base model + dataset from HuggingFace at runtime)
5. `huggingface-cli upload` of the adapter to `HKASK_HF_MODEL_REPOSITORY`
6. Writes a completion manifest (pure bash, no Python) — includes TRL/PEFT/transformers versions
7. `exec sleep infinity` — keeps the pod alive for SSH debugging

If training fails, the entrypoint writes a failure manifest and still `exec sleep infinity`s so the operator can SSH in to inspect logs.

## Version pinning (Lesson 12)

TRL is under active development (v1.0 released March 2026, v1.8.0 current). API changes between versions can produce adapters that don't behave the same at inference. This is the same class of bug as Lesson 6 (PiSSA portability) — version mismatch between training and inference environments.

The entrypoint pins:
- `trl==1.8.0` — the version the `lora-training` skill is anchored to
- `peft==0.19.0` — the PEFT version the skill's method catalog references
- `transformers==5.9.0` — compatible with both TRL and PEFT

The rendered TRL script includes a version assertion that fails fast if the installed TRL version is incompatible. The completion manifest records the exact versions used for training, enabling inference-time compatibility checks.

## Environment variables

Set by `RunpodHost::submit()` (see `runpod.rs`):

| Env var | Required | Purpose |
|---|---|---|
| `HKASK_TRL_SCRIPT` | **yes** | Full TRL Python script, rendered by `TrlHarness::render_config()` |
| `HKASK_JOB_ID` | yes | hKask job id (used in manifest + upload commit message) |
| `HKASK_BASE_MODEL` | yes | HuggingFace base model repo id |
| `HKASK_HF_DATASET_REPOSITORY` | yes | Dataset repo id |
| `HKASK_HF_DATASET_REVISION` | optional | Dataset revision |
| `HKASK_HF_DATASET_PATH` | optional | Path within dataset repo |
| `HKASK_HF_MODEL_REPOSITORY` | optional | Destination adapter repo on HuggingFace |
| `HKASK_COMPLETION_MANIFEST_PATH` | optional | Where to write the completion manifest (default `/workspace/completion.json`) |
| `HF_TOKEN` | optional | HuggingFace token (private repos + adapter upload) |
| `HKASK_OUTPUT_DIR` | optional | Override output dir (default `/workspace/outputs`) |

## lora-training skill integration

The TRL Python script rendered by `TrlHarness::render_config()` carries the LoRA method selection. The `peft_init_lora_weights` field selects the PEFT init method per the skill's method catalog — same as axolotl. The skill's G6 gate (harness capability) recommends `harness=trl` when the operator needs TRL-specific capabilities (assistant_only_loss, packing strategies, VLMs, preference optimization).

The skill's 17 quality gates (G-M1..G-M5, G-Q1..G-Q6, G-D1..G-D3, G-F1..G-F2, G-H1) are enforced **upstream** by the `audit-config` phase before the pod is created — the entrypoint does not re-derive them. See `docs/reference/lora-training-catalog.md` for the full catalog.

## Build & push

```bash
cd docker/trl-lora-trainer
podman build -t docker.io/mdzaxo/trl-lora-trainer:latest .
podman push docker.io/mdzaxo/trl-lora-trainer:latest
```

## What is NOT in this image

- **No CUDA devel base image** — RunPod pods provide the host CUDA driver; TRL pulls GPU wheels at pip install time.
- **No PyTorch base image** — torch is pip-installed as a TRL dependency.
- **No baked-in model weights or datasets** — pulled from HuggingFace at runtime.
- **No Python config generator** — the script is rendered in Rust and passed as `HKASK_TRL_SCRIPT`.
- **No bloated 5GB+ image** — target is <500MB; this image is ~130MB uncompressed.
