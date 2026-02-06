// Package service provides business logic for the parking-fee-service.
package service

import (
	"github.com/sdv-parking-demo/backend/parking-fee-service/internal/model"
	"github.com/sdv-parking-demo/backend/parking-fee-service/internal/store"
)

// ZoneService provides business logic for zone operations.
type ZoneService struct {
	store *store.ZoneStore
}

// NewZoneService creates a new ZoneService with the given store.
func NewZoneService(store *store.ZoneStore) *ZoneService {
	return &ZoneService{
		store: store,
	}
}

// FindZoneByLocation finds the zone containing the given coordinates.
// Returns nil if no zone contains the location.
func (s *ZoneService) FindZoneByLocation(lat, lng float64) *model.Zone {
	return s.store.FindByLocation(lat, lng)
}
