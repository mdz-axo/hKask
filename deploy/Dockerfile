# Stage 1: Build kask (Rust)
FROM rust:1.91-slim-bookworm AS kask-builder
RUN apt-get update && apt-get install -y pkg-config libssl-dev libsqlite3-dev libdbus-1-dev && rm -rf /var/lib/apt/lists/*
WORKDIR /app
COPY Cargo.toml Cargo.lock ./
COPY crates/ crates/
COPY mcp-servers/ mcp-servers/
RUN cargo build --release --bin kask

# Stage 2: Runtime
FROM debian:bookworm-slim

RUN apt-get update && apt-get install -y \
    ca-certificates \
    libsqlite3-0 \
    && rm -rf /var/lib/apt/lists/*

# Install Litestream (pre-built binary)
ADD https://github.com/benbjohnson/litestream/releases/download/v0.5.0/litestream-v0.5.0-linux-amd64.tar.gz /tmp/litestream.tar.gz
RUN tar -xzf /tmp/litestream.tar.gz -C /usr/local/bin litestream && \
    chmod +x /usr/local/bin/litestream && \
    rm /tmp/litestream.tar.gz

COPY --from=kask-builder /app/target/release/kask /usr/local/bin/kask

COPY deploy/k8s/entrypoint.sh /usr/local/bin/entrypoint.sh
RUN chmod +x /usr/local/bin/entrypoint.sh

VOLUME /data
EXPOSE 3000

ENV HKASK_DATA_DIR=/data

ENTRYPOINT ["/usr/local/bin/entrypoint.sh"]
