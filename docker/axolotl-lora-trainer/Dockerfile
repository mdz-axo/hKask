# =============================================================================
# hKask Axolotl LoRA Trainer — General-purpose LoRA fine-tuning image
# =============================================================================
# Extends the existing hkask-unsloth-training image by adding axolotl and
# a configurable training entrypoint.
#
# Build:
#   podman build -t docker.io/mdzaxo/axolotl-lora-trainer:latest .
# Push:
#   podman push docker.io/mdzaxo/axolotl-lora-trainer:latest
# =============================================================================

FROM localhost/mdzaxo/hkask-unsloth-training:v1.0.0

# Install axolotl and dependencies
RUN pip install --no-cache-dir axolotl[transformers] pyyaml huggingface_hub

# Copy the training entrypoint and config generator
COPY entrypoint.sh /workspace/axolotl-entrypoint.sh
COPY generate_config.py /workspace/generate_config.py
RUN chmod +x /workspace/axolotl-entrypoint.sh

# Default environment variables (can be overridden at pod creation time)
ENV HKASK_BASE_MODEL=""
ENV HKASK_HF_DATASET_REPO=""
ENV HKASK_HF_DATASET_FILE=""
ENV HKASK_HF_DATASET_REVISION="main"
ENV HKASK_HF_MODEL_REPO=""
ENV HKASK_HF_TOKEN=""
ENV HKASK_LORA_R="16"
ENV HKASK_LORA_ALPHA="32"
ENV HKASK_LORA_DROPOUT="0"
ENV HKASK_LORA_TARGET_MODULES="q_proj,k_proj,v_proj,o_proj,gate_proj,up_proj,down_proj"
ENV HKASK_PEFT_INIT="true"
ENV HKASK_EVA_DATALOADER=""
ENV HKASK_EVA_RHO="2.5"
ENV HKASK_NUM_EPOCHS="3"
ENV HKASK_LEARNING_RATE="0.0001"
ENV HKASK_WARMUP_STEPS="100"
ENV HKASK_MICRO_BATCH_SIZE="1"
ENV HKASK_EVAL_BATCH_SIZE="1"
ENV HKASK_GRAD_ACCUM="16"
ENV HKASK_GRADIENT_CHECKPOINTING="true"
ENV HKASK_LR_SCHEDULER="cosine"
ENV HKASK_WEIGHT_DECAY="0.01"
ENV HKASK_MAX_GRAD_NORM="0.3"
ENV HKASK_OPTIM="adamw_8bit"
ENV HKASK_VAL_SET_SIZE="0.05"
ENV HKASK_EVAL_STEPS="200"
ENV HKASK_SAVE_STEPS="200"
ENV HKASK_SAVE_TOTAL_LIMIT="5"
ENV HKASK_EARLY_STOPPING_PATIENCE="25"
ENV HKASK_SEQUENCE_LEN="4096"
ENV HKASK_BF16="true"
ENV HKASK_FLASH_ATTENTION="false"
ENV HKASK_LIGER_KERNEL="true"
ENV HKASK_CUT_CROSS_ENTROPY="true"
ENV HKASK_LOAD_IN_4BIT="false"
ENV HKASK_LOAD_IN_8BIT="false"
ENV HKASK_DATASET_TYPE="chat_template"
ENV HKASK_TRUST_REMOTE_CODE="true"
ENV HKASK_OUTPUT_DIR="/workspace/outputs"
ENV HKASK_WORKSPACE="/workspace"
ENV HKASK_JOB_ID=""

# The entrypoint runs the container's normal startup, then starts training
ENTRYPOINT ["/bin/bash", "/workspace/axolotl-entrypoint.sh"]
CMD ["train"]
