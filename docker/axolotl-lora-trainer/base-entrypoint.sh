#!/usr/bin/env bash
# ── Unsloth Training Entrypoint ────────────────────────────────────────────
# Handles volume mounting, model caching, and training orchestration for
# Unsloth on RunPod (Serverless or Pod).
#
# Modes (set via CMD or RUN_MODE env var):
#   jupyter      — Start JupyterLab (default, interactive use)
#   train        — Run /workspace/train.py with env-configured parameters
#   download-all — Populate network volume with models, then exit
#   shell        — Drop into bash (debugging)
#
# Environment variables:
#   RUN_MODE         — "jupyter" | "train" | "download-all" | "shell"
#   HF_TOKEN         — HuggingFace token (for gated models like Llama)
#   WANDB_API_KEY    — Weights & Biases API key (optional)
#   JUPYTER_PASSWORD — JupyterLab password (default: "unsloth")
#   MODEL_NAME       — HuggingFace model ID (e.g., "unsloth/Llama-3.2-3B")
#   DATASET_NAME     — HuggingFace dataset ID (e.g., "yahma/alpaca-cleaned")
#   OUTPUT_DIR       — Checkpoint output dir (default: /runpod-volume/outputs)
#   PROMPT_FORMAT    — Dataset format: auto | alpaca | sharegpt | raw

set -euo pipefail

MODE="${1:-${RUN_MODE:-jupyter}}"
JUPYTER_PASSWORD="${JUPYTER_PASSWORD:-unsloth}"
VOLUME="${RUNPOD_VOLUME_PATH:-/runpod-volume}"
LOCKFILE="/runpod-volume/.training.lock"

# ── Volume Setup ──────────────────────────────────────────────────────────
setup_volume() {
    echo "=== Volume Setup ==="

    if [ -d "$VOLUME" ]; then
        echo "  Network volume found at $VOLUME"

        # Prevent concurrent pods from corrupting shared volume state.
        # Only enforce for train mode (jupyter/shell are safe for concurrent use).
        if [ "$MODE" = "train" ]; then
            if [ -f "$LOCKFILE" ]; then
                echo "  ERROR: Training lock file exists: $LOCKFILE"
                echo "  Another pod may be training on this volume."
                echo "  Remove the lock file if you're sure no training is active:"
                echo "    rm $LOCKFILE"
                exit 1
            fi
            echo "$$" > "$LOCKFILE"
            trap 'rm -f "$LOCKFILE"' EXIT
            echo "  Training lock acquired (pid $$)"
        fi

        # Create standard directory structure
        mkdir -p "$VOLUME"/{models,datasets,outputs,hf-cache}

        # HF_HOME redirects HuggingFace cache to volume (set in Dockerfile).
        # Ensure the directory exists so HF doesn't fail on first download.
        mkdir -p "${HF_HOME:-/workspace/.hf-cache}"
        echo "  HF cache → ${HF_HOME:-/workspace/.hf-cache}"

        # Symlink workspace subdirs to volume
        ln -sf "$VOLUME/models" /workspace/models 2>/dev/null || true
        ln -sf "$VOLUME/datasets" /workspace/datasets 2>/dev/null || true
        ln -sf "$VOLUME/outputs" /workspace/outputs 2>/dev/null || true

        echo "  Volume size: $(df -h "$VOLUME" | tail -1 | awk '{print $2}')"
        echo "  Volume used: $(df -h "$VOLUME" | tail -1 | awk '{print $3}')"
    else
        echo "  No network volume found (path: $VOLUME)"
        echo "  Models and checkpoints will NOT persist across restarts!"
        mkdir -p /workspace/{models,datasets,outputs,hf-cache}
    fi
}

# ── GPU Info ──────────────────────────────────────────────────────────────
show_gpu_info() {
    echo ""
    echo "=== GPU Info ==="
    if command -v nvidia-smi &>/dev/null; then
        nvidia-smi --query-gpu=name,memory.total,memory.free --format=csv,noheader 2>/dev/null || echo "  nvidia-smi failed"
    else
        echo "  No NVIDIA GPU detected"
    fi
    echo "  PyTorch CUDA available: $(python3 -c 'import torch; print(torch.cuda.is_available())' 2>/dev/null || echo 'unknown')"
    echo "  Unsloth version: $(pip show unsloth 2>/dev/null | grep Version | awk '{print $2}' || echo 'unknown')"
}

# ── HuggingFace Login ─────────────────────────────────────────────────────
hf_login() {
    if [ -n "${HF_TOKEN:-}" ]; then
        echo "  Logging into HuggingFace..."
        python3 -c "from huggingface_hub import login; login(token='$HF_TOKEN')" || true
        echo "  HF login complete"
    else
        echo "  No HF_TOKEN set — gated models (Llama, Gemma) will fail to download"
    fi
}

# ── W&B Login ─────────────────────────────────────────────────────────────
wandb_login() {
    if [ -n "${WANDB_API_KEY:-}" ]; then
        echo "  Logging into Weights & Biases..."
        python3 -c "import wandb; wandb.login(key='$WANDB_API_KEY')" || true
    fi
}

# ── Banner ────────────────────────────────────────────────────────────────
echo "╔══════════════════════════════════════════════════════════════╗"
echo "║           Unsloth Training — RunPod Endpoint                ║"
echo "║           Mode: $MODE                                ║"
echo "╚══════════════════════════════════════════════════════════════╝"

setup_volume
hf_login
wandb_login
show_gpu_info

# ── Mode Dispatch ─────────────────────────────────────────────────────────
case "$MODE" in
    jupyter)
        echo ""
        echo "=== Starting JupyterLab on port 8888 ==="
        echo "  Password: $JUPYTER_PASSWORD"
        echo "  Notebooks: /workspace/unsloth-notebooks/"
        echo ""

        # Hash Jupyter password. Try jupyter_server.auth first (newer),
        # fall back to notebook.auth (older images).
        JP_HASH=$(python3 -c "
try:
    from jupyter_server.auth import passwd
    print(passwd('$JUPYTER_PASSWORD'))
except ImportError:
    from notebook.auth import passwd
    print(passwd('$JUPYTER_PASSWORD'))
" 2>/dev/null || echo '')

        PASSWORD_ARG=""
        if [ -n "$JP_HASH" ]; then
            PASSWORD_ARG="--ServerApp.password=$JP_HASH"
        fi

        exec jupyter-lab \
            --ip=0.0.0.0 \
            --port=8888 \
            --no-browser \
            --allow-root \
            $PASSWORD_ARG \
            --ServerApp.token='' \
            --notebook-dir=/workspace
        ;;

    train)
        echo ""
        echo "=== Starting Training ==="
        echo "  Model:   ${MODEL_NAME:-unsloth/Llama-3.2-3B-Instruct}"
        echo "  Dataset: ${DATASET_NAME:-yahma/alpaca-cleaned}"
        echo "  Format:  ${PROMPT_FORMAT:-auto}"
        echo "  Output:  ${OUTPUT_DIR:-/runpod-volume/outputs}"
        echo ""

        TRAIN_CMD="python3 /workspace/train.py \
            --model_name ${MODEL_NAME:-unsloth/Llama-3.2-3B-Instruct} \
            --dataset_name ${DATASET_NAME:-yahma/alpaca-cleaned} \
            --prompt_format ${PROMPT_FORMAT:-auto} \
            --output_dir ${OUTPUT_DIR:-/runpod-volume/outputs} \
            ${TRAIN_ARGS:-}"

        # Run in tmux for session persistence
        if command -v tmux &>/dev/null; then
            echo "  Launching in tmux session 'training'"
            echo "  Attach: tmux attach -t training"
            echo "  Detach: Ctrl+B, D"
            echo ""
            tmux new-session -d -s training "$TRAIN_CMD 2>&1 | tee /workspace/training.log"
            echo "  Training started. Log: /workspace/training.log"
            echo "  Container will remain alive. Stop pod when training completes."
            sleep infinity
        else
            exec $TRAIN_CMD
        fi
        ;;

    download-all)
        exec /populate-volume.sh "$VOLUME"
        ;;

    shell)
        exec /bin/bash
        ;;

    axolotl-train)
        echo ""
        echo "=== Starting Axolotl LoRA Training ==="
        exec /workspace/axolotl-entrypoint.sh
        ;;

    *)
        echo "ERROR: Unknown mode '$MODE'"
        echo "Usage: entrypoint.sh {jupyter|train|download-all|shell}"
        exit 1
        ;;
esac
