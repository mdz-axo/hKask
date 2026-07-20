# hKask Axolotl LoRA Trainer

General-purpose Docker image for LoRA fine-tuning with Axolotl on RunPod.

## Overview

This image extends `winglian/axolotl-cloud:main-latest` with a configurable
entrypoint that reads all training parameters from environment variables.
The same image can be used for any base model, dataset, and LoRA configuration.

## Build & Push

```bash
cd docker/axolotl-lora-trainer
docker build -t mdzaxolotl/axolotl-lora-trainer:latest .
docker push mdzaxolotl/axolotl-lora-trainer:latest
```

## Usage on RunPod

Create a pod with this image and set the environment variables:

```
imageName: mdzaxolotl/axolotl-lora-trainer:latest
gpuTypeId: NVIDIA H100 80GB HBM3
containerDiskInGb: 100
env:
  - HKASK_BASE_MODEL=unsloth/Qwen3.6-27B
  - HKASK_HF_DATASET_REPO=mdz-axo/capabilities-researcher-qa
  - HKASK_HF_DATASET_FILE=train_chat_full.jsonl
  - HKASK_HF_MODEL_REPO=mdz-axo/capabilities-researcher-v3-eva
  - HKASK_HF_TOKEN=hf_xxx
  - HKASK_LORA_R=32
  - HKASK_LORA_ALPHA=64
  - HKASK_PEFT_INIT=eva
  - HKASK_EVA_DATALOADER=mdz-axo/capabilities-researcher-qa
  - HKASK_NUM_EPOCHS=3
  - HKASK_LEARNING_RATE=0.0001
```

## Environment Variables

### Required

| Variable | Description | Example |
|---|---|---|
| `HKASK_BASE_MODEL` | HuggingFace base model ID | `unsloth/Qwen3.6-27B` |
| `HKASK_HF_DATASET_REPO` | HuggingFace dataset repository | `mdz-axo/capabilities-researcher-qa` |
| `HKASK_HF_DATASET_FILE` | Dataset file within the repo | `train_chat_full.jsonl` |
| `HKASK_HF_MODEL_REPO` | HuggingFace model repo for adapter upload | `mdz-axo/capabilities-researcher-v3-eva` |

### Authentication

| Variable | Description | Default |
|---|---|---|
| `HKASK_HF_TOKEN` | HuggingFace API token (for private datasets/repos) | (empty) |
| `HKASK_HF_DATASET_REVISION` | Dataset revision/branch | `main` |

### LoRA Configuration

| Variable | Description | Default |
|---|---|---|
| `HKASK_LORA_R` | LoRA rank | `16` |
| `HKASK_LORA_ALPHA` | LoRA alpha | `32` |
| `HKASK_LORA_DROPOUT` | LoRA dropout | `0` |
| `HKASK_LORA_TARGET_MODULES` | Comma-separated target modules | `q_proj,k_proj,v_proj,o_proj,gate_proj,up_proj,down_proj` |
| `HKASK_PEFT_INIT` | Init method: `true`, `eva`, or `pissa` | `true` |
| `HKASK_EVA_DATALOADER` | EVA dataloader dataset (required if PEFT_INIT=eva) | (empty) |
| `HKASK_EVA_RHO` | EVA redistribution uniformity | `2.5` |

### Training Hyperparameters

| Variable | Description | Default |
|---|---|---|
| `HKASK_NUM_EPOCHS` | Number of training epochs | `3` |
| `HKASK_LEARNING_RATE` | Learning rate | `0.0001` |
| `HKASK_WARMUP_STEPS` | Warmup steps | `100` |
| `HKASK_MICRO_BATCH_SIZE` | Micro batch size | `1` |
| `HKASK_EVAL_BATCH_SIZE` | Eval batch size | `1` |
| `HKASK_GRAD_ACCUM` | Gradient accumulation steps | `16` |
| `HKASK_GRADIENT_CHECKPOINTING` | Enable gradient checkpointing | `true` |
| `HKASK_LR_SCHEDULER` | LR scheduler | `cosine` |
| `HKASK_WEIGHT_DECAY` | Weight decay | `0.01` |
| `HKASK_MAX_GRAD_NORM` | Max gradient norm | `0.3` |
| `HKASK_OPTIM` | Optimizer | `adamw_8bit` |

### Evaluation

| Variable | Description | Default |
|---|---|---|
| `HKASK_VAL_SET_SIZE` | Validation set fraction | `0.05` |
| `HKASK_EVAL_STEPS` | Eval steps | `200` |
| `HKASK_SAVE_STEPS` | Save steps | `200` |
| `HKASK_SAVE_TOTAL_LIMIT` | Max checkpoints to keep | `5` |
| `HKASK_EARLY_STOPPING_PATIENCE` | Early stopping patience | `25` |

### Model Loading

| Variable | Description | Default |
|---|---|---|
| `HKASK_SEQUENCE_LEN` | Max sequence length | `4096` |
| `HKASK_BF16` | Use bfloat16 | `true` |
| `HKASK_FLASH_ATTENTION` | Use flash attention | `false` |
| `HKASK_LIGER_KERNEL` | Use Liger kernel | `true` |
| `HKASK_CUT_CROSS_ENTROPY` | Use cut cross entropy | `true` |
| `HKASK_LOAD_IN_4BIT` | Load in 4-bit (QLoRA) | `false` |
| `HKASK_LOAD_IN_8BIT` | Load in 8-bit | `false` |
| `HKASK_TRUST_REMOTE_CODE` | Trust remote code | `true` |

### Dataset

| Variable | Description | Default |
|---|---|---|
| `HKASK_DATASET_TYPE` | Dataset format type | `chat_template` |

### Output

| Variable | Description | Default |
|---|---|---|
| `HKASK_OUTPUT_DIR` | Output directory | `/workspace/outputs` |
| `HKASK_WORKSPACE` | Workspace directory | `/workspace` |

## How It Works

1. **Container starts** — runs `/start.sh` in background (Jupyter, SSH)
2. **Dataset download** — pulls the dataset file from HuggingFace
3. **Config generation** — `generate_config.py` reads env vars and writes YAML
4. **Training** — runs `axolotl train config.yaml`
5. **Upload** — uploads the adapter to the HuggingFace model repo
6. **Manifest** — writes `/workspace/completion.json` with job status
7. **Keep alive** — `sleep infinity` for SSH debugging

## Completion Manifest

After training, the container writes `/workspace/completion.json`:

```json
{
  "job_id": "abc123",
  "status": "completed",
  "base_model": "unsloth/Qwen3.6-27B",
  "dataset_repo": "mdz-axo/capabilities-researcher-qa",
  "dataset_file": "train_chat_full.jsonl",
  "model_repo": "mdz-axo/capabilities-researcher-v3-eva",
  "lora_r": 32,
  "lora_alpha": 64,
  "num_epochs": 3,
  "finished_at": "2026-07-20T00:00:00Z"
}
```

## SSH Access

The container stays alive after training for SSH debugging. Connect via:

```bash
ssh -p <port> root@<ip>
```

Check the training log:

```bash
cat /workspace/training.log
```

Check the generated config:

```bash
cat /workspace/config.yaml
```
