# Containerfile for parking-fee-service
#
# Multi-stage build: Go builder -> distroless static runtime
# Build context: repository root

# -- Stage 1: Build ----------------------------------------------------------
FROM docker.io/library/golang:1.22-bookworm AS builder

WORKDIR /build

# Copy Go module files first for dependency caching
COPY backend/parking-fee-service/go.mod backend/parking-fee-service/go.sum* backend/parking-fee-service/

RUN cd backend/parking-fee-service && go mod download

# Copy source code
COPY backend/parking-fee-service/ backend/parking-fee-service/

RUN cd backend/parking-fee-service && \
    CGO_ENABLED=0 GOOS=linux go build -o /out/parking-fee-service .

# -- Stage 2: Runtime --------------------------------------------------------
FROM gcr.io/distroless/static-debian12:nonroot

COPY --from=builder /out/parking-fee-service /usr/local/bin/parking-fee-service

EXPOSE 8080

ENTRYPOINT ["parking-fee-service"]
