# Errata: DATA_BROKER Kuksa 0.5.0 API Behavior

**Spec:** 02_data_broker (design.md, test_spec.md)
**Date:** 2026-03-11
**Kuksa Databroker Version:** 0.5.0

## Findings

### 1. gRPC Health Check Not Implemented

**Spec assumption:** The DATA_BROKER supports standard gRPC health checks (02-REQ-8.1, 02-REQ-8.2).

**Actual behavior:** Kuksa Databroker 0.5.0 does not implement the standard `grpc.health.v1.Health` service. Health check requests return `UNIMPLEMENTED`.

**Workaround:** Use `ListMetadata` as a health/readiness probe. A successful `ListMetadata("Vehicle")` response with non-zero metadata entries indicates the databroker is healthy and ready.

**Test impact:** `TestDataBrokerHealthDuringStartup` (TS-02-E4) falls back to `ListMetadata` when the gRPC health service returns `UNIMPLEMENTED`.

### 2. Type Mismatch Strictly Rejected

**Spec assumption:** Some Kuksa versions may silently coerce types (noted in TS-02-E3).

**Actual behavior:** Kuksa 0.5.0 strictly rejects type-mismatched writes. Writing a string value to a boolean signal returns a gRPC error (not `INVALID_ARGUMENT` code but a rejection nonetheless).

**Test impact:** `TestDataBrokerTypeMismatch` (TS-02-E3) passes -- the write is rejected as expected.

### 3. Unset Signal Returns Empty Datapoint

**Spec assumption:** Reading an unset signal returns metadata with no current value (02-REQ-3.2).

**Actual behavior:** Kuksa 0.5.0 returns a `Datapoint` with a nil/empty `Value` field for signals that have never been written to. This matches the spec expectation.

**Test impact:** `TestDataBrokerUnsetSignal` (TS-02-E2) uses `Vehicle.CurrentLocation.Altitude` (a signal not written by any other test) to validate this behavior.

### 4. Non-Existent Signal Returns NOT_FOUND

**Spec assumption:** Get/set on non-existent signals returns `NOT_FOUND` (02-REQ-6.E1).

**Actual behavior:** Kuksa 0.5.0 returns `NOT_FOUND` for both `GetValue` and `PublishValue` on paths not in the VSS tree. This matches the spec.

### 5. Subscription Initial Message

**Spec assumption:** Subscribers receive updates when values change (02-REQ-7.1).

**Actual behavior:** Kuksa 0.5.0 sends an initial message on subscription containing the current value (or empty if unset). Subsequent value changes trigger additional messages. Tests must consume this initial message before waiting for update messages.

### 6. VSS Bundled Version

**Spec assumption:** The DATA_BROKER loads VSS v5.1 (design.md).

**Actual behavior:** Kuksa Databroker 0.5.0 bundles VSS v4.0 (`vss_release_4.0.json`). All 5 standard signals specified in 02-REQ-3.1 exist in VSS v4.0, so there is no functional impact. The `--vss` flag accepts a comma-separated list to load both the bundled VSS and the custom overlay.

### 7. UDS Not Accessible from macOS Host

**Spec assumption:** UDS endpoint available at `/tmp/kuksa/databroker.sock` (02-REQ-4.1).

**Actual behavior:** On macOS, Podman runs containers in a Linux VM. Named volumes (`kuksa-uds`) are not directly accessible from the macOS host filesystem. UDS tests are skipped on macOS with `runtime.GOOS == "darwin"`. On Linux hosts and within containers, UDS works as expected.

**Test impact:** `TestDataBrokerUDSAccess` (TS-02-5) skips on macOS unless `FORCE_UDS_TEST=1` is set.
