# PRD: PARKING_FEE_SERVICE (Phase 2.4 — Backend)

> Extracted from the [main PRD](../prd.md). This spec covers Phase 2.4
> (backend portion): the PARKING_FEE_SERVICE cloud service and mock
> PARKING_APP CLI extensions for testing it.

## Scope

From the main PRD, Phase 2.4 (backend scope only):

- Implement PARKING_FEE_SERVICE (Go): REST API for zone/operator lookup by
  location, adapter metadata retrieval, health check.
- Extend mock PARKING_APP CLI (Go): add subcommands for PARKING_FEE_SERVICE
  endpoints.
- Integration test of zone lookup → adapter metadata → UPDATE_SERVICE install
  flow.

### Components in scope

| Component | Work | Language |
|-----------|------|----------|
| PARKING_FEE_SERVICE | Full implementation | Go |
| Mock PARKING_APP CLI | Extend with PFS subcommands | Go |

### Out of scope for this spec

- **Android PARKING_APP (AAOS):** Separate spec.
- **Session management:** Handled by PARKING_OPERATOR_ADAPTOR (spec 04).
- **OCI registry pull:** UPDATE_SERVICE uses pre-built local images (spec 04).
  PARKING_FEE_SERVICE provides metadata only.

### Zone lookup flow

```
PARKING_APP (or mock CLI)
    │
    ▼
PARKING_FEE_SERVICE
    ├─ GET /api/v1/zones?lat=X&lon=Y
    │   → returns matching zones (point-in-polygon + fuzzy radius)
    │
    ├─ GET /api/v1/zones/{zone_id}
    │   → returns zone details (operator, rate, geofence)
    │
    └─ GET /api/v1/zones/{zone_id}/adapter
        → returns adapter metadata (image_ref, checksum)

PARKING_APP uses adapter metadata to call:
    UPDATE_SERVICE.InstallAdapter(image_ref, checksum)
```

### Zone data model

Each zone is 1:1 with an operator. Zones are hardcoded in the service with
realistic Munich coordinates. The data model embeds operator and adapter
info directly:

| Field | Type | Description |
|-------|------|-------------|
| `zone_id` | string | Unique zone identifier |
| `name` | string | Human-readable zone name |
| `operator_name` | string | Parking operator name |
| `polygon` | []LatLon | Geofence polygon coordinates |
| `adapter_image_ref` | string | OCI image reference for the adapter |
| `adapter_checksum` | string | SHA256 checksum of the adapter image |
| `rate_type` | string | `"per_minute"` or `"flat"` |
| `rate_amount` | float64 | Rate value |
| `currency` | string | Currency code (EUR) |

### Location matching

1. **Exact match:** Point-in-polygon test for each zone's geofence.
2. **Fuzzy match:** If no polygon contains the point, find zones within 200m
   of the nearest polygon edge.
3. **Multiple matches:** Return all matching zones, sorted by distance.

### Demo zones (Munich)

Three hardcoded zones with realistic geofence polygons:

1. **zone-marienplatz** — Central Munich (Marienplatz area)
2. **zone-olympiapark** — Olympiapark area
3. **zone-sendlinger-tor** — Sendlinger Tor area

Each zone uses `per_minute` rate at €0.05/min, currency EUR. All zones
reference the same adapter image (`localhost/parking-operator-adaptor:latest`)
for the demo.

## Dependencies

| Spec | Relationship | Notes |
|------|-------------|-------|
| 01_repo_setup | Depends on | Go module, Makefile, skeleton service at `backend/parking-fee-service/` |
| 04_qm_partition | Integrates with | UPDATE_SERVICE (adapter install), mock PARKING_APP CLI (extend) |

## Clarifications

### Architecture

- **A1 (Session endpoints):** No. Session management is entirely on the
  PARKING_OPERATOR_ADAPTOR (spec 04). PARKING_FEE_SERVICE handles only
  zone/operator lookup and adapter metadata.

- **A2 (Registry gatekeeper):** Metadata only. PARKING_FEE_SERVICE returns
  adapter metadata (image_ref, checksum). The vehicle's UPDATE_SERVICE pulls
  from the registry directly (or uses pre-built local images for the demo).

- **A3 (PARKING_APP scope):** Backend only. This spec focuses on the
  PARKING_FEE_SERVICE. The Android PARKING_APP is a separate spec. The mock
  parking-app-cli is extended to test the PARKING_FEE_SERVICE.

### Implementation

- **U1 (Zone data model):** 1:1 zone-to-operator. Operator info embedded in
  zone. No separate operator entity needed for the demo.

- **U2 (Location lookup):** 200m fuzzy radius. Point-in-polygon for exact
  match, Haversine distance for fuzzy match. Return all matches.

- **U3 (Demo zones):** 3 zones in Munich with realistic geofence polygons.

- **U4 (Authentication):** No authentication for the demo.

- **U5 (Persistence):** In-memory with hardcoded seed data. No database.
