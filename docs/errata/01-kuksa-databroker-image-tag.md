# Errata: Kuksa Databroker Image Tag

**Spec:** 01_project_setup (design.md)
**Date:** 2026-03-11

## Divergence

The design document specifies the Kuksa Databroker image as:

```
ghcr.io/eclipse-kuksa/kuksa-databroker:master
```

The actual container registry does not publish a `master` tag. The correct tag is `main`:

```
ghcr.io/eclipse-kuksa/kuksa-databroker:main
```

## Rationale

The `master` tag does not exist in the `ghcr.io/eclipse-kuksa/kuksa-databroker` registry. The project uses `main` as its default branch and publishes images under the `main` tag. Using `master` causes a pull failure.

## Impact

`deployments/compose.yml` uses `ghcr.io/eclipse-kuksa/kuksa-databroker:main` instead of the spec-prescribed `:master` tag.
