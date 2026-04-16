// Package store provides an in-memory data store for zones and operators,
// indexed for fast lookup.
package store

import (
	"parking-fee-service/backend/parking-fee-service/model"
)

// Store is the in-memory data store for zones and operators.
type Store struct {
	zones     map[string]*model.Zone
	operators map[string]*model.Operator
	byZone    map[string][]*model.Operator
}

// NewStore creates a Store indexed from the given slices.
func NewStore(zones []model.Zone, operators []model.Operator) *Store {
	panic("not implemented")
}

// GetZone retrieves a zone by ID.
func (s *Store) GetZone(id string) (*model.Zone, bool) {
	panic("not implemented")
}

// GetOperator retrieves an operator by ID.
func (s *Store) GetOperator(id string) (*model.Operator, bool) {
	panic("not implemented")
}

// GetOperatorsByZoneIDs returns all operators whose zone_id is in zoneIDs.
func (s *Store) GetOperatorsByZoneIDs(zoneIDs []string) []model.Operator {
	panic("not implemented")
}
