// Package store provides data storage implementations for the parking-fee-service.
package store

import (
	"sync"

	"github.com/sdv-parking-demo/backend/parking-fee-service/internal/model"
)

// ZoneStore provides in-memory storage for parking zones.
type ZoneStore struct {
	zones []model.Zone
	mu    sync.RWMutex
}

// NewZoneStore creates a new ZoneStore with the given zones.
func NewZoneStore(zones []model.Zone) *ZoneStore {
	return &ZoneStore{
		zones: zones,
	}
}

// FindByLocation returns the zone containing the given coordinates.
// Returns nil if no zone contains the location.
func (s *ZoneStore) FindByLocation(lat, lng float64) *model.Zone {
	s.mu.RLock()
	defer s.mu.RUnlock()

	for i := range s.zones {
		if s.zones[i].Bounds.ContainsPoint(lat, lng) {
			zone := s.zones[i]
			return &zone
		}
	}
	return nil
}

// List returns all zones in the store.
func (s *ZoneStore) List() []model.Zone {
	s.mu.RLock()
	defer s.mu.RUnlock()

	result := make([]model.Zone, len(s.zones))
	copy(result, s.zones)
	return result
}
