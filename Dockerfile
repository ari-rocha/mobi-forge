FROM rust:latest AS builder

WORKDIR /app

COPY Cargo.toml Cargo.lock ./
RUN mkdir src && echo "fn main() {}" > src/main.rs

RUN cargo fetch

COPY . .

RUN cargo build --release

FROM debian:bookworm-slim

RUN apt-get update && apt-get install -y --no-install-recommends ca-certificates tini && rm -rf /var/lib/apt/lists/*

WORKDIR /app

COPY --from=builder /app/target/release/mobi-forge /usr/local/bin/mobi-forge

ENV ROCKET_ADDRESS=0.0.0.0 \
    ROCKET_PORT=8080

EXPOSE 8080

ENTRYPOINT ["/usr/bin/tini", "--"]
CMD ["mobi-forge"]
