# Errata: Infrastructure Port and Runtime Divergences (Spec 01)

## Kuksa Databroker Port: 55556 vs 55555

**Spec says:** `design.md` for `01_project_setup` specifies Kuksa Databroker on port 55555.

**Implementation uses:** Port 55556, as established by spec `02_data_broker` which determined that Kuksa Databroker 0.5.0 defaults to TCP port 55556.

**Reason:** The `02_data_broker` spec overrides the initial port choice. All downstream specs (04, 08, 09) reference port 55556 consistently. The test scripts in `tests/setup/test_infra.sh` were updated to check for 55556.

## Container Runtime: Podman Support

**Spec says:** `design.md` references Docker and Docker Compose for local infrastructure.

**Implementation:** The Makefile detects `podman` or `docker` at runtime using `$(CONTAINER_RT)` and constructs the compose command accordingly. This supports environments where Docker is not installed but Podman is available (including systems where `docker` is aliased to `podman` in the shell but not available as a binary to Make).

## Go Skeleton Exit Behavior

**Spec says:** Skeleton binaries should exit with code 0 when run without configuration.

**Implementation:** Later specs (e.g., spec 05 for parking-fee-service) added real HTTP server logic. The `parking-fee-service` binary now attempts to bind port 8080 and may exit non-zero if the port is in use. The test scripts in `tests/setup/test_build.sh` were updated to accept binaries that produce startup output (containing "starting") even if they exit non-zero, to accommodate this evolution.
