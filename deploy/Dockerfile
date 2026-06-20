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

# Stage 3: Build Conduit (Rust)
FROM rust:1.85-slim-bookworm AS conduit-builder
RUN apt-get update && apt-get install -y pkg-config libssl-dev libsqlite3-dev && rm -rf /var/lib/apt/lists/*
RUN git clone --depth 1 --branch v0.9.0 https://gitlab.com/famedly/conduit.git /conduit
WORKDIR /conduit
RUN cargo build --release --bin conduit

# Stage 4: Runtime
FROM debian:bookworm-slim

RUN apt-get update && apt-get install -y \
    ca-certificates \
    libsqlite3-0 \
    supervisor \
    gettext-base \
    && rm -rf /var/lib/apt/lists/*

COPY --from=kask-builder /app/target/release/kask /usr/local/bin/kask
COPY --from=litestream-builder /root/go/bin/litestream /usr/local/bin/litestream
COPY --from=conduit-builder /conduit/target/release/conduit /usr/local/bin/conduit

# Configuration templates rendered at container start via envsubst
COPY deploy/fly/litestream.yml.template /etc/litestream.yml.template
COPY deploy/fly/conduit.toml.template /etc/conduit/conduit.toml.template
COPY deploy/fly/supervisord.conf /etc/supervisor/conf.d/hkask.conf

COPY deploy/fly/entrypoint.sh /usr/local/bin/entrypoint.sh
RUN chmod +x /usr/local/bin/entrypoint.sh

VOLUME /data
EXPOSE 3000 8008 8448

ENV HKASK_DATA_DIR=/data

ENTRYPOINT ["/usr/local/bin/entrypoint.sh"]
