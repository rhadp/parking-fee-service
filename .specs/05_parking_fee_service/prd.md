# PRD: PARKING_FEE_SERVICE (Phase 2.4)

> Extracted from the master PRD at `.specs/prd.md`. This spec covers Phase 2.4.

## Scope

Implement the PARKING_FEE_SERVICE as a cloud-based Go REST API for parking operator discovery and adapter metadata retrieval.

## Component Description

- Cloud-based service providing:
  - REST API for parking operator discovery and adapter provisioning
  - Location-based lookup of available PARKING_OPERATORs (geofence polygon matching)
  - Adapter metadata retrieval (OCI image reference, SHA-256 checksum)
  - Health check endpoint
- Acts as a gatekeeper for an external OCI registry (Google Artifact Registry); does not run its own registry
- Does NOT manage parking sessions -- session lifecycle is handled by PARKING_OPERATOR_ADAPTOR and PARKING_OPERATOR directly

## REST API Endpoints

- `GET /operators?lat={lat}&lon={lon}` — lookup operators by location
- `GET /operators/{id}/adapter` — get adapter metadata (image_ref, checksum)
- `GET /health` — health check

## Geofence Matching

- Geofence polygons (lat/lon pairs) define operator zones
- Lookups allow fuzziness — "near" a zone counts as a match
- Hardcoded but realistic geofence polygons (data TBD)

## Adapter Metadata

- `image_ref`: OCI image reference in Google Artifact Registry
- `checksum_sha256`: SHA-256 checksum of OCI manifest digest
- `version`: Adapter version string

## Tech Stack

- Language: Go 1.22+
- HTTP framework: standard library net/http or chi router
- Port: 8080

## Dependencies

| Spec | From Group | To Group | Relationship |
|------|-----------|----------|--------------|
| 01_project_setup | 2 | 1 | Requires Go module structure and build system |
