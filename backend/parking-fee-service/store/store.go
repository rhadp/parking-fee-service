// Package store provides an in-memory data store for zones and operators.
package store

import (
	"github.com/rhadp/parking-fee-service/backend/parking-fee-service/model"
)

// Store holds zones and operators indexed for fast lookup.
// STUB: indexes are not yet populated.
type Store struct {
	zones     map[string]*model.Zone
	operators map[string]*model.Operator
	byZone    map[string][]*model.Operator
}

// NewStore creates a Store from the provided zones and operators.
// STUB: returns an empty (unindexed) store.
func NewStore(zones []model.Zone, operators []model.Operator) *Store {
	return &Store{
		zones:     make(map[string]*model.Zone),
		operators: make(map[string]*model.Operator),
		byZone:    make(map[string][]*model.Operator),
	}
}

// GetZone retrieves a zone by ID. Returns (nil, false) if not found.
func (s *Store) GetZone(id string) (*model.Zone, bool) {
	z, ok := s.zones[id]
	return z, ok
}

// GetOperator retrieves an operator by ID. Returns (nil, false) if not found.
func (s *Store) GetOperator(id string) (*model.Operator, bool) {
	op, ok := s.operators[id]
	return op, ok
}

// GetOperatorsByZoneIDs returns all operators whose ZoneID is in zoneIDs.
// STUB: always returns nil.
func (s *Store) GetOperatorsByZoneIDs(zoneIDs []string) []model.Operator {
	return nil
}
