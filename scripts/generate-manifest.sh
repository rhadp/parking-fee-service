#!/bin/bash
# scripts/generate-manifest.sh
#
# Generate container manifest with image metadata for validation purposes.
# Extracts git commit hash, build timestamp, and labels from built images.
#
# Usage: ./generate-manifest.sh <image-name> [output-file]
# Example: ./generate-manifest.sh locking-service:latest manifest.json

set -euo pipefail

IMAGE_NAME="${1:-}"
OUTPUT_FILE="${2:-manifest.json}"

if [[ -z "$IMAGE_NAME" ]]; then
    echo "Usage: $0 <image-name> [output-file]"
    echo "Example: $0 locking-service:latest manifest.json"
    exit 1
fi

# Get git metadata
GIT_COMMIT=$(git rev-parse HEAD 2>/dev/null || echo "unknown")
GIT_COMMIT_SHORT=$(git rev-parse --short HEAD 2>/dev/null || echo "unknown")
GIT_BRANCH=$(git rev-parse --abbrev-ref HEAD 2>/dev/null || echo "unknown")
GIT_TAG=$(git describe --tags --exact-match 2>/dev/null || echo "")
GIT_DIRTY=$(git diff --quiet 2>/dev/null && echo "false" || echo "true")

# Build timestamp
BUILD_TIMESTAMP=$(date -u +"%Y-%m-%dT%H:%M:%SZ")

# Determine version from git tag or commit
if [[ -n "$GIT_TAG" ]]; then
    VERSION="$GIT_TAG"
else
    VERSION="$GIT_COMMIT_SHORT"
    if [[ "$GIT_DIRTY" == "true" ]]; then
        VERSION="${VERSION}-dirty"
    fi
fi

# Extract image digest if image exists
IMAGE_DIGEST=""
if command -v podman &>/dev/null; then
    IMAGE_DIGEST=$(podman inspect --format='{{.Digest}}' "$IMAGE_NAME" 2>/dev/null || echo "")
elif command -v docker &>/dev/null; then
    IMAGE_DIGEST=$(docker inspect --format='{{.Id}}' "$IMAGE_NAME" 2>/dev/null || echo "")
fi

# Extract labels from image if it exists
LABELS="{}"
if command -v podman &>/dev/null && podman image exists "$IMAGE_NAME" 2>/dev/null; then
    LABELS=$(podman inspect --format='{{json .Config.Labels}}' "$IMAGE_NAME" 2>/dev/null || echo "{}")
elif command -v docker &>/dev/null && docker image inspect "$IMAGE_NAME" &>/dev/null; then
    LABELS=$(docker inspect --format='{{json .Config.Labels}}' "$IMAGE_NAME" 2>/dev/null || echo "{}")
fi

# Generate manifest JSON
cat > "$OUTPUT_FILE" << EOF
{
  "image_ref": "$IMAGE_NAME",
  "digest": "$IMAGE_DIGEST",
  "version": "$VERSION",
  "git": {
    "commit": "$GIT_COMMIT",
    "commit_short": "$GIT_COMMIT_SHORT",
    "branch": "$GIT_BRANCH",
    "tag": "$GIT_TAG",
    "dirty": $GIT_DIRTY
  },
  "build": {
    "timestamp": "$BUILD_TIMESTAMP"
  },
  "labels": $LABELS
}
EOF

echo "Manifest generated: $OUTPUT_FILE"
echo "  Image: $IMAGE_NAME"
echo "  Version: $VERSION"
echo "  Git Commit: $GIT_COMMIT_SHORT"
