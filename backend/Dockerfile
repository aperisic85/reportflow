FROM rust:1-bookworm AS builder
WORKDIR /build
COPY Cargo.toml ./
COPY src ./src
COPY migrations ./migrations
RUN cargo build --release
FROM debian:bookworm-slim
RUN apt-get update && apt-get install -y --no-install-recommends ca-certificates && rm -rf /var/lib/apt/lists/*
COPY --from=builder /build/target/release/reportflow-api /usr/local/bin/reportflow-api
ENV RUST_LOG=info
EXPOSE 8095
CMD ["reportflow-api"]
