// Package store provides an in-memory data store for zones and operators.
package store

import (
	"github.com/rhadp/parking-fee-service/backend/parking-fee-service/model"
)

// Store holds zones and operators indexed for fast lookup.
type Store struct {
	zones     map[string]*model.Zone
	operators map[string]*model.Operator
	byZone    map[string][]*model.Operator
}

// NewStore creates a Store from the provided zones and operators, building
// indexes for fast lookup by zone ID and operator ID.
func NewStore(zones []model.Zone, operators []model.Operator) *Store {
	s := &Store{
		zones:     make(map[string]*model.Zone, len(zones)),
		operators: make(map[string]*model.Operator, len(operators)),
		byZone:    make(map[string][]*model.Operator),
	}

	for i := range zones {
		z := &zones[i]
		s.zones[z.ID] = z
	}

	for i := range operators {
		op := &operators[i]
		s.operators[op.ID] = op
		s.byZone[op.ZoneID] = append(s.byZone[op.ZoneID], op)
	}

	return s
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
// The returned slice is never nil (empty input yields an empty slice).
func (s *Store) GetOperatorsByZoneIDs(zoneIDs []string) []model.Operator {
	result := []model.Operator{}
	for _, zid := range zoneIDs {
		for _, op := range s.byZone[zid] {
			result = append(result, *op)
		}
	}
	return result
}
