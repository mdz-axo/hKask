# axolotl-lora-trainer

Minimal Docker image for LoRA fine-tuning with [Axolotl](https://github.com/axolotl-ai-cloud/axolotl) on [RunPod](https://runpod.io) H100 pods. Pushed to `docker.io/mdzaxo/axolotl-lora-trainer:latest`.

**Image size: ~129MB uncompressed, ~44MB compressed.** All heavy packages (PyTorch, axolotl, PEFT, transformers) are pip-installed at pod startup — **not** baked into the image. Previous attempts produced 22-31GB images by baking in CUDA toolkits, Unsloth, and Python config generators; this image avoids all three.

## Design

This image is the runtime container for hKask's `RunpodHost` training provider. It is intentionally minimal and follows the project's tooling policy (Rust-only; no Python in our code):

| Concern | Owner | Where |
|---|---|---|
| Axolotl YAML config generation | Rust `AxolotlHarness::render_config()` | `mcp-servers/hkask-mcp-training/src/providers/harness.rs` |
| Pod creation + env var passing | Rust `RunpodHost::submit()` | `mcp-servers/hkask-mcp-training/src/providers/runpod.rs` |
| LoRA method + gate selection | `lora-training` skill | `.agents/skills/lora-training/SKILL.md`, `docs/reference/lora-training-catalog.md` |
| Image + entrypoint | This directory | `Dockerfile`, `entrypoint.sh` |

The image contains **only**: `debian:bookworm-slim` + `python3` + `pip` + `bash` + `curl` + `git` + `build-essential`. The entrypoint is a single bash script — no Python config generator, no Unsloth, no baked-in CUDA toolkit, no PyTorch base image.

## What the entrypoint does

`entrypoint.sh` runs in this order:

1. `pip install axolotl huggingface_hub` (at pod startup — keeps the image small)
2. Writes `HKASK_AXOLOTL_CONFIG` env var to `/workspace/config.yml`
3. `huggingface-cli login` if `HF_TOKEN` is set
4. `axolotl train /workspace/config.yml` (axolotl pulls base model + dataset from HuggingFace at runtime)
5. `huggingface-cli upload` of the adapter to `HKASK_HF_MODEL_REPOSITORY`
6. Writes a completion manifest (pure bash, no Python)
7. `exec sleep infinity` — keeps the pod alive for SSH debugging

If training fails, the entrypoint writes a failure manifest and still `exec sleep infinity`s so the operator can SSH in to inspect logs.

## Environment variables

Set by `RunpodHost::submit()` (see `runpod.rs` lines ~423–574):

| Env var | Required | Purpose |
|---|---|---|
| `HKASK_AXOLOTL_CONFIG` | **yes** | Full axolotl YAML, rendered by `AxolotlHarness::render_config()` |
| `HKASK_JOB_ID` | yes | hKask job id (used in manifest + upload commit message) |
| `HKASK_BASE_MODEL` | yes | HuggingFace base model repo id |
| `HKASK_HF_DATASET_REPOSITORY` | yes | Dataset repo id |
| `HKASK_HF_DATASET_REVISION` | optional | Dataset revision |
| `HKASK_HF_DATASET_PATH` | optional | Path within dataset repo |
| `HKASK_HF_MODEL_REPOSITORY` | optional | Destination adapter repo on HuggingFace |
| `HKASK_COMPLETION_MANIFEST_PATH` | optional | Where to write the completion manifest (default `/workspace/completion.json`) |
| `HF_TOKEN` | optional | HuggingFace token (private repos + adapter upload) |
| `AXOLOTL_OUTPUT_DIR` | optional | Override axolotl output dir (default `/workspace/outputs`) |

The remaining `HKASK_LORA_*`, `HKASK_NUM_EPOCHS`, `HKASK_LEARNING_RATE`, etc. env vars are passed by `RunpodHost::submit()` for observability — they are **already baked into the YAML** by `AxolotlHarness::render_config()`, so the entrypoint does not re-read them.

## lora-training skill integration

The axolotl YAML rendered by `AxolotlHarness::render_config()` carries the LoRA method selection. The `peft_init_lora_weights` field selects the PEFT init method per the skill's method catalog:

| `peft_init_lora_weights` | Method | Gate | Source |
|---|---|---|---|
| `true` (default) | LoRA | G4=default | arXiv:2106.09685 |
| `eva` | EVA | G4=data-driven init | PEFT v0.19.0 `eva_config` |
| `pissa` | PiSSA | G4=fast convergence | arXiv:2404.02948 |
| `lora_ga` | LoRA-GA | G4=fast convergence | arXiv:2407.05000 |
| (plus `peft_use_rslora: true`) | rsLoRA | G3=r>64 | arXiv:2312.03732 |
| (plus `peft_use_dora: true`) | DoRA | G4=cost-sensitive | arXiv:2402.09353 |

The skill's 16 quality gates (G-M1..G-M5, G-Q1..G-Q6, G-D1..G-D3, G-F1..G-F2) are enforced **upstream** by the `audit-config` phase before the pod is created — the entrypoint does not re-derive them. See `docs/reference/lora-training-catalog.md` for the full catalog.

## Build & push

```bash
cd docker/axolotl-lora-trainer
podman build -t docker.io/mdzaxo/axolotl-lora-trainer:latest .
podman push docker.io/mdzaxo/axolotl-lora-trainer:latest
```

The build completes in seconds and the push is small enough to finish in seconds (not hours). Verify size:

```bash
podman images docker.io/mdzaxo/axolotl-lora-trainer:latest --format '{{.Size}}'
# ~129MB uncompressed
podman save docker.io/mdzaxo/axolotl-lora-trainer:latest -o /tmp/img.tar
gzip -c /tmp/img.tar | wc -c
# ~44MB compressed (what actually gets pushed)
```

## Usage from hKask

`RunpodHost::submit()` will create a pod with this image when `RUNPOD_DOCKER_IMAGE=docker.io/mdzaxo/axolotl-lora-trainer:latest` is set. The pod's `docker_args` invoke the entrypoint directly. GPU selection defaults to `NVIDIA H100 80GB HBM3` for 70B+ models; override with `RUNPOD_GPU_TYPE_ID`. The lora-training skill's G2 gate (memory budget vs model size) informs this heuristic.

```bash
export RUNPOD_DOCKER_IMAGE=docker.io/mdzaxo/axolotl-lora-trainer:latest
export RUNPOD_GPU_TYPE_ID="NVIDIA H100 80GB HBM3"
export HF_TOKEN=hf_xxx
# then submit a training job via the hkask-mcp-training server
```

## What is NOT in this image

- **No CUDA devel base image** — RunPod pods provide the host CUDA driver; axolotl pulls GPU wheels at pip install time.
- **No PyTorch base image** — torch is pip-installed as an axolotl dependency.
- **No Unsloth** — deprecated per project policy.
- **No baked-in model weights or datasets** — pulled from HuggingFace at runtime.
- **No Python config generator** — the config is rendered in Rust and passed as `HKASK_AXOLOTL_CONFIG`.
- **No bloated 5GB+ image** — target is <500MB; this image is ~129MB uncompressed (~44MB compressed push).
