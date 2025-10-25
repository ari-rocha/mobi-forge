FROM rust:latest AS builder

WORKDIR /app

COPY Cargo.toml Cargo.lock ./
RUN mkdir src && echo "fn main() {}" > src/main.rs

COPY config ./config
COPY templates ./templates
COPY mock-data ./mock-data
COPY static ./static
COPY commerce-data ./commerce-data

RUN cargo fetch

COPY . .

RUN cargo build --release

RUN cargo install wasm-pack --locked --version 0.13.1

RUN cd catalog-search && wasm-pack build --release --target web --out-dir ../static/pkg

# RUN cargo run --release --manifest-path catalog-tools/Cargo.toml -- from-json --furniture commerce-data/Furniture.json --variations commerce-data/Variation.json --catalog-out static/catalog.bin

RUN cargo run --manifest-path catalog-tools/Cargo.toml -- mock --count 3000 --variations-per-product 2 --catalog-out static/catalog.bin

FROM debian:bookworm-slim

RUN apt-get update && apt-get install -y --no-install-recommends ca-certificates tini curl && rm -rf /var/lib/apt/lists/*

WORKDIR /app

COPY --from=builder /app/target/release/mobi-forge /usr/local/bin/mobi-forge
COPY --from=builder /app/templates ./templates
COPY --from=builder /app/mock-data ./mock-data
COPY --from=builder /app/config ./config
COPY --from=builder /app/static ./static
COPY --from=builder /app/commerce-data ./commerce-data

ENV TEMPLATE_DIR=templates \
    MOCK_DATA_DIR=mock-data \
    ROUTES_FILE=config/routes.json \
    STATIC_DIR=static \
    MOBI_API_TOKEN="" \
    ROCKET_ADDRESS=0.0.0.0 \
    ROCKET_PORT=8080

EXPOSE 8080

HEALTHCHECK --interval=30s --timeout=5s --start-period=5s --retries=3 CMD curl -fS http://127.0.0.1:8080/health || exit 1

ENTRYPOINT ["/usr/bin/tini", "--"]

CMD ["mobi-forge"]
