# Erratum: Databroker CLI Flags and Image Version

**Spec:** 02_data_broker
**Affected requirements:** 02-REQ-1.1, 02-REQ-1.2, 02-REQ-2.1, 02-REQ-3.1, 02-REQ-6.4
**Date:** 2026-04-24

## E02-1: Image version discrepancy

02-REQ-1.1 specifies the image tag as `:0.6`. 02-REQ-1.2 acceptance criterion 2
says the container SHALL report version `0.5.1` in startup logs. The design
document tech stack table says `0.5.1`. These are contradictory.

**Resolution:** The compose.yml uses `ghcr.io/eclipse-kuksa/kuksa-databroker:0.6`
as specified in 02-REQ-1.1. The running container reports package version
`0.6.1`. The acceptance criterion referencing `0.5.1` is incorrect.

## E02-2: TCP listener CLI flags

02-REQ-2.1 acceptance criterion 3 states the command args SHALL include
`--address 0.0.0.0:55555`. Kuksa Databroker 0.6.1 does not accept combined
host:port for `--address`; the correct form is `--address 0.0.0.0 --port 55555`
(two separate flags).

**Resolution:** The compose.yml uses `--address 0.0.0.0` and `--port 55555`
as separate arguments. Static tests verify these flags individually.

## E02-3: UDS listener CLI flag

02-REQ-3.1 acceptance criterion states the command args SHALL include
`--uds-path /tmp/kuksa-databroker.sock`. The `--uds-path` flag does not exist
in Kuksa Databroker 0.6.1; the correct flag is `--unix-socket`.

**Resolution:** The compose.yml uses `--unix-socket /tmp/kuksa-databroker.sock`.
Static tests verify the `--unix-socket` flag.

## E02-4: VSS metadata file

The design document specifies the overlay file as `deployments/vss/overlay.vspec`
(YAML-based VSSpec format). The actual deliverable is `deployments/vss-overlay.json`
(JSON format). Kuksa Databroker 0.6.1 expects JSON-formatted VSS metadata files.

When `--vss` is specified explicitly, the built-in default tree is NOT loaded
automatically. The compose.yml uses
`--vss vss_release_5.1.json,/vss-overlay.json` to load both the bundled
standard VSS 5.1 tree and the custom overlay.

## E02-5: GetMetadata RPC

The test specification pseudocode references `conn.GetMetadata(signal)`. No
`GetMetadata` RPC exists in the Kuksa VAL gRPC API (v1 or v2). Signal metadata
is accessed via `Get` with `VIEW_CURRENT_VALUE` and `FIELD_VALUE`/`FIELD_METADATA`
fields. Tests use this approach instead.

## E02-6: UDS socket host path

The UDS socket is created inside the container at `/tmp/kuksa-databroker.sock`.
With the bind mount `/tmp/kuksa:/tmp`, the socket appears on the host at
`/tmp/kuksa/kuksa-databroker.sock`. Tests check both `/tmp/kuksa/kuksa-databroker.sock`
and `/tmp/kuksa-databroker.sock` via the `effectiveUDSSocket()` helper.
