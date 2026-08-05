# Stage 1: Build
FROM rust:1.82-slim AS builder

RUN apt-get update && apt-get install -y pkg-config libssl-dev && rm -rf /var/lib/apt/lists/*

WORKDIR /app
COPY Cargo.toml Cargo.lock* ./
# Create a dummy src to cache dependencies
RUN mkdir src && echo "fn main() {}" > src/main.rs && echo "fn main() {}" > src/consumer.rs && echo "" > src/lib.rs
RUN cargo build --release --bin api-server 2>/dev/null || true
RUN rm -rf src

COPY src ./src
RUN cargo build --release --bin api-server

# Stage 2: Runtime
FROM debian:bookworm-slim

RUN apt-get update && apt-get install -y ca-certificates libssl3 && rm -rf /var/lib/apt/lists/*

WORKDIR /app
COPY --from=builder /app/target/release/api-server ./api-server

RUN mkdir -p logs

EXPOSE 5000

CMD ["./api-server"]
