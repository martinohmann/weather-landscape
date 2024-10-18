# This uses cargo-chef to cache dependencies in order to speed up docker builds.
#
# See: https://github.com/LukeMathWalker/cargo-chef
FROM lukemathwalker/cargo-chef:latest-rust-1.82.0 AS chef
WORKDIR /app

## Prepare
FROM chef AS planner
COPY . .
RUN cargo chef prepare --recipe-path recipe.json

## Build
FROM chef AS builder
COPY --from=planner /app/recipe.json recipe.json
# Build the dependencies, this is the caching Docker layer.
RUN cargo chef cook --release --recipe-path recipe.json
# Build the application.
COPY . .
RUN cargo build --release

## Package
FROM debian:bookworm-slim
RUN apt-get update && apt-get install --yes ca-certificates && rm -rf /var/lib/apt/lists/*
COPY --from=builder /app/target/release/landscape-weather-server /usr/local/bin/landscape-weather-server
USER nobody
EXPOSE 8080
ENTRYPOINT ["/usr/local/bin/landscape-weather-server"]
