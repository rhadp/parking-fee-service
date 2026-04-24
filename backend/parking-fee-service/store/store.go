// Package store provides an in-memory data store for zones and operators,
// indexed for fast lookup by zone ID and operator ID.
package store

import (
	"github.com/rhadp/parking-fee-service/backend/parking-fee-service/model"
)

// Store holds zones and operators in memory with index structures.
type Store struct{}

// NewStore creates a Store from the given zones and operators.
func NewStore(zones []model.Zone, operators []model.Operator) *Store {
	return &Store{}
}

// GetZone returns the zone with the given ID, or false if not found.
func (s *Store) GetZone(id string) (*model.Zone, bool) {
	return nil, false
}

// GetOperator returns the operator with the given ID, or false if not found.
func (s *Store) GetOperator(id string) (*model.Operator, bool) {
	return nil, false
}

// GetOperatorsByZoneIDs returns all operators whose zone_id is in the
// given set of zone IDs.
func (s *Store) GetOperatorsByZoneIDs(zoneIDs []string) []model.Operator {
	return nil
}
