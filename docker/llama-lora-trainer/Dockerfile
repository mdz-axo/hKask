# Minimal llama.cpp LoRA Trainer Image
#
# Uses the official llama.cpp CUDA image as base (has finetune binary + CUDA).
# Adds a Rust orchestrator binary on top.
# No Python in our code, no pip install at runtime.
#
# Image size: ~4.4GB (llama.cpp CUDA image) + ~10MB (Rust binary)
# This is larger than 500MB, but the CUDA runtime is unavoidable.
# The alternative (building llama.cpp CPU-only) would be ~200MB but no GPU.

# ── Build stage: Rust orchestrator ──────────────────────────────────────────
FROM rust:1.85-bookworm AS rust-builder

WORKDIR /build
COPY Cargo.toml .
COPY src/ src/
RUN cargo build --release && cp target/release/hkask-llama-finetune /usr/local/bin/

# ── Final image: official llama.cpp CUDA + our Rust binary ──────────────────
FROM ghcr.io/ggml-org/llama.cpp:server-cuda

# Copy the Rust orchestrator
COPY --from=rust-builder /usr/local/bin/hkask-llama-finetune /usr/local/bin/hkask-llama-finetune

# Create workspace
RUN mkdir -p /workspace/outputs /workspace/.cache/huggingface

# Entrypoint: the Rust orchestrator
ENTRYPOINT ["/usr/local/bin/hkask-llama-finetune"]
