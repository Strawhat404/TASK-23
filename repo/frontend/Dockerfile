FROM rust:1.87-bookworm AS builder

WORKDIR /app

RUN apt-get update && apt-get install -y --no-install-recommends \
        pkg-config \
        libssl-dev \
        curl \
    && rm -rf /var/lib/apt/lists/*

RUN rustup target add wasm32-unknown-unknown

# Download pre-built dx CLI binary (avoids OOM from compiling dioxus-cli from source)
RUN curl -L https://github.com/DioxusLabs/dioxus/releases/download/v0.6.3/dx-x86_64-unknown-linux-gnu-v0.6.3.tar.gz \
    | tar -xz -C /usr/local/bin

COPY Cargo.toml Cargo.lock ./
COPY Dioxus.toml           ./
COPY shared/               shared/
COPY frontend/             frontend/
COPY backend/Cargo.toml    backend/Cargo.toml

RUN mkdir -p backend/src && echo "fn main() {}" > backend/src/main.rs

RUN dx build --release

FROM nginx:alpine

COPY --from=builder /app/dist              /usr/share/nginx/html
COPY frontend/nginx.conf /etc/nginx/conf.d/default.conf

EXPOSE 80

CMD ["nginx", "-g", "daemon off;"]
