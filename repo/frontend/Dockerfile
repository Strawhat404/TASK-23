FROM rust:1.87-bookworm AS builder

WORKDIR /app

RUN apt-get update && apt-get install -y --no-install-recommends \
        pkg-config libssl-dev curl \
    && rm -rf /var/lib/apt/lists/*

RUN rustup target add wasm32-unknown-unknown

# Pre-built dx CLI (retry up to 5 times on connection failure)
RUN curl -L --retry 5 --retry-delay 3 --retry-connrefused \
    https://github.com/DioxusLabs/dioxus/releases/download/v0.6.3/dx-x86_64-unknown-linux-gnu-v0.6.3.tar.gz \
    | tar -xz -C /usr/local/bin

# Cache dependencies first
COPY Cargo.toml Cargo.lock ./
COPY Dioxus.toml           ./
COPY shared/Cargo.toml     shared/Cargo.toml
COPY frontend/Cargo.toml   frontend/Cargo.toml
COPY backend/Cargo.toml    backend/Cargo.toml

RUN mkdir -p shared/src frontend/src backend/src \
 && echo "pub fn _stub() {}" > shared/src/lib.rs \
 && echo "fn main() {}"      > frontend/src/main.rs \
 && echo "fn main() {}"      > backend/src/main.rs

RUN cargo fetch

# Build with real source
COPY shared/   shared/
COPY frontend/ frontend/
# Place custom index.html at workspace root for dx to use as template
COPY frontend/index.html ./index.html
RUN dx build --package frontend --platform web \
 && cp frontend/assets/main.css target/dx/frontend/debug/web/public/main.css

FROM nginx:alpine
COPY --from=builder /app/target/dx/frontend/debug/web/public /usr/share/nginx/html
COPY frontend/nginx.conf /etc/nginx/conf.d/default.conf
EXPOSE 80
CMD ["nginx", "-g", "daemon off;"]
