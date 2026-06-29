# Stage 1: Build kask + MCP servers (Rust)
# kask serve spawns MCP servers as child processes — they must be in the image.
FROM rust:1.91-slim-bookworm AS kask-builder
RUN apt-get update && apt-get install -y pkg-config libssl-dev libsqlite3-dev libdbus-1-dev && rm -rf /var/lib/apt/lists/*
WORKDIR /app
COPY Cargo.toml Cargo.lock ./
COPY crates/ crates/
COPY mcp-servers/ mcp-servers/

# Build CLI binary (default features: tui + communication + api)
RUN cargo build --release --bin kask

# Build MCP server binaries (kask serve spawns these via McpRuntime)
RUN cargo build --release \
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
    --package hkask-mcp-kanban \
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

# CLI binary
COPY --from=kask-builder /app/target/release/kask /usr/local/bin/kask
# MCP server binaries (wildcard copy: all hkask-mcp-* files)
COPY --from=kask-builder /app/target/release/hkask-mcp-* /usr/local/bin/

COPY deploy/k8s/entrypoint.sh /usr/local/bin/entrypoint.sh
RUN chmod +x /usr/local/bin/entrypoint.sh

VOLUME /data
EXPOSE 3000

ENV HKASK_DATA_DIR=/data

ENTRYPOINT ["/usr/local/bin/entrypoint.sh"]
