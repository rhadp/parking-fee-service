# Implementation Tasks: DATA_BROKER (Spec 02)

> Task breakdown for deploying and configuring Eclipse Kuksa Databroker.

## References

- Requirements: `.specs/02_data_broker/requirements.md`
- Design: `.specs/02_data_broker/design.md`
- Test Specification: `.specs/02_data_broker/test_spec.md`

## Dependencies

| Spec | From Group | To Group | Relationship |
|------|-----------|----------|--------------|
| 01_project_setup | Groups 2-4 | Group 1 | Requires repo structure, docker-compose base, Makefile with `infra-up`/`infra-down` targets, and Go test module in `tests/setup/` |

## Test Commands

| Command | Purpose |
|---------|---------|
| `cd tests/setup && go test -run TestDataBroker -v` | Run DATA_BROKER spec tests |
| `make infra-up` | Start local infrastructure (docker-compose) |
| `make infra-down` | Stop local infrastructure |

---

## Group 1: Write Failing Spec Tests

**Goal:** Write Go tests that validate DATA_BROKER deployment and configuration. All tests MUST fail initially (no infrastructure exists yet).

**Depends on:** 01_project_setup (Go test module in `tests/setup/` must exist)

### Task 1.1: Create DATA_BROKER test file

Create `tests/setup/databroker_test.go` with the following test functions. Each test connects to `localhost:55556` via gRPC and validates one aspect of the DATA_BROKER configuration.

- `TestDataBrokerHealth` -- Verify gRPC connectivity to `localhost:55556` and successful metadata query. (TS-02-1)
- `TestDataBrokerStandardSignals` -- Query metadata for all 5 standard VSS signals, verify each exists with the correct data type. (TS-02-2)
- `TestDataBrokerCustomSignals` -- Query metadata for all 3 custom overlay signals, verify each exists with the correct data type. (TS-02-3)
- `TestDataBrokerWriteReadRoundTrip` -- For each of the 8 signals, write a test value and read it back, verifying the round-trip. Use table-driven subtests. (TS-02-P1)
- `TestDataBrokerSubscription` -- Subscribe to a signal, write a value from a separate goroutine, verify the subscriber receives the update within 5 seconds. (TS-02-P2)
- `TestDataBrokerCrossPartitionAccess` -- Perform a metadata query and a write/read round-trip over TCP on `localhost:55556` from the test host. (TS-02-4)
- `TestDataBrokerNonExistentSignal` -- Attempt get and set on `Vehicle.NonExistent.Signal`, verify NOT_FOUND error. (TS-02-E1)
- `TestDataBrokerUnsetSignal` -- Read a signal that has not been written since startup, verify no current value. (TS-02-E2)

### Task 1.2: Add Kuksa gRPC client dependency

Add the necessary Go dependencies for connecting to Kuksa Databroker via gRPC. Options:

- Use the official Kuksa client Go library if available, OR
- Use raw gRPC with the Kuksa proto definitions (generate Go code from `.proto` files), OR
- Use `grpcurl`-style dynamic reflection if Kuksa supports gRPC reflection.

Update `tests/setup/go.mod` and `tests/setup/go.sum` accordingly.

### Task 1.3: Verify all tests fail

Run `cd tests/setup && go test -run TestDataBroker -v` and confirm all tests fail with connection refused or similar errors (DATA_BROKER is not yet running).

**Exit criteria:** All `TestDataBroker*` tests exist, compile, and fail because no DATA_BROKER is available.

---

## Group 2: VSS Overlay Configuration

**Goal:** Create the VSS overlay JSON file that defines the 3 custom signals.

**Depends on:** None (can proceed in parallel with Group 1)

### Task 2.1: Create overlay directory structure

Create the directory `config/vss/` in the project root if it does not already exist.

### Task 2.2: Create VSS overlay file

Create `config/vss/vss_overlay.json` with the following content:

```json
{
  "Vehicle": {
    "type": "branch",
    "children": {
      "Parking": {
        "type": "branch",
        "description": "Parking-related signals for the parking demo.",
        "children": {
          "SessionActive": {
            "type": "actuator",
            "datatype": "boolean",
            "description": "Indicates whether a parking session is currently active. Written by PARKING_OPERATOR_ADAPTOR."
          }
        }
      },
      "Command": {
        "type": "branch",
        "children": {
          "Door": {
            "type": "branch",
            "children": {
              "Lock": {
                "type": "actuator",
                "datatype": "string",
                "description": "Lock/unlock command request as JSON. Written by CLOUD_GATEWAY_CLIENT."
              },
              "Response": {
                "type": "actuator",
                "datatype": "string",
                "description": "Command execution result as JSON. Written by LOCKING_SERVICE."
              }
            }
          }
        }
      }
    }
  }
}
```

### Task 2.3: Validate overlay JSON

Verify the overlay JSON is syntactically valid (e.g., `python3 -m json.tool config/vss/vss_overlay.json`).

**Exit criteria:** `config/vss/vss_overlay.json` exists and is valid JSON matching the design spec.

---

## Group 3: Databroker Container Configuration

**Goal:** Add the Kuksa Databroker service to docker-compose and wire up the overlay file, ports, and health check.

**Depends on:** 01_project_setup (docker-compose base file must exist), Group 2 (overlay file)

### Task 3.1: Add databroker service to docker-compose

Add the `databroker` service to the project's `docker-compose.yml` (or `docker-compose.override.yml` if the project uses overrides):

