# DATA_BROKER Configuration Deltas

Differences between `design.md` and actual implementation for task group 2.

## Image Version

- **Design:** `ghcr.io/eclipse-kuksa/kuksa-databroker:0.5.1`
- **Actual:** `ghcr.io/eclipse-kuksa/kuksa-databroker:0.5.0`
- **Reason:** Version 0.5.1 does not exist on ghcr.io. 0.5.0 is the closest
  available version in the 0.5.x line.

## VSS Overlay File Scope

- **Design:** Overlay file contains only custom signals (Vehicle.Command.Door.*)
- **Actual:** File contains both standard signals (Vehicle.Speed, IsLocked,
  IsOpen, Latitude, Longitude) and custom signals
- **Reason:** Kuksa Databroker 0.5.x does not embed a default VSS specification.
  The `--vss` flag loads signal definitions from the provided file. Without
  standard signals in the file, they would not be available in DATA_BROKER.

## Token Configuration Format

- **Design:** Simple static bearer token strings (e.g., "locking-service-token")
  stored in `infra/kuksa/tokens.json`
- **Actual:** RS256-signed JWT tokens with `kuksa-vss` claims, stored as
  individual `.token` files in `infra/kuksa/tokens/`
- **Reason:** Kuksa Databroker uses JWT-based authorization natively. The
  `--jwt-public-key` flag requires JWTs signed with a corresponding private key.
  Simple string tokens are not supported. See ADR-002 for details.

## UDS Socket Configuration

- **Design:** Direct bind-mount of socket file at `/tmp/kuksa-databroker.sock`
- **Actual:** Bind-mount of directory `/tmp/kuksa:/tmp/kuksa`; UDS socket
  support depends on Kuksa 0.5.0 having `--enable-unix-socket` flag
- **Reason:** Bind-mounting a non-existent socket file creates a directory
  instead. The directory mount approach is standard for container UDS sharing.
  The actual UDS path will be `/tmp/kuksa/databroker.sock` if the flag is
  available.
