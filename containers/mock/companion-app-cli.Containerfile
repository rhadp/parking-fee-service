# Containerfile for companion-app-cli
#
# Multi-stage build: Go builder -> distroless static runtime
# Build context: repository root

# -- Stage 1: Build ----------------------------------------------------------
FROM docker.io/library/golang:1.22-bookworm AS builder

WORKDIR /build

# Copy Go module files first for dependency caching
COPY mock/companion-app-cli/go.mod mock/companion-app-cli/go.sum* mock/companion-app-cli/

RUN cd mock/companion-app-cli && go mod download

# Copy source code
COPY mock/companion-app-cli/ mock/companion-app-cli/

RUN cd mock/companion-app-cli && \
    CGO_ENABLED=0 GOOS=linux go build -o /out/companion-app-cli .

# -- Stage 2: Runtime --------------------------------------------------------
FROM gcr.io/distroless/static-debian12:nonroot

COPY --from=builder /out/companion-app-cli /usr/local/bin/companion-app-cli

ENTRYPOINT ["companion-app-cli"]
