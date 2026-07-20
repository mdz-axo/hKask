#!/usr/bin/env python3
"""
Generate axolotl YAML config from environment variables.

All configuration is read from HKASK_* environment variables set by the
Docker image or RunPod pod creation. This allows the same Docker image to
be used for any base model, dataset, and LoRA configuration.

Output: YAML config to stdout (redirect to config.yaml)
"""

import os
import sys
import yaml


def env(key: str, default: str = "") -> str:
    return os.environ.get(key, default)


def env_int(key: str, default: int = 0) -> int:
    try:
        return int(os.environ.get(key, str(default)))
    except (ValueError, TypeError):
        return default


def env_float(key: str, default: float = 0.0) -> float:
    try:
        return float(os.environ.get(key, str(default)))
    except (ValueError, TypeError):
        return default


def env_bool(key: str, default: bool = False) -> bool:
    val = os.environ.get(key, str(default)).lower()
    return val in ("true", "1", "yes")


def env_list(key: str, default: list = None) -> list:
    val = os.environ.get(key, "")
    if not val:
        return default or []
    return [item.strip() for item in val.split(",") if item.strip()]


def main():
    config = {}

    # ── Model configuration ───────────────────────────────────────────────
    config["base_model"] = env("HKASK_BASE_MODEL")
    config["adapter"] = "lora"
    config["load_in_4bit"] = env_bool("HKASK_LOAD_IN_4BIT")
    config["load_in_8bit"] = env_bool("HKASK_LOAD_IN_8BIT")
    config["trust_remote_code"] = env_bool("HKASK_TRUST_REMOTE_CODE", True)

    # ── Precision ─────────────────────────────────────────────────────────
    config["sequence_len"] = env_int("HKASK_SEQUENCE_LEN", 4096)
    config["bf16"] = env_bool("HKASK_BF16", True)

    # ── LoRA configuration ────────────────────────────────────────────────
    config["lora_r"] = env_int("HKASK_LORA_R", 16)
    config["lora_alpha"] = env_int("HKASK_LORA_ALPHA", 32)
    config["lora_dropout"] = env_float("HKASK_LORA_DROPOUT", 0.0)
    config["lora_target_modules"] = env_list(
        "HKASK_LORA_TARGET_MODULES",
        ["q_proj", "k_proj", "v_proj", "o_proj", "gate_proj", "up_proj", "down_proj"],
    )

    # ── PEFT initialization ──────────────────────────────────────────────
    peft_init = env("HKASK_PEFT_INIT", "true")
    if peft_init.lower() == "eva":
        config["peft_init_lora_weights"] = "eva"
        # EVA config — requires a dataloader for activation-SVD
        eva_dataloader = env("HKASK_EVA_DATALOADER")
        if eva_dataloader:
            config["eva_config"] = {
                "dataloader": eva_dataloader,
                "rho": env_float("HKASK_EVA_RHO", 2.5),
            }
    elif peft_init.lower() == "pissa":
        config["peft_init_lora_weights"] = "pissa_niter_4"
    elif peft_init.lower() in ("true", "default", "1"):
        # Standard LoRA init (random A, zero B)
        pass
    else:
        # Custom init string passed through
        config["peft_init_lora_weights"] = peft_init

    # ── Dataset configuration ─────────────────────────────────────────────
    config["datasets"] = [
        {
            "path": env("HKASK_HF_DATASET_REPO"),
            "data_files": env("HKASK_HF_DATASET_FILE"),
            "type": env("HKASK_DATASET_TYPE", "chat_template"),
        }
    ]

    # ── Training hyperparameters ──────────────────────────────────────────
    config["num_epochs"] = env_int("HKASK_NUM_EPOCHS", 3)
    config["learning_rate"] = env_float("HKASK_LEARNING_RATE", 0.0001)
    config["warmup_steps"] = env_int("HKASK_WARMUP_STEPS", 100)
    config["micro_batch_size"] = env_int("HKASK_MICRO_BATCH_SIZE", 1)
    config["eval_batch_size"] = env_int("HKASK_EVAL_BATCH_SIZE", 1)
    config["gradient_accumulation_steps"] = env_int("HKASK_GRAD_ACCUM", 16)
    config["gradient_checkpointing"] = env_bool("HKASK_GRADIENT_CHECKPOINTING", True)
    config["lr_scheduler"] = env("HKASK_LR_SCHEDULER", "cosine")
    config["weight_decay"] = env_float("HKASK_WEIGHT_DECAY", 0.01)
    config["max_grad_norm"] = env_float("HKASK_MAX_GRAD_NORM", 0.3)
    config["optim"] = env("HKASK_OPTIM", "adamw_8bit")

    # ── Evaluation ────────────────────────────────────────────────────────
    config["val_set_size"] = env_float("HKASK_VAL_SET_SIZE", 0.05)
    config["eval_steps"] = env_int("HKASK_EVAL_STEPS", 200)
    config["save_steps"] = env_int("HKASK_SAVE_STEPS", 200)
    config["save_total_limit"] = env_int("HKASK_SAVE_TOTAL_LIMIT", 5)
    config["early_stopping_patience"] = env_int("HKASK_EARLY_STOPPING_PATIENCE", 25)

    # ── Optimizations ─────────────────────────────────────────────────────
    config["liger_kernel"] = env_bool("HKASK_LIGER_KERNEL", True)
    config["flash_attention"] = env_bool("HKASK_FLASH_ATTENTION", False)
    config["cut_cross_entropy"] = env_bool("HKASK_CUT_CROSS_ENTROPY", True)

    # ── Output ────────────────────────────────────────────────────────────
    config["output_dir"] = env("HKASK_OUTPUT_DIR", "/workspace/outputs")
    config["strict"] = False

    # ── HuggingFace upload (optional) ─────────────────────────────────────
    model_repo = env("HKASK_HF_MODEL_REPO")
    if model_repo:
        config["hub_model_id"] = model_repo

    # ── Write YAML to stdout ──────────────────────────────────────────────
    yaml.dump(config, sys.stdout, default_flow_style=False, sort_keys=False)


if __name__ == "__main__":
    main()
