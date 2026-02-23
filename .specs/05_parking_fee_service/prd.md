# PRD: Parking Fee Service (Phase 2.4)

> Extracted from the main PRD (`.specs/prd.md`) — Phase 2.4: Parking Fee Service.

## Context

This specification covers the implementation of the PARKING_FEE_SERVICE, the
cloud-based backend service responsible for parking operator discovery and
adapter provisioning metadata. The PARKING_FEE_SERVICE enables vehicles to
discover which parking operators are available at their current location and
retrieve the metadata needed to install the corresponding adapter.

The service uses geofence polygon matching to determine which operators cover
a given GPS coordinate, with fuzziness support so that vehicles near (but not
exactly within) a zone boundary are still matched.

This phase also enhances the mock PARKING_APP CLI to call the
PARKING_FEE_SERVICE for operator lookups and adapter metadata retrieval,
enabling integration testing of the full discovery flow.

## Scope

From the main PRD, Phase 2.4:

- Implementation of the PARKING_FEE_SERVICE (operator discovery and adapter
  metadata only)
- Integration test of PARKING_APP mock, PARKING_FEE_SERVICE, UPDATE_SERVICE
  communication

### PARKING_FEE_SERVICE (Go)

- REST API for parking operator discovery and adapter provisioning
- Endpoints:
  - `GET /operators?lat={lat}&lon={lon}` — Lookup operators by vehicle
    location. Uses geofence polygon matching. Allows fuzziness ("near" a zone
    counts). Returns list of matching operators with zone info and rates.
  - `GET /operators/{id}/adapter` — Get adapter metadata: OCI image reference,
    SHA-256 checksum, version. Acts as gatekeeper for Google Artifact Registry
    (does not run its own registry).
  - `GET /health` — Health check endpoint.
- Geofence matching: Hardcoded but realistic geofence polygons (clarification
  IA4). Point-in-polygon test with fuzziness buffer.
- Data: Operator records with zone polygons, rates, adapter image references.
  Stored in-memory or JSON config (demo scope, no database).
- Authentication: Bearer tokens (demo, clarification U4).

### Mock PARKING_APP CLI Enhancements

- `lookup` command calls PARKING_FEE_SERVICE to discover operators by location
- `adapter` command calls PARKING_FEE_SERVICE to retrieve adapter metadata
- Displays returned operator list with zone and rate info

### Integration Tests

- PARKING_APP CLI to PARKING_FEE_SERVICE operator lookup by location
- PARKING_APP CLI to PARKING_FEE_SERVICE adapter metadata retrieval
- Full flow: lookup operators, get adapter info (would feed into
  UPDATE_SERVICE InstallAdapter)

## Key Design Constraints

- **No session management** (clarification A7): PARKING_FEE_SERVICE does NOT
  manage parking sessions. Session lifecycle is handled by
  PARKING_OPERATOR_ADAPTOR and PARKING_OPERATOR directly.
- **No registry**: PARKING_FEE_SERVICE acts as a gatekeeper for Google Artifact
  Registry (clarification A3). It provides adapter metadata (image reference,
  checksum) but does not host or proxy container images.
- **Hardcoded zones**: Geofence polygons are hardcoded but realistic
  (clarification IA4). Data is stored in-memory or loaded from a JSON config
  file.
- **Demo authentication**: Bearer tokens (clarification U4). No real identity
  provider; tokens are static or configured.
- **Multi-vehicle support**: Different vehicles may query different zones
  simultaneously. The service is stateless with respect to vehicles.

## Technology Stack

- Language: Go (1.22+)
- HTTP framework: Standard library `net/http`
- Data storage: In-memory with optional JSON config file
- Geospatial: Custom point-in-polygon implementation (no external GIS library
  for demo scope)
- Testing: Go standard testing package, `net/http/httptest`

## Out-of-Scope for This Spec

- Parking session management (start, stop, fee calculation)
- Real OCI registry integration or image pulling
- Production authentication/authorization
- Database-backed operator storage
- Operator onboarding or approval workflows
- Real payment processing
- Android PARKING_APP implementation (Phase 2.5)

## Dependencies

| Spec | Relationship | Notes |
|------|-------------|-------|
| 01_project_setup | Depends on | Uses Go module skeleton, Makefile, go.work |
| 04_rhivos_qm | Integrates with | Adapter metadata feeds into UPDATE_SERVICE InstallAdapter |
