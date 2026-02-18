# Containerfile for parking-operator-adaptor
#
# Multi-stage build: Rust builder -> minimal Debian runtime
# Build context: repository root

# -- Stage 1: Build ----------------------------------------------------------
FROM docker.io/library/rust:1.75-bookworm AS builder

RUN apt-get update && apt-get install -y protobuf-compiler && rm -rf /var/lib/apt/lists/*

WORKDIR /build

# Copy the full Rust workspace and proto sources needed for compilation
COPY rhivos/Cargo.toml rhivos/Cargo.toml
COPY proto/ proto/
COPY rhivos/parking-proto/ rhivos/parking-proto/
COPY rhivos/locking-service/ rhivos/locking-service/
COPY rhivos/cloud-gateway-client/ rhivos/cloud-gateway-client/
COPY rhivos/update-service/ rhivos/update-service/
COPY rhivos/parking-operator-adaptor/ rhivos/parking-operator-adaptor/
COPY mock/sensors/ mock/sensors/

RUN cd rhivos && cargo build --release --bin parking-operator-adaptor

# -- Stage 2: Runtime --------------------------------------------------------
FROM docker.io/library/debian:bookworm-slim

RUN apt-get update && \
    apt-get install -y --no-install-recommends ca-certificates && \
    rm -rf /var/lib/apt/lists/*

COPY --from=builder /build/rhivos/target/release/parking-operator-adaptor /usr/local/bin/parking-operator-adaptor

EXPOSE 50054

ENTRYPOINT ["parking-operator-adaptor"]
