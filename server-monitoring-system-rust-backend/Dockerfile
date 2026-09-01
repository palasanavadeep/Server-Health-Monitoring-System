# syntax=docker/dockerfile:1.7

# ── Stage 1: Planner ────────────────────────────────────────────────────────────
# Computes the dependency recipe from Cargo manifests so the next stage
# can cache the heavy compilation step separately from application code.
FROM lukemathwalker/cargo-chef:latest-rust-1 AS planner

WORKDIR /app

COPY Cargo.toml Cargo.lock ./
COPY src ./src

RUN cargo chef prepare --recipe-path recipe.json

# ── Stage 2: Builder ────────────────────────────────────────────────────────────
# Cook (pre-compile) all dependencies first (cached layer), then compile
# the application binaries on top.
FROM lukemathwalker/cargo-chef:latest-rust-1 AS builder

WORKDIR /app

RUN apt-get update && \
    apt-get install -y --no-install-recommends \
        pkg-config \
        libssl-dev \
        ca-certificates && \
    rm -rf /var/lib/apt/lists/*

# Restore the dependency recipe and pre-compile deps (cached unless Cargo.lock changes)
COPY --from=planner /app/recipe.json recipe.json

RUN --mount=type=cache,target=/usr/local/cargo/registry \
    --mount=type=cache,target=/usr/local/cargo/git \
    cargo chef cook --release --recipe-path recipe.json

# Copy full source after deps are compiled (only app code triggers rebuild)
COPY . .

# Build all three application binaries
RUN --mount=type=cache,target=/usr/local/cargo/registry \
    --mount=type=cache,target=/usr/local/cargo/git \
    cargo build --release --bin api-server --bin consumer --bin metrics-worker

RUN ls -lah /app/target/release

# ── Stage 3: Runtime ────────────────────────────────────────────────────────────
# Minimal image — only the compiled binaries and system runtime deps.
FROM debian:bookworm-slim AS runtime

WORKDIR /app

RUN apt-get update && \
    apt-get install -y --no-install-recommends \
        ca-certificates \
        libssl3 \
        curl && \
    rm -rf /var/lib/apt/lists/*

# Copy all release binaries from the builder
COPY --from=builder /app/target/release/api-server     /usr/local/bin/api-server
COPY --from=builder /app/target/release/consumer       /usr/local/bin/consumer
COPY --from=builder /app/target/release/metrics-worker /usr/local/bin/metrics-worker

# Copy optional init/utility scripts
COPY scripts/ /app/scripts/

# Non-root user for security
RUN useradd -m -u 1001 appuser && chown -R appuser:appuser /app
USER appuser

EXPOSE 5000

# Default command — overridden in docker-compose for the consumer service
CMD ["api-server"]
