# Erratum: Service version constant origin

**Spec:** 05_parking_fee_service
**Requirement:** 05-REQ-6.1 (startup logging)
**Design section:** main package, Path 4 (Startup)

## Issue

05-REQ-6.1 requires logging the service "version" at startup, but neither the
Config struct nor any design section specifies where the version string
originates. The requirement is silent on whether the version comes from a
hardcoded constant, build-time ldflags injection, a config field, or an
embedded file.

TS-05-15 does not assert the specific version value, so the test cannot verify
whether version management is correct across releases.

## Resolution

The implementation uses a hardcoded Go constant in `main.go`:

```go
const version = "0.1.0"
```

This constant is logged via `slog.Info` at startup alongside port, zone count,
and operator count. The value follows semantic versioning.

### Rationale

- **Simplicity:** A hardcoded constant requires no build infrastructure
  changes and is immediately readable in the source.
- **Demo scope:** The parking-fee-service is a demo application where
  automated release versioning is not required.
- **Future migration path:** The constant can be replaced with
  `var version string` and `-ldflags "-X main.version=..."` if CI-based
  version injection is needed later, without changing the logging call site.

## Affected code

- `backend/parking-fee-service/main.go`: `const version = "0.1.0"`

## Trade-offs

The version must be updated manually in source code for each release. This is
acceptable for a demo project but would not scale for production services with
automated release pipelines.
