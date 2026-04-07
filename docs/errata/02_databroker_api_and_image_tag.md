# Errata 02: DATA_BROKER API Divergences and Image Tag Correction

This document records deviations between the spec 02 (`02_data_broker`) and the
actual implementation. The spec documents are preserved unchanged; this errata
supersedes them for implementation purposes.

---

## E02-1: Kuksa Databroker image tag 0.5.1 does not exist

**Affected spec locations:**
- `requirements.md` 02-REQ-1.1: "image as `ghcr.io/eclipse-kuksa/kuksa-databroker:0.5.1`"
- `design.md` Compose service interface table
- `test_spec.md` TS-02-3

**Reality:** Tag `0.5.1` is not published on `ghcr.io/eclipse-kuksa/kuksa-databroker`.
The most recent available tag at the time of implementation is `0.5.0`.

**Correction:** All compose configuration, container inspection, and image-version
tests use `ghcr.io/eclipse-kuksa/kuksa-databroker:0.5.0`.

---

## E02-2: `GetMetadata` RPC does not exist in the VAL gRPC API

**Affected spec locations:**
- `requirements.md` 02-REQ-5.2: "GetMetadata or equivalent introspection call"
- `test_spec.md` TS-02-1, TS-02-2, TS-02-4, TS-02-5, TS-02-P1, TS-02-SMOKE-1,
  TS-02-SMOKE-2 (all use `GetMetadata` in pseudocode)

**Reality:** The project's VAL gRPC service (defined in `proto/kuksa/val.proto`)
exposes three RPCs: `Get`, `Set`, and `Subscribe`. There is no `GetMetadata` RPC.

**Correction:** Signal existence is verified using the `Get` RPC. A successful Get
for a signal path (even if the signal has no value yet) proves the path is
registered in the databroker's VSS tree. Type compatibility is verified by
performing a `Set` with a zero value of the expected type and checking
`SetResponse.success == true`.

---

## E02-3: UDS CLI flag is `--unix-socket`, not `--uds-path`

**Affected spec locations:**
- `requirements.md` 02-REQ-3.1 acceptance criteria: "command args SHALL include
  `--uds-path /tmp/kuksa-databroker.sock`"
- `design.md` execution path and compose service interface

**Reality:** The Kuksa Databroker 0.5.0 CLI does not have a `--uds-path` flag.
The correct flag is `--unix-socket`.

**Correction:** The compose.yml command args use `--unix-socket /tmp/kuksa-databroker.sock`.

---

## E02-4: Local gRPC client uses project-internal proto package

**Affected spec locations:**
- `design.md` Technology Stack: "kuksa-client or grpcurl … latest"
- `test_spec.md` (implicitly assumes a standard Kuksa gRPC client)

**Reality:** The project defines its own simplified `kuksa.VAL` gRPC service in
`proto/kuksa/val.proto` (generated code in `gen/go/kuksa/`). The integration
tests use the project's generated Go client rather than an external `kuksa-client`
tool, because the Go module path `parking-fee-service/gen/go` cannot be required
by other modules (it lacks a dot in its first path element). The generated
proto files are embedded locally in `tests/databroker/kuksa/` to avoid the
cross-module dependency issue.

**Correction:** Tests import `parking-fee-service/tests/databroker/kuksa` (a local
copy of the generated proto types) and use `grpc.NewClient` for TCP and UDS
connections. This is functionally equivalent to using a Kuksa gRPC client.

---

## E02-5: Subscription may deliver initial value on subscribe

**Affected spec locations:**
- `test_spec.md` TS-02-P4: "no more = stream.Recv(timeout=1s) / assert no_more == TIMEOUT"

**Reality:** Kuksa Databroker typically delivers the current signal value
immediately upon subscription establishment (initial-value notification).

**Correction:** TS-02-P4 implementation (`TestSubscriptionDelivery`) drains any
initial-value notification via a `drainInitial()` helper (500 ms window) before
performing the test write. This prevents false failures from initial-value
notifications being misidentified as write-triggered updates. The "exactly once"
assertion is applied only to updates received after the drain.
