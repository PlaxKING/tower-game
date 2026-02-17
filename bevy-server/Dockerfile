# Multi-stage Rust build for Tower Bevy Server
# Build context: project root (docker-compose context: .)
#
# Usage:
#   docker compose up -d bevy-server
#   docker build -f bevy-server/Dockerfile .

# Stage 1: Build
FROM rust:1.93-bookworm AS builder

WORKDIR /app

# Install build dependencies (LMDB, OpenSSL, protobuf compiler)
RUN apt-get update && apt-get install -y \
    liblmdb-dev \
    libssl-dev \
    pkg-config \
    protobuf-compiler \
    && rm -rf /var/lib/apt/lists/*

# Copy proto definitions first (needed by build.rs)
COPY shared/proto/ shared/proto/

# Copy Cargo manifests for dependency caching
COPY bevy-server/Cargo.toml bevy-server/Cargo.lock bevy-server/build.rs bevy-server/

# Create dummy source and benches to pre-build dependencies
RUN mkdir -p bevy-server/src bevy-server/benches \
    && echo "fn main() {}" > bevy-server/src/main.rs \
    && echo "" > bevy-server/src/lib.rs \
    && echo "fn main() {}" > bevy-server/benches/floor_generation.rs \
    && echo "fn main() {}" > bevy-server/benches/cache_comparison_benchmarks.rs
RUN cd bevy-server && cargo build --release 2>/dev/null || true
RUN rm -rf bevy-server/src bevy-server/benches

# Copy actual source and benches
COPY bevy-server/src/ bevy-server/src/
COPY bevy-server/benches/ bevy-server/benches/

# Force cargo to detect source changes (Docker COPY preserves timestamps from build context)
RUN find bevy-server/src -name '*.rs' -exec touch {} + \
    && find bevy-server/benches -name '*.rs' -exec touch {} +

# Build the real binary
RUN cd bevy-server && cargo build --release --bin tower-bevy-server

# Stage 2: Runtime (minimal image)
FROM debian:bookworm-slim

RUN apt-get update && apt-get install -y \
    liblmdb0 \
    libssl3 \
    ca-certificates \
    curl \
    && rm -rf /var/lib/apt/lists/*

WORKDIR /app

# Copy the built binary
COPY --from=builder /app/bevy-server/target/release/tower-bevy-server /app/tower-bevy-server

# Create data directories
RUN mkdir -p /app/data/templates /app/data/floor_cache

# Environment defaults
ENV DATABASE_URL=postgres://postgres:localdb@postgres:5432/tower_game
ENV LMDB_PATH=/app/data/templates
ENV LMDB_MAX_SIZE=500002816
ENV API_PORT=50051
ENV RUST_LOG=info

# Expose ports
EXPOSE 5000/udp
EXPOSE 50051/tcp

# Health check via HTTP API
HEALTHCHECK --interval=10s --timeout=3s --retries=5 \
    CMD curl -f http://localhost:50051/health || exit 1

CMD ["/app/tower-bevy-server"]
