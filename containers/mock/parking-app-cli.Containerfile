# Containerfile for parking-app-cli
#
# Multi-stage build: Go builder -> distroless static runtime
# Build context: repository root

# -- Stage 1: Build ----------------------------------------------------------
FROM docker.io/library/golang:1.22-bookworm AS builder

WORKDIR /build

# Copy generated proto Go packages (referenced via replace directive)
COPY proto/gen/go/ proto/gen/go/

# Copy Go module files first for dependency caching
COPY mock/parking-app-cli/go.mod mock/parking-app-cli/go.sum* mock/parking-app-cli/

RUN cd mock/parking-app-cli && go mod download

# Copy source code
COPY mock/parking-app-cli/ mock/parking-app-cli/

RUN cd mock/parking-app-cli && \
    CGO_ENABLED=0 GOOS=linux go build -o /out/parking-app-cli .

# -- Stage 2: Runtime --------------------------------------------------------
FROM gcr.io/distroless/static-debian12:nonroot

COPY --from=builder /out/parking-app-cli /usr/local/bin/parking-app-cli

ENTRYPOINT ["parking-app-cli"]
