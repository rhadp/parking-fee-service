# Errata: Mock Sensors Proto Compatibility (Spec 09)

## Context

The spec design.md references kuksa.val.v1 as the gRPC API for mock sensors.
However, the production kuksa-databroker (v0.5.0+) exposes only the
kuksa.val.v2.VAL service. Since no v1 service exists in the project, the mock
sensors use kuksa.val.v2 throughout.

## Divergence

| Spec says | Implementation does |
|-----------|-------------------|
| `kuksa.val.v1` gRPC `Set` RPC | `kuksa.val.v2` gRPC `PublishValue` RPC |
| Module path `kuksa::val::v1` | Module path `kuksa::val::v2` |

## Rationale

Using v2 aligns with the existing codebase patterns established by specs 02
(DATA_BROKER), 03 (locking-service), and 04 (cloud-gateway-client), all of
which use the kuksa.val.v2 proto. This avoids maintaining two proto versions.

## Testing Impact

Integration tests use a **stub kuksa.val.v2 gRPC server** that captures
`PublishValue` calls and records the signal path and value for assertion.
This stub is implemented in `tests/mock-apps/sensor_test.go` and
`tests/mock-apps/internal/kuksav2/`.

Tests that need a real DATA_BROKER should be skipped when the v2 service is
unavailable (e.g., in CI environments without a broker container).
