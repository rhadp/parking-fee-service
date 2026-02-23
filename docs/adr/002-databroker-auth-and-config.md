# ADR-002: DATA_BROKER Authentication and Configuration

## Status

Accepted

## Context

The RHIVOS safety partition requires DATA_BROKER (Eclipse Kuksa Databroker) to
enforce write-access control via bearer tokens so that only authorized services
can write to specific VSS signals (02-REQ-1.5). Additionally, DATA_BROKER must
serve both standard VSS signals and custom command signals (02-REQ-1.1,
02-REQ-1.2).

Several decisions were needed:

1. **Authentication mechanism:** Kuksa Databroker supports JWT-based
   authorization with per-signal read/write permissions via the `kuksa-vss`
   claim. The design.md mentioned simple bearer token strings (e.g.,
   "locking-service-token"), but Kuksa requires signed JWTs.

2. **VSS file approach:** Kuksa Databroker 0.5.x does not embed a default VSS
   specification. The `--vss` flag loads signal definitions from a file. We
   needed to decide between a minimal overlay (custom signals only) and a
   complete file (standard + custom signals).

3. **Image version:** The design.md specified Kuksa Databroker 0.5.1, but this
   version does not exist. Available versions are 0.5.0 and 0.6.0.

4. **UDS socket sharing:** The design requires a UDS endpoint accessible from
   both containerized databroker and host-side Rust services.

## Decision

### Authentication: Kuksa-native JWT tokens with RS256

We use Kuksa's built-in JWT authorization (`--jwt-public-key` flag) with
RS256-signed tokens. Each service gets a dedicated JWT with fine-grained
per-signal permissions defined in the `kuksa-vss` claim.

- **Key pair:** RSA 2048-bit key pair stored at `infra/kuksa/keys/`
- **Tokens:** Pre-generated JWT files stored at `infra/kuksa/tokens/`
- **Regeneration:** Script at `infra/kuksa/generate-tokens.py`
- **Token summary:** `infra/kuksa/tokens.json` documents all permissions

This is a demo-only setup. The RSA private key is committed to the repository
for reproducibility. In production, keys would be managed by a secrets system.

### VSS file: Single comprehensive file

The `infra/kuksa/vss-overlay.json` file contains both standard VSS signals
(Vehicle.Speed, IsLocked, IsOpen, Latitude, Longitude) and custom command
signals (Vehicle.Command.Door.Lock, Vehicle.Command.Door.Response). This
ensures all required signals are available when Kuksa starts with `--vss`.

### Image version: 0.5.0

We use `ghcr.io/eclipse-kuksa/kuksa-databroker:0.5.0` because version 0.5.1
does not exist. The design.md reference to 0.5.1 is a documentation error.

### UDS socket: Host-mounted directory

A bind mount (`/tmp/kuksa:/tmp/kuksa`) shares a directory between the container
and host. UDS socket support depends on the Kuksa Databroker version supporting
the `--enable-unix-socket` flag. If UDS is not available in 0.5.0, services can
fall back to the TCP endpoint (port 55556) for local development.

## Consequences

- Services must load their JWT token file and pass it as gRPC metadata
  (`authorization: Bearer <token>`) when writing to DATA_BROKER.
- The `databroker-client` crate (task group 3) must support both token-based
  and unauthenticated connections.
- If Kuksa 0.5.0 does not support the `--enable-unix-socket` flag, the UDS
  integration test (TS-02-3) will require either upgrading to 0.6.0 or running
  services alongside the databroker without containerization.
- Token permissions are enforced at the DATA_BROKER level, not by individual
  services. Services simply present their token; DATA_BROKER validates it.
