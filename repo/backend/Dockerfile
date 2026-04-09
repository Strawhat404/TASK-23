FROM rust:1.88-bookworm AS builder

WORKDIR /app

RUN apt-get update && apt-get install -y --no-install-recommends \
        pkg-config libssl-dev \
    && rm -rf /var/lib/apt/lists/*

# Copy manifests + lock file first for dependency caching
COPY Cargo.toml Cargo.lock Rocket.toml ./
COPY shared/Cargo.toml   shared/Cargo.toml
COPY backend/Cargo.toml  backend/Cargo.toml
COPY frontend/Cargo.toml frontend/Cargo.toml

# Stub sources so cargo can resolve and cache all deps
RUN mkdir -p shared/src backend/src frontend/src \
 && echo "pub fn _stub() {}" > shared/src/lib.rs \
 && echo "fn main() {}"      > backend/src/main.rs \
 && echo "fn main() {}"      > frontend/src/main.rs

# Cache all dependencies (this layer is reused unless Cargo.toml/lock changes)
RUN cargo build --release --bin backend 2>&1 | tail -5 || true
RUN find target/release/deps \( -name "backend*" -o -name "shared*" \) -delete 2>/dev/null; true

# Now copy real source and do the final build
COPY shared/  shared/
COPY backend/ backend/
RUN touch shared/src/lib.rs && cargo build --release --bin backend

FROM debian:bookworm-slim

RUN apt-get update && apt-get install -y --no-install-recommends \
        ca-certificates libssl3 curl \
    && rm -rf /var/lib/apt/lists/*

WORKDIR /app
COPY --from=builder /app/target/release/backend /app/backend
COPY --from=builder /app/Rocket.toml            /app/Rocket.toml

ENV ROCKET_ADDRESS=0.0.0.0
EXPOSE 8000
CMD ["/app/backend"]
