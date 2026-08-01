# syntax=docker/dockerfile:1

# ============================ Build stage ============================
FROM rust:1.97.1-alpine3.24 AS builder

RUN apk add --no-cache \
        build-base \
        pkgconfig \
        openssl-dev \
        perl \
        curl \
        nodejs \
        npm

RUN rustup target add wasm32-unknown-unknown

WORKDIR /app

COPY Cargo.toml ./
RUN mkdir -p src \
    && printf '' > src/lib.rs \
    && printf 'fn main() {}\n' > src/main.rs \
    && cargo generate-lockfile \
    && cargo fetch

RUN cargo install cargo-leptos --locked --version 0.3.7 \
    && cargo install wasm-bindgen-cli --locked --version 0.2.126

RUN npm install -g sass esbuild \
    && apk add --no-cache binaryen

COPY . .

RUN cargo leptos build --release

# ============================ Runtime stage ============================
FROM alpine:3.24 AS runtime

RUN apk add --no-cache \
        ca-certificates \
        openssl \
        tzdata

WORKDIR /app

COPY --from=builder /app/Cargo.toml /app/Cargo.toml
COPY --from=builder /app/target/release/leptos_btc /app/leptos_btc
COPY --from=builder /app/target/site /app/site

ENV LEPTOS_SITE_ROOT=/app/site
ENV LEPTOS_SITE_ADDR=0.0.0.0:3000
ENV LEPTOS_ENV=PROD
ENV LEPTOS_TAILWIND_VERSION=v4.3.3

EXPOSE 3000

CMD ["/app/leptos_btc"]