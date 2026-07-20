# Minimal llama.cpp LoRA Trainer Image
#
# Builds llama.cpp's finetune binary + a Rust orchestrator.
# No Python, no axolotl, no pip install at runtime.
# The only runtime dependency is libcuda.so (provided by RunPod host).
#
# Image size target: <500MB (llama.cpp binary ~15MB + Rust binary ~10MB + debian-slim ~80MB)

# ── Build stage 1: llama.cpp ─────────────────────────────────────────────────
FROM debian:bookworm-slim AS llama-builder

RUN apt-get update && apt-get install -y --no-install-recommends \
        build-essential \
        cmake \
        git \
        ca-certificates \
    && rm -rf /var/lib/apt/lists/*

# Clone and build llama.cpp with CUDA support
# We build with GGML_CUDA=ON so the finetune binary can use GPU
RUN git clone --depth 1 https://github.com/ggml-org/llama.cpp.git /llama.cpp \
    && cd /llama.cpp \
    && cmake -B build \
        -DCMAKE_BUILD_TYPE=Release \
        -DGGML_CUDA=ON \
        -DLLAMA_CURL=OFF \
    && cmake --build build --target llama-finetune -j$(nproc) \
    && cp build/bin/llama-finetune /usr/local/bin/llama-finetune

# ── Build stage 2: Rust orchestrator ────────────────────────────────────────
FROM rust:1.85-bookworm AS rust-builder

WORKDIR /build
COPY Cargo.toml .
COPY src/ src/
RUN cargo build --release && cp target/release/hkask-llama-finetune /usr/local/bin/

# ── Final image ──────────────────────────────────────────────────────────────
FROM debian:bookworm-slim

# Runtime deps: libcuda1 (NVIDIA driver userspace), ca-certificates, curl (for HF)
# No build-essential, no git, no cmake — those were build-stage only.
RUN apt-get update && apt-get install -y --no-install-recommends \
        ca-certificates \
        curl \
    && rm -rf /var/lib/apt/lists/*

# Copy the built binaries from the build stages
COPY --from=llama-builder /usr/local/bin/llama-finetune /usr/local/bin/llama-finetune
COPY --from=rust-builder /usr/local/bin/hkask-llama-finetune /usr/local/bin/hkask-llama-finetune

# Create workspace
RUN mkdir -p /workspace/outputs /workspace/.cache/huggingface

# Entryppoint: the Rust orchestrator
# It reads env vars (HKASK_*) and invokes llama-finetune
ENTRYPOINT ["/usr/local/bin/hkask-llama-finetune"]
