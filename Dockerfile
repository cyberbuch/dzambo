# dzambo API сървър — multi-stage Rust build.
# Миграциите са embedded в бинаря (embed_migrations), не се копират отделно.

FROM rust:1-bookworm AS builder
RUN apt-get update && apt-get install -y --no-install-recommends \
    libpq-dev pkg-config build-essential cmake && rm -rf /var/lib/apt/lists/*
WORKDIR /app
# Кеширане на dependency-тата: първо само манифестите.
COPY Cargo.toml Cargo.lock ./
COPY crates/domain/Cargo.toml crates/domain/Cargo.toml
COPY crates/storage/Cargo.toml crates/storage/Cargo.toml
COPY crates/api/Cargo.toml crates/api/Cargo.toml
COPY crates/server/Cargo.toml crates/server/Cargo.toml
COPY src-tauri/Cargo.toml src-tauri/Cargo.toml
RUN mkdir -p crates/domain/src crates/storage/src crates/api/src crates/server/src src-tauri/src \
    && echo "" > crates/domain/src/lib.rs \
    && echo "" > crates/storage/src/lib.rs \
    && echo "" > crates/api/src/lib.rs \
    && echo "fn main(){}" > crates/server/src/main.rs \
    && echo "" > src-tauri/src/lib.rs \
    && echo "fn main(){}" > src-tauri/src/main.rs \
    && cargo build --release -p server 2>/dev/null || true
# Реалният код.
COPY crates crates
COPY .cargo .cargo
RUN touch crates/domain/src/lib.rs crates/storage/src/lib.rs crates/api/src/lib.rs crates/server/src/main.rs \
    && cargo build --release -p server

FROM debian:bookworm-slim AS runtime
RUN apt-get update && apt-get install -y --no-install-recommends \
    libpq5 ca-certificates && rm -rf /var/lib/apt/lists/*
WORKDIR /app
COPY --from=builder /app/target/release/dzambo-server /app/dzambo-server
ENV DZAMBO_BIND=0.0.0.0 DZAMBO_PORT=8095
EXPOSE 8095
CMD ["/app/dzambo-server"]