// Package store provides an in-memory data store for zones and operators.
package store

import (
	"parking-fee-service/backend/parking-fee-service/model"
)

// Store holds zones and operators indexed for fast lookup.
type Store struct{}

// NewStore creates a new Store from the given zones and operators.
func NewStore(zones []model.Zone, operators []model.Operator) *Store {
	return &Store{}
}

// GetZone returns the zone with the given ID.
func (s *Store) GetZone(id string) (*model.Zone, bool) {
	return nil, false
}

// GetOperator returns the operator with the given ID.
func (s *Store) GetOperator(id string) (*model.Operator, bool) {
	return nil, false
}

// GetOperatorsByZoneIDs returns all operators whose zone_id is in the given set.
func (s *Store) GetOperatorsByZoneIDs(zoneIDs []string) []model.Operator {
	return nil
}
