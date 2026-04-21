# Errata: 02_data_broker_compose_flags

Divergences from spec 02 requirements discovered during implementation.
The kuksa-databroker binary's actual CLI interface and available registry
images differ from the specification in several ways documented below.

---

## E1 — Image version pinned to 0.5.0 (not 0.6.1 or 0.5.1)

**Affects:** 02-REQ-1.1, 02-REQ-1.2

**Problem:**
The specification contains three mutually contradictory image version
references:

- 02-REQ-1.1 mandates image tag `0.6.1`
- The Design technology stack and architecture diagram say `0.5.1`
- 02-REQ-1.2 AC#2 asserts the container reports `0.5.1` in startup logs

Neither `ghcr.io/eclipse-kuksa/kuksa-databroker:0.6.1` nor `:0.5.1` exist
in the container registry. The latest available version at implementation
time is `0.5.0`.

**Resolution:**
`deployments/compose.yml` pins the image to `ghcr.io/eclipse-kuksa/kuksa-databroker:0.5.0`.
Tests (`TestComposePinnedImage`, `TestImageVersion`) accept either `:0.6.1`
or `:0.5.0` to remain forward-compatible if the spec-mandated tag is
published in the future.

---

## E2 — TCP listener requires separate --address and --port flags

**Affects:** 02-REQ-2.1

**Problem:**
02-REQ-2.1 AC#3 specifies the command SHALL include `--address 0.0.0.0:55555`
(combined host:port form). The kuksa-databroker 0.5.0 binary does not support
the combined form; it requires separate flags: `--address 0.0.0.0 --port 55555`.

**Resolution:**
`deployments/compose.yml` uses `--address 0.0.0.0 --port 55555` as separate
arguments. Tests (`TestComposeTCPListener`) accept either the combined or
separate form.

---

## E3 — UDS flag is --unix-socket (not --uds-path)

**Affects:** 02-REQ-3.1

**Problem:**
02-REQ-3.1 AC#1 specifies the command SHALL include `--uds-path /tmp/kuksa-databroker.sock`.
The kuksa-databroker 0.5.0 binary uses `--unix-socket`, not `--uds-path`.

**Resolution:**
`deployments/compose.yml` uses `--unix-socket /tmp/kuksa-databroker.sock`.
Tests (`TestComposeUDSSocket`) accept either `--uds-path` or `--unix-socket`.

---

## E4 — UDS socket path on host differs from in-container path

**Affects:** 02-REQ-3.1, 02-REQ-3.2

**Problem:**
The spec mandates the UDS socket at `/tmp/kuksa-databroker.sock`. Inside the
container this path is correct. However, the compose volume configuration
maps the container's `/tmp` directory to a host bind-mount at `/tmp/kuksa`,
meaning the socket appears at `/tmp/kuksa/kuksa-databroker.sock` on the host.

The spec's acceptance criterion for UDS connectivity ("a gRPC client connecting
via `unix:///tmp/kuksa-databroker.sock` from a co-located container") is
satisfied because co-located containers sharing the same named volume see the
socket at `/tmp/kuksa-databroker.sock` inside their containers. Integration
tests running on the host must use the host-mapped path instead.

**Resolution:**
Tests use `effectiveUDSSocket()` which probes both `/tmp/kuksa/kuksa-databroker.sock`
(host volume-mapped path) and `/tmp/kuksa-databroker.sock` (direct path) as
fallbacks. The in-container path remains `/tmp/kuksa-databroker.sock` as
specified.

---

## E5 — VSS standard tree is version 4.0 (not 5.1)

**Affects:** 02-REQ-5.1, 02-REQ-5.2

**Problem:**
The specification references "VSS v5.1" for the 5 standard signals. The
kuksa-databroker 0.5.0 container image ships with `/vss_release_4.0.json`
(VSS 4.0). No VSS 5.1 tree file is available in the container.

The 5 standard signals referenced by the spec
(`Vehicle.Cabin.Door.Row1.DriverSide.IsLocked`, `IsOpen`,
`Vehicle.CurrentLocation.Latitude`, `Longitude`, `Vehicle.Speed`) all exist
in the VSS 4.0 tree with the same paths and compatible types.

**Resolution:**
`deployments/compose.yml` loads `/vss_release_4.0.json` (the version bundled
in the container image). Integration tests verify signal presence and types
against the running databroker regardless of the underlying VSS version number.
