# Errata: DATA_BROKER Compose Flags (Spec 02)

This erratum documents corrections to the original spec 02 requirements and
design documents discovered during implementation and registry validation.

## E1: Image tag 0.5.1 does not exist

**Original spec:** 02-REQ-1.1 specified image tag `:0.5.1`.

**Correction:** Tag `:0.5.1` does not exist in the
`ghcr.io/eclipse-kuksa/kuksa-databroker` container registry. The correct tag
is `:0.5.0`.

**Resolution:** All spec documents updated to reference `:0.5.0`.

## E2: CLI flag format for address and port

**Original spec:** 02-REQ-2 acceptance criteria and design.md specified
`--address 0.0.0.0:55555` (combined host:port).

**Correction:** Kuksa Databroker does not accept the combined `host:port`
format. The correct flags are `--address 0.0.0.0 --port 55555` (separate
arguments).

**Resolution:** All spec documents updated to use the split flag format.

## E3: UDS flag name

**Original spec:** 02-REQ-3.1 acceptance criteria and design.md specified
`--uds-path`.

**Correction:** The actual Kuksa Databroker CLI flag is `--unix-socket`, not
`--uds-path`.

**Resolution:** All spec documents updated to use `--unix-socket`.

## E4: Standard VSS tree must be explicitly loaded

**Original spec:** prd.md C6 stated that standard VSS signals are "included in
the default Kuksa Databroker image" and loaded automatically.

**Correction:** The image bundles the VSS release tree file (e.g.,
`vss_release_4.0.json`) but does not load it unless explicitly passed via the
`--vss` flag. Without it, only overlay signals are available.

**Resolution:** Added 02-REQ-5.3 requiring explicit `--vss` loading of both
the standard tree and custom overlay. Updated prd.md C6 and design.md
accordingly.

## E5: Overlay requires intermediate branch nodes

**Original spec:** prd.md C9 stated "No structural changes to the overlay file
are needed."

**Correction:** The Kuksa Databroker flat JSON format requires explicit branch
node definitions for custom signal paths not present in the standard VSS tree.
The overlay must include `Vehicle.Parking`, `Vehicle.Command`, and
`Vehicle.Command.Door` with `type: "branch"`, or the leaf signals will fail to
load.

**Resolution:** Added 02-REQ-6.5 requiring branch nodes. Updated prd.md C9
and design.md overlay module description.
