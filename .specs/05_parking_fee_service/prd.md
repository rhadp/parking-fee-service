# PRD: PARKING_FEE_SERVICE (Phase 2.4)

> Extracted from the master PRD at `.specs/prd.md`. This spec covers Phase 2.4.

## Scope

Implement the PARKING_FEE_SERVICE as a cloud-based Go REST API for parking operator discovery and adapter metadata retrieval.

## Component Description

- Cloud-based service providing:
  - REST API for parking operator discovery and adapter provisioning
  - Location-based lookup of available PARKING_OPERATORs (geofence polygon matching with configurable proximity threshold)
  - Adapter metadata retrieval (OCI image reference, SHA-256 checksum, version)
  - Health check endpoint
- Acts as a gatekeeper for an external OCI registry (Google Artifact Registry); does not run its own registry
- Operator/zone data is hardcoded but realistic (Munich area)
- Does NOT manage parking sessions -- session lifecycle is handled by PARKING_OPERATOR_ADAPTOR and PARKING_OPERATOR directly

## REST API Endpoints

- `GET /operators?lat={lat}&lon={lon}` -- lookup operators by location (geofence polygon matching with proximity threshold from settings)
- `GET /operators/{id}/adapter` -- get adapter metadata (image_ref, checksum_sha256, version)
- `GET /health` -- health check

## Geofence Matching

- Geofence polygons (lat/lon pairs) define operator zones
- Lookups use point-in-polygon matching combined with proximity-based matching
- A configurable proximity threshold (in meters) determines "near zone" matching -- coordinates outside a polygon but within the threshold distance are treated as matches
- The proximity threshold is loaded from the service configuration
- Hardcoded but realistic geofence polygons for Munich area

## Rate Model

Two rate types are supported:

- **per-hour**: Charged by the hour (e.g., 2.50 EUR/hr)
- **flat-fee**: A fixed fee per parking session (e.g., 5.00 EUR)

The data model keeps rate information simple: a rate type enum and a numeric amount with currency.

## Adapter Metadata

- `image_ref`: OCI image reference in Google Artifact Registry
- `checksum_sha256`: SHA-256 checksum of OCI manifest digest
- `version`: Adapter version string

## Configuration

The service loads operator/zone data and settings from a configuration file (JSON) at startup. The configuration includes:

- Proximity threshold in meters (default: 500)
- Server port (default: 8080)
- List of zones with geofence polygon coordinates
- List of operators with zone associations, rate information, and adapter metadata

While the data is hardcoded for this demo, it is structured as loadable configuration to allow easy modification without code changes.

## Tech Stack

- Language: Go 1.22+
- HTTP framework: standard library `net/http` (Go 1.22 `ServeMux` pattern matching)
- No database -- in-memory configuration loaded at startup
- No external Go dependencies beyond standard library
- Port: 8080 (configurable)

## Dependencies

| Spec | From Group | To Group | Relationship |
|------|-----------|----------|--------------|
| 01_project_setup | 2 | 1 | Uses repo structure and Go project skeleton from group 2 |
