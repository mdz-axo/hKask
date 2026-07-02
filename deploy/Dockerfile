# Stage 1: Build kask + MCP servers (Rust)
# kask serve spawns MCP servers as child processes — they must be in the image.
FROM rust:1.91-slim-bookworm AS kask-builder
RUN apt-get update && apt-get install -y pkg-config libssl-dev libsqlite3-dev libdbus-1-dev && rm -rf /var/lib/apt/lists/*
WORKDIR /app
COPY Cargo.toml Cargo.lock ./
COPY crates/ crates/
COPY mcp-servers/ mcp-servers/

# Single cargo invocation builds CLI + all MCP servers, sharing compilation cache.
RUN cargo build --release \
    --bin kask \
    --package hkask-mcp-memory \
    --package hkask-mcp-condenser \
    --package hkask-mcp-research \
    --package hkask-mcp-companies \
    --package hkask-mcp-communication \
    --package hkask-mcp-fal \
    --package hkask-mcp-media \
    --package hkask-mcp-docproc \
    --package hkask-mcp-training \
    --package hkask-mcp-replica \
    --package hkask-mcp-kata-kanban \
    --package hkask-mcp-skill \
    --package hkask-mcp-curator \
    --package hkask-mcp-filesystem

# Stage 2: Runtime
FROM debian:bookworm-slim

RUN apt-get update && apt-get install -y \
    ca-certificates \
    libsqlite3-0 \
    libdbus-1-3 \
    && rm -rf /var/lib/apt/lists/*

# CLI binary + all MCP server binaries
COPY --from=kask-builder /app/target/release/kask /usr/local/bin/kask
COPY --from=kask-builder /app/target/release/hkask-mcp-* /usr/local/bin/

VOLUME /data
EXPOSE 3000

ENV HKASK_DATA_DIR=/data

# Create data dir (mounted PVC may not exist on first deploy),
# then start kask serve. Litestream sidecar handles S3 restore.
CMD ["sh", "-c", "mkdir -p ${HKASK_DATA_DIR} && exec kask serve"]
