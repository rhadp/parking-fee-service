package store

import "github.com/rhadp/parking-fee-service/backend/parking-fee-service/model"

// Store is an in-memory data store for zones and operators.
type Store struct{}

// NewStore creates a new Store indexed by zone and operator IDs.
func NewStore(_ []model.Zone, _ []model.Operator) *Store {
	return &Store{}
}

// GetZone returns the zone with the given ID.
func (s *Store) GetZone(_ string) (*model.Zone, bool) {
	return nil, false
}

// GetOperator returns the operator with the given ID.
func (s *Store) GetOperator(_ string) (*model.Operator, bool) {
	return nil, false
}

// GetOperatorsByZoneIDs returns all operators whose zone_id matches any of the given IDs.
func (s *Store) GetOperatorsByZoneIDs(_ []string) []model.Operator {
	return nil
}
