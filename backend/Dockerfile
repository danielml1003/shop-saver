# ── Stage 1: build ────────────────────────────────────────────────────────────
FROM rust:1.94-slim AS builder

RUN apt-get update && apt-get install -y pkg-config libssl-dev && rm -rf /var/lib/apt/lists/*

WORKDIR /app

# Cache dependencies: copy manifests first, build a dummy main, then replace with real source.
COPY Cargo.toml Cargo.lock ./
RUN mkdir src && echo "fn main() {}" > src/main.rs
RUN cargo build --release
RUN rm -rf src

COPY src ./src
# Touch main.rs so cargo knows source changed
RUN touch src/main.rs
RUN cargo build --release

# ── Stage 2: minimal runtime ───────────────────────────────────────────────────
FROM debian:bookworm-slim

RUN apt-get update \
    && apt-get install -y --no-install-recommends ca-certificates libssl3 \
    && rm -rf /var/lib/apt/lists/* \
    && useradd --system --no-create-home appuser

WORKDIR /app

COPY --from=builder /app/target/release/backend ./shop-saver

USER appuser

EXPOSE 3000

CMD ["./shop-saver"]
