# trl-lora-trainer
# Minimal Docker image for LoRA fine-tuning with [TRL](https://github.com/huggingface/trl) on [RunPod](https://runpod.io) H100 pods.
#
# Parallel to `docker/axolotl-lora-trainer/` — same design ethic, different harness.
# Pushed to `docker.io/mdzaxo/trl-lora-trainer:latest`.
#
# **Image size: ~130MB uncompressed.** All heavy packages (PyTorch, TRL, PEFT,
# transformers) are pip-installed at pod startup — **not** baked into the image.
# This mirrors the axolotl-lora-trainer design (Lesson 11 of the RunPod guide).

# ── Stage 1: Minimal Python base ───────────────────────────────────────────
FROM python:3.11-slim

# ── System dependencies ────────────────────────────────────────────────────
# Same minimal set as axolotl-lora-trainer: bash, curl, git, build-essential.
# No CUDA toolkit (RunPod pods provide the host CUDA driver).
# No PyTorch base image (torch is pip-installed as a TRL dependency).
RUN apt-get update && apt-get install -y --no-install-recommends \
        bash \
        curl \
        git \
        build-essential \
        ca-certificates \
    && rm -rf /var/lib/apt/lists/*

# ── Entrypoint ─────────────────────────────────────────────────────────────
# The entrypoint is a single bash script — no Python config generator.
# It handles the full training lifecycle:
# 1. pip install trl peft bitsandbytes accelerate huggingface_hub (pinned versions)
# 2. Write $HKASK_TRL_SCRIPT env var to /workspace/train.py
# 3. huggingface-cli login if HF_TOKEN is set
# 4. python /workspace/train.py (TRL pulls base model + dataset from HuggingFace)
# 5. huggingface-cli upload of the adapter to $HKASK_HF_MODEL_REPOSITORY
# 6. Write a completion manifest (pure bash, no Python)
# 7. exec sleep infinity — keeps the pod alive for SSH debugging
COPY entrypoint.sh /usr/local/bin/entrypoint.sh
RUN chmod +x /usr/local/bin/entrypoint.sh

ENTRYPOINT ["/usr/local/bin/entrypoint.sh"]
