// Package zones provides the in-memory zone data store for PARKING_FEE_SERVICE.
//
// It defines the Zone data model, an in-memory Store keyed by zone_id, and
// methods for zone lookup by ID and by geographic location (using the geo
// package for point-in-polygon and fuzzy distance matching).
package zones

import (
	"sort"

	"github.com/rhadp/parking-fee-service/backend/parking-fee-service/geo"
)

// fuzzyRadiusMeters is the maximum distance in meters for fuzzy matching
// when a point is outside all zone polygons.
const fuzzyRadiusMeters = 200.0

// Zone represents a parking zone with its geofence polygon, operator info,
// rate configuration, and adapter metadata.
type Zone struct {
	ZoneID          string       `json:"zone_id"`
	Name            string       `json:"name"`
	OperatorName    string       `json:"operator_name"`
	Polygon         []geo.LatLon `json:"polygon"`
	AdapterImageRef string       `json:"adapter_image_ref"`
	AdapterChecksum string       `json:"adapter_checksum"`
	RateType        string       `json:"rate_type"`   // "per_minute" or "flat"
	RateAmount      float64      `json:"rate_amount"`
	Currency        string       `json:"currency"`
}

// ZoneMatch represents a zone returned from a location lookup, including
// the distance from the query point to the zone polygon.
type ZoneMatch struct {
	ZoneID         string  `json:"zone_id"`
	Name           string  `json:"name"`
	OperatorName   string  `json:"operator_name"`
	RateType       string  `json:"rate_type"`
	RateAmount     float64 `json:"rate_amount"`
	Currency       string  `json:"currency"`
	DistanceMeters float64 `json:"distance_meters"`
}

// AdapterMetadata represents the container image reference and checksum
// needed to install a parking operator adapter.
type AdapterMetadata struct {
	ZoneID   string `json:"zone_id"`
	ImageRef string `json:"image_ref"`
	Checksum string `json:"checksum"`
}

// Store is an in-memory zone data store keyed by zone_id.
type Store struct {
	zones map[string]*Zone
}

// NewStore creates a new empty Store.
func NewStore() *Store {
	return &Store{
		zones: make(map[string]*Zone),
	}
}

// Add inserts a zone into the store. If a zone with the same ID already
// exists, it is overwritten.
func (s *Store) Add(z *Zone) {
	s.zones[z.ZoneID] = z
}

// GetByID returns the zone with the given ID. The second return value
// indicates whether the zone was found.
func (s *Store) GetByID(zoneID string) (*Zone, bool) {
	z, ok := s.zones[zoneID]
	return z, ok
}

// FindByLocation returns zones matching or near the given geographic point.
//
// Step 1: perform a point-in-polygon test for each zone. Any zone whose
// polygon contains the point is included with distance_meters = 0.
//
// Step 2: if no exact (inside-polygon) matches are found, find zones whose
// nearest polygon edge is within 200 meters using Haversine distance.
//
// Results are sorted by distance_meters ascending (nearest first).
func (s *Store) FindByLocation(lat, lon float64) []ZoneMatch {
	var exactMatches []ZoneMatch
	var fuzzyMatches []ZoneMatch

	for _, z := range s.zones {
		if geo.PointInPolygon(lat, lon, z.Polygon) {
			exactMatches = append(exactMatches, ZoneMatch{
				ZoneID:         z.ZoneID,
				Name:           z.Name,
				OperatorName:   z.OperatorName,
				RateType:       z.RateType,
				RateAmount:     z.RateAmount,
				Currency:       z.Currency,
				DistanceMeters: 0,
			})
		} else {
			dist := geo.DistanceToPolygon(lat, lon, z.Polygon)
			if dist <= fuzzyRadiusMeters {
				fuzzyMatches = append(fuzzyMatches, ZoneMatch{
					ZoneID:         z.ZoneID,
					Name:           z.Name,
					OperatorName:   z.OperatorName,
					RateType:       z.RateType,
					RateAmount:     z.RateAmount,
					Currency:       z.Currency,
					DistanceMeters: dist,
				})
			}
		}
	}

	// If there are exact matches, return them (all have distance 0).
	if len(exactMatches) > 0 {
		sort.Slice(exactMatches, func(i, j int) bool {
			return exactMatches[i].ZoneID < exactMatches[j].ZoneID
		})
		return exactMatches
	}

	// Otherwise return fuzzy matches sorted by distance ascending.
	sort.Slice(fuzzyMatches, func(i, j int) bool {
		if fuzzyMatches[i].DistanceMeters != fuzzyMatches[j].DistanceMeters {
			return fuzzyMatches[i].DistanceMeters < fuzzyMatches[j].DistanceMeters
		}
		return fuzzyMatches[i].ZoneID < fuzzyMatches[j].ZoneID
	})

	return fuzzyMatches
}
