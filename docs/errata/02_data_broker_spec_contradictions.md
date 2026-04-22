# Errata: Spec 02 — DATA_BROKER Spec Contradictions

## 1. Image Version

The spec documents contain mutually contradictory image version references:

- `requirements.md` (02-REQ-1.1): mandates tag `0.6.1`
- `design.md`: references version `0.5.1` in text and `0.6.1` in image ref
- `tasks.md` errata notes: records `0.5.0` as the available version

**Resolution:** Static tests (`TestComposePinnedImage`) verify that the image
is pinned to a specific version (not `:latest`) without asserting a particular
version number. The live test (`TestImageVersion`) checks the running container
uses a pinned version. Task group 2 will pin to whichever version is actually
available in the registry.

## 2. CLI Flag Format

The acceptance criteria specify `--address 0.0.0.0:55555` (combined form) and
`--uds-path` for the UDS socket. The actual kuksa-databroker binary uses:

- `--address 0.0.0.0 --port 55555` (separate flags)
- `--unix-socket` (not `--uds-path`)

**Resolution:** Static tests accept both flag formats:
- `TestComposeTCPListener` accepts either `--address 0.0.0.0:55555` or
  `--address 0.0.0.0` + `--port 55555`
- `TestComposeUDSSocket` accepts either `--uds-path` or `--unix-socket`

## 3. UDS Socket Path (Host vs Container)

The requirements specify UDS path `/tmp/kuksa-databroker.sock`. Inside the
container this is correct, but the host-accessible path depends on the volume
mount configuration. With a bind mount of `/tmp/kuksa` to `/tmp` in the
container, the host path becomes `/tmp/kuksa/kuksa-databroker.sock`.

**Resolution:** UDS connectivity tests check both paths:
- `/tmp/kuksa/kuksa-databroker.sock` (bind mount layout)
- `/tmp/kuksa-databroker.sock` (direct mount layout)

## 4. Overlay File Format and Path

The spec documents reference three different overlay formats/paths:
- Glossary: "A YAML file"
- `design.md`: `deployments/vss/overlay.vspec`
- `tasks.md`: `deployments/vss-overlay.json`

**Resolution:** Tests use `deployments/vss-overlay.json` (JSON format), which
is the actual file that exists in the repository and is used by the compose.yml
volume mount.

## 5. VSS Version

The spec references VSS v5.1, but the available kuksa-databroker image bundles
VSS 4.0 (`/vss_release_4.0.json`). The 5 standard signal paths
(Vehicle.Speed, Vehicle.Cabin.Door.*, Vehicle.CurrentLocation.*) exist in
both VSS versions with the same paths and types.

**Resolution:** Tests verify signal existence and types without asserting
the VSS version number.

## 6. Subscription Delivery Semantics

TS-02-P4 asserts "exactly once" delivery, but the kuksa-databroker typically
delivers an initial current-value event on subscription establishment. The
requirement (02-REQ-10.1) only says "deliver update notifications when the
signal value changes" without multiplicity constraints.

**Resolution:** Subscription tests verify that at least one update with the
expected value is received, using a loop that handles initial current-value
events. No "exactly once" assertion is made.
