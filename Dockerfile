# syntax=docker/dockerfile:1

# ============================ Build stage ============================
FROM rust:1-slim AS builder

# Build essentials + SSL (axum/tokio need it) + node (for dart-sass & esbuild)
RUN apt-get update && apt-get install -y --no-install-recommends \
        pkg-config \
        libssl-dev \
        curl \
        nodejs \
        npm \
    && rm -rf /var/lib/apt/lists/*

# wasm target required by cargo-leptos
RUN rustup target add wasm32-unknown-unknown

WORKDIR /app

# --- dependency cache layer: copy manifest first, then cargo fetch ---
# Empty placeholder sources keep the package valid without compiling our code.
# Cargo.lock is gitignored in this project, so generate it if absent.
COPY Cargo.toml ./
RUN mkdir -p src \
    && printf '' > src/lib.rs \
    && printf 'fn main() {}\n' > src/main.rs \
    && cargo generate-lockfile \
    && cargo fetch

# Build tooling. Installing into PATH means cargo-leptos reuses these
# binaries instead of trying to download them from GitHub releases
# (the download step was the source of the earlier timeout error).
RUN cargo install cargo-leptos --locked --version 0.3.7 \
    && cargo install wasm-bindgen-cli --locked --version 0.2.126

# dart-sass (project uses style/main.scss) and esbuild (JS minification).
# binaryen provides wasm-opt, which cargo-leptos uses in release builds.
RUN npm install -g sass esbuild \
    && apt-get update && apt-get install -y --no-install-recommends binaryen \
    && rm -rf /var/lib/apt/lists/*

# --- now the real source and the build ---
COPY . .

# Build server binary + WASM + site assets in release mode.
RUN cargo leptos build --release

# ============================ Runtime stage ============================
FROM debian:bookworm-slim AS runtime

RUN apt-get update && apt-get install -y --no-install-recommends \
        ca-certificates \
        libssl3 \
    && rm -rf /var/lib/apt/lists/*

WORKDIR /app

# The server binary and the compiled site (HTML, JS, WASM, CSS).
COPY --from=builder /app/target/release/leptos_btc /app/leptos_btc
COPY --from=builder /app/target/site /app/site

# Tell the binary where to find assets and what address to bind.
# site-addr in Cargo.toml is 127.0.0.1:3000; override to 0.0.0.0 so the
# container is reachable from outside.
ENV LEPTOS_SITE_ROOT=/app/site
ENV LEPTOS_SITE_ADDR=0.0.0.0:3000
ENV LEPTOS_ENV=PROD

EXPOSE 3000

CMD ["/app/leptos_btc"]
