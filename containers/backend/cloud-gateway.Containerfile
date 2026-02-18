# Containerfile for cloud-gateway
#
# Multi-stage build: Go builder -> distroless static runtime
# Build context: repository root

# -- Stage 1: Build ----------------------------------------------------------
FROM docker.io/library/golang:1.22-bookworm AS builder

WORKDIR /build

# Copy Go module files first for dependency caching
COPY backend/cloud-gateway/go.mod backend/cloud-gateway/go.sum* backend/cloud-gateway/

RUN cd backend/cloud-gateway && go mod download

# Copy source code
COPY backend/cloud-gateway/ backend/cloud-gateway/

RUN cd backend/cloud-gateway && \
    CGO_ENABLED=0 GOOS=linux go build -o /out/cloud-gateway .

# -- Stage 2: Runtime --------------------------------------------------------
FROM gcr.io/distroless/static-debian12:nonroot

COPY --from=builder /out/cloud-gateway /usr/local/bin/cloud-gateway

EXPOSE 8081

ENTRYPOINT ["cloud-gateway"]