- **Image:** `ghcr.io/eclipse-kuksa/kuksa-databroker:latest` (pin to a specific tag once validated)
- **Container name:** `databroker`
- **Ports:** Map `55556:55556`
- **Volumes:** Mount `./config/vss/vss_overlay.json` to the container path expected by Kuksa (e.g., `/config/vss/vss_overlay.json`)
- **Command:** Configure `--address 0.0.0.0 --port 55556` and the overlay flag
- **Health check:** TCP check on port 55556 or gRPC health probe
- **Restart policy:** `unless-stopped`

### Task 3.2: Validate container startup

Run `make infra-up` and verify:

1. The `databroker` container starts without errors.
2. The container reaches a healthy state.
3. Port 55556 is accessible from the host.
4. Container logs show successful VSS tree loading and overlay application.

### Task 3.3: Validate container teardown

Run `make infra-down` and verify the `databroker` container is stopped and removed cleanly.

**Exit criteria:** `make infra-up` starts a healthy Kuksa Databroker container; `make infra-down` stops it cleanly.

---

## Group 4: Verification Scripts and Health Checks

**Goal:** Create helper scripts and verify end-to-end functionality.

**Depends on:** Group 3 (running databroker container)

### Task 4.1: Run spec tests against running infrastructure

Execute `cd tests/setup && go test -run TestDataBroker -v` against the running infrastructure and fix any test issues.

Iterate until all tests pass. Common issues to investigate:

- Kuksa API differences (proto versions, method names)
- Signal path case sensitivity
- Overlay loading behavior
- Data type mapping between Go and Kuksa

### Task 4.2: Create verification script (optional)

If useful, create a lightweight shell script `scripts/verify-databroker.sh` that:

1. Checks that the `databroker` container is running and healthy.
2. Uses `grpcurl` or `kuksa-client` CLI to query a signal.
3. Prints a summary of registered signals.

This is a convenience tool for developers, not a required deliverable.

### Task 4.3: Document any Kuksa version-specific findings

If the Kuksa Databroker version used differs from assumptions in the design (e.g., different CLI flags, different proto API, UDS support), document the findings and update the design document accordingly.

**Exit criteria:** All `TestDataBroker*` tests pass; any version-specific issues are documented.

---

## Group 5: Checkpoint

**Goal:** Validate all requirements are met and the spec is complete.

**Depends on:** Groups 1-4

### Task 5.1: Full test run

Run the complete test suite and verify all tests pass:

```
make infra-up
cd tests/setup && go test -run TestDataBroker -v
make infra-down
```

### Task 5.2: Requirements verification

Verify each requirement is covered by at least one passing test:

| Requirement | Test(s) | Status |
|-------------|---------|--------|
| 02-REQ-1.1 | TS-02-1 | |
| 02-REQ-1.2 | TS-02-1 | |
| 02-REQ-2.1 | TS-02-2 | |
| 02-REQ-2.2 | TS-02-E2 | |
| 02-REQ-3.1 | TS-02-3 | |
| 02-REQ-3.2 | TS-02-E1 | |
| 02-REQ-4.1 | TS-02-4 (via TCP; UDS validated by design) | |
| 02-REQ-4.2 | TS-02-P1 | |
| 02-REQ-5.1 | TS-02-4 | |
| 02-REQ-5.2 | TS-02-P1, TS-02-4 | |
| 02-REQ-6.1 | TS-02-P1 | |
| 02-REQ-6.2 | TS-02-P1 | |
| 02-REQ-6.3 | TS-02-P2 | |

### Task 5.3: Definition of Done checklist

- [ ] Kuksa Databroker container starts via `make infra-up`
- [ ] Health check passes within 30 seconds
- [ ] All 5 standard VSS signals queryable via gRPC on port 55556
- [ ] All 3 custom overlay signals queryable via gRPC on port 55556
- [ ] Signal write/read round-trip works for bool, float, double, string
- [ ] Pub/sub subscription delivers value updates
- [ ] All `TestDataBroker*` tests pass
- [ ] `make infra-down` cleans up the container

**Exit criteria:** All checklist items are confirmed; spec 02 is complete.

---

## Traceability Matrix

| Requirement | Design Section | Test Spec | Task Group |
|-------------|---------------|-----------|------------|
| 02-REQ-1.1 | Docker-Compose Integration | TS-02-1 | Group 3 |
| 02-REQ-1.2 | Docker-Compose Integration | TS-02-1 | Group 3 |
| 02-REQ-1.3 | Error Handling | TS-02-1 | Group 3 |
| 02-REQ-2.1 | Technology Stack, VSS Overlay | TS-02-2 | Group 3 |
| 02-REQ-2.2 | Correctness Properties (CP-2) | TS-02-E2 | Group 1 |
| 02-REQ-3.1 | VSS Overlay Configuration | TS-02-3 | Group 2 |
| 02-REQ-3.2 | Error Handling | TS-02-E1 | Group 1 |
| 02-REQ-4.1 | Network Configuration (UDS) | TS-02-4 | Group 3 |
| 02-REQ-4.2 | Network Configuration (UDS) | TS-02-P1 | Group 1 |
| 02-REQ-4.3 | Error Handling | TS-02-E1 | Group 3 |
| 02-REQ-5.1 | Network Configuration (TCP) | TS-02-4 | Group 3 |
| 02-REQ-5.2 | Network Configuration (TCP) | TS-02-P1, TS-02-4 | Group 1 |
| 02-REQ-5.3 | Error Handling | TS-02-1 | Group 3 |
| 02-REQ-6.1 | Access Control Model | TS-02-P1 | Group 1 |
| 02-REQ-6.2 | Access Control Model | TS-02-P1 | Group 1 |
| 02-REQ-6.3 | Correctness Properties (CP-3) | TS-02-P2 | Group 1 |
