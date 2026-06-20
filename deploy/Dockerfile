# Stage 1: Build kask (Rust)
FROM rust:1.85-slim-bookworm AS kask-builder
RUN apt-get update && apt-get install -y pkg-config libssl-dev && rm -rf /var/lib/apt/lists/*
WORKDIR /app
COPY Cargo.toml Cargo.lock ./
COPY crates/ crates/
COPY mcp-servers/ mcp-servers/
RUN cargo build --release --bin kask

# Stage 2: Build Litestream (Go)
FROM golang:1.23-bookworm AS litestream-builder
RUN go install github.com/benbjohnson/litestream/cmd/litestream@v0.5.0

# Stage 3: Runtime
FROM debian:bookworm-slim

RUN apt-get update && apt-get install -y \
    ca-certificates \
    libsqlite3-0 \
    gettext-base \
    && rm -rf /var/lib/apt/lists/*

COPY --from=kask-builder /app/target/release/kask /usr/local/bin/kask
COPY --from=litestream-builder /root/go/bin/litestream /usr/local/bin/litestream

# Litestream configuration template — rendered at container start
COPY deploy/fly/litestream.yml.template /etc/litestream.yml.template

COPY deploy/fly/entrypoint.sh /usr/local/bin/entrypoint.sh
RUN chmod +x /usr/local/bin/entrypoint.sh

VOLUME /data
EXPOSE 3000

ENV HKASK_DATA_DIR=/data

ENTRYPOINT ["/usr/local/bin/entrypoint.sh"]
