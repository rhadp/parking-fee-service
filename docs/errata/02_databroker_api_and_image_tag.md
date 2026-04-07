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

## E02-4: Kuksa v2 API uses different service path and RPC methods

**Affected spec locations:**
- `design.md` Technology Stack: "kuksa-client or grpcurl … latest"
- `test_spec.md` (all pseudocode uses `Get`, `Set`, `Subscribe` methods)
- `proto/kuksa/val.proto` (original spec-derived proto)

**Reality:** Kuksa Databroker 0.5.0 exposes the `kuksa.val.v2.VAL` gRPC service,
not `kuksa.VAL`. The actual API methods are `GetValue`, `GetValues`,
`PublishValue`, `Subscribe`, `ListMetadata`, and `GetServerInfo`. The data model
uses `Value` (with `typed_value` oneof) nested inside `Datapoint`, and
`SignalID` for signal identification. Subscribe responses use
`map<string, Datapoint>` instead of `repeated DataEntry`.

**Correction:** The proto file (`proto/kuksa/val.proto`) was rewritten to match
the actual `kuksa.val.v2` API. Generated Go code in `tests/databroker/kuksa/`
uses the correct service path and message types. All test helpers were updated:
`Set` → `PublishValue`, `Get` → `GetValue`, data access via `Value.TypedValue`.

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

---

## E02-6: Standard VSS signals are not built-in to Kuksa 0.5.0

**Affected spec locations:**
- `requirements.md` 02-REQ-5.1: states standard signals are present in the
  Kuksa built-in VSS tree
- `design.md` execution path step 4: "Kuksa loads the default VSS v5.1 tree"

**Reality:** Kuksa Databroker 0.5.0 does not include a built-in VSS tree. The
`--vss` flag specifies the only signal source. If no VSS file is provided, no
signals are available.

**Correction:** The VSS overlay file (`deployments/vss-overlay.json`) was expanded
to include all 8 required signals (5 standard + 3 custom). This ensures all
signals are available when the databroker loads the overlay.

---

## E02-7: UDS tests require Linux or bind-mounted socket volume

**Affected spec locations:**
- `test_spec.md` TS-02-2, TS-02-7, TS-02-8, TS-02-9, TS-02-11, TS-02-P3:
  assume UDS socket is accessible at `/tmp/kuksa-databroker.sock` from the host

**Reality:** The UDS socket is inside the `kuksa-uds` named volume, which is
stored in the podman VM on macOS. Unix domain sockets do not cross VM boundaries,
so UDS tests cannot connect from the macOS host.

**Correction:** UDS tests skip gracefully when the socket file is not accessible.
On Linux hosts where the volume is directly accessible, UDS tests run normally.
The compose.yml correctly configures the shared volume for container-to-container
UDS communication (the production use case).
