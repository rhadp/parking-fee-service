# Erratum: Version String Source (05-REQ-6.1)

## Requirement

05-REQ-6.1 mandates that the service logs its "version" at startup, but
neither the requirements nor the design document specifies where the version
string originates (hardcoded constant, build-time injection via ldflags,
environment variable, etc.).

## Implementation Decision

The implementation uses a hardcoded Go constant:

```go
const version = "0.1.0"
```

This is sufficient for the demo context and avoids build-system complexity.
The version is logged at startup via `log/slog` alongside port, zone count,
and operator count.

## Rationale

- The service is a demo component; no release pipeline or automated
  versioning is defined in the spec.
- A hardcoded constant is the simplest approach that satisfies the
  requirement to log a version string.
- If build-time injection is needed later, the constant can be replaced
  with a `var` set via `-ldflags "-X main.version=..."`.

## Impact

Low. The version string is purely informational and does not affect
functional behavior. No downstream consumers parse or depend on the
version value.
