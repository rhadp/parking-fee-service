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
	s := &Store{
		zones:     make(map[string]*model.Zone, len(zones)),
		operators: make(map[string]*model.Operator, len(operators)),
		byZone:    make(map[string][]*model.Operator),
	}

	for i := range zones {
		z := zones[i] // take address of loop var copy
		s.zones[z.ID] = &z
	}

	for i := range operators {
		op := operators[i] // take address of loop var copy
		s.operators[op.ID] = &op
		s.byZone[op.ZoneID] = append(s.byZone[op.ZoneID], &op)
	}

	return s
}

// GetZone retrieves a zone by ID.
func (s *Store) GetZone(id string) (*model.Zone, bool) {
	z, ok := s.zones[id]
	return z, ok
}

// GetOperator retrieves an operator by ID.
func (s *Store) GetOperator(id string) (*model.Operator, bool) {
	op, ok := s.operators[id]
	return op, ok
}

// GetOperatorsByZoneIDs returns all operators whose zone_id is in zoneIDs.
// The result is deterministic in insertion order per zone.
func (s *Store) GetOperatorsByZoneIDs(zoneIDs []string) []model.Operator {
	result := make([]model.Operator, 0)
	for _, zid := range zoneIDs {
		for _, op := range s.byZone[zid] {
			result = append(result, *op)
		}
	}
	return result
}
