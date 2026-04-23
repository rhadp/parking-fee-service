# Errata: Inert Configuration Fields

**Spec:** 07_update_service
**Requirement:** 07-REQ-7.2
**Severity:** Informational

## Observation

07-REQ-7.2 mandates two configuration fields that no requirement, execution
path, or module interface consumes after loading:

- `registry_url` (string) — never read by `InstallAdapter` or any podman
  command. The caller supplies the full OCI image reference, so no
  registry URL prefix is prepended.
- `container_storage_path` (default `/var/lib/containers/adapters/`) —
  never passed to any podman command. Podman uses its own built-in storage
  path, and no code references this field after configuration is loaded.

## Resolution

Both fields are retained in the `Config` struct exactly as specified by
07-REQ-7.2 for forward-compatibility. They can be consumed by future
requirements (e.g., prefixing image references with `registry_url`, or
configuring podman storage via `--root`).

No tests verify these fields influence runtime behavior because there is no
runtime behavior to verify. The config loading tests (TS-07-14, TS-07-E13)
confirm the fields are parsed and defaulted correctly.

## Impact

None. The fields are deserialized, stored, and logged at startup. They have
no effect on service behavior.
