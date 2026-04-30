# Errata: Kuksa Databroker CLI and Image (Spec 02)

## E02-1: Image tag vs package version

| Spec says | Implementation does |
|-----------|---------------------|
| Image `ghcr.io/eclipse-kuksa/kuksa-databroker:0.6` | Same image tag; container self-reports package version **0.6.1** at startup |

### Rationale

The Docker image tag `:0.6` is the latest patch release in the 0.6.x line.
The container binary reports `Package version: 0.6.1` in its startup banner.
The compose.yml pins to `:0.6` per 02-REQ-1.1. Tests that verify the running
version check for the image tag (`:0.6`), not the internal package version.

## E02-2: CLI flag names differ from spec

| Spec says | Binary accepts |
|-----------|----------------|
| `--address 0.0.0.0:55555` (combined host:port) | `--address 0.0.0.0 --port 55555` (separate flags) |
| `--uds-path /tmp/kuksa-databroker.sock` | `--unix-socket /tmp/kuksa-databroker.sock` |

### Rationale

The kuksa-databroker 0.6.1 binary uses `--address` for bind IP only (default
`127.0.0.1`) and `--port` for the TCP port (default `55555`). Combined
`host:port` syntax on `--address` is not supported. The UDS flag is
`--unix-socket`, not `--uds-path`. The compose.yml uses the correct binary
flags.

## E02-3: VSS metadata loading

| Spec says | Implementation does |
|-----------|---------------------|
| Overlay loaded via "appropriate CLI flag or configuration" | Uses `--vss vss_release_5.1.json,/vss-overlay.json` |

### Rationale

When `--vss` is specified explicitly, the databroker does **not** auto-load
the default VSS tree. The bundled file must be included in the comma-separated
list. The bundled VSS 5.1 file is named `vss_release_5.1.json` (confirmed via
the `KUKSA_DATABROKER_METADATA_FILE` env default). The overlay is mounted at
`/vss-overlay.json` (container root) and appended to the `--vss` list.

## E02-4: UDS volume mount path

| Spec says | Implementation does |
|-----------|---------------------|
| "shared volume mount exposing the UDS socket directory" | Bind mount `/tmp/kuksa:/tmp` |

### Rationale

The databroker creates the socket at `/tmp/kuksa-databroker.sock` inside the
container. The bind mount `/tmp/kuksa:/tmp` maps the container's `/tmp` to the
host directory `/tmp/kuksa`, making the socket accessible to both the host
(at `/tmp/kuksa/kuksa-databroker.sock`) and co-located containers that mount
the same volume. The `effectiveUDSSocket()` test helper checks both
`/tmp/kuksa/kuksa-databroker.sock` and `/tmp/kuksa-databroker.sock`.
