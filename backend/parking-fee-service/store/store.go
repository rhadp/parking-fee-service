// Package store provides an in-memory data store for zones and operators,
// indexed for fast lookup by zone ID and operator ID.
package store

import (
	"github.com/rhadp/parking-fee-service/backend/parking-fee-service/model"
)

// Store holds zones and operators in memory with index structures.
type Store struct {
	zones           map[string]model.Zone
	operators       map[string]model.Operator
	operatorsByZone map[string][]model.Operator
}

// NewStore creates a Store from the given zones and operators.
func NewStore(zones []model.Zone, operators []model.Operator) *Store {
	s := &Store{
		zones:           make(map[string]model.Zone, len(zones)),
		operators:       make(map[string]model.Operator, len(operators)),
		operatorsByZone: make(map[string][]model.Operator),
	}

	for _, z := range zones {
		s.zones[z.ID] = z
	}

	for _, op := range operators {
		s.operators[op.ID] = op
		s.operatorsByZone[op.ZoneID] = append(s.operatorsByZone[op.ZoneID], op)
	}

	return s
}

// GetZone returns the zone with the given ID, or false if not found.
func (s *Store) GetZone(id string) (*model.Zone, bool) {
	z, ok := s.zones[id]
	if !ok {
		return nil, false
	}
	return &z, true
}

// GetOperator returns the operator with the given ID, or false if not found.
func (s *Store) GetOperator(id string) (*model.Operator, bool) {
	op, ok := s.operators[id]
	if !ok {
		return nil, false
	}
	return &op, true
}

// GetOperatorsByZoneIDs returns all operators whose zone_id is in the
// given set of zone IDs.
func (s *Store) GetOperatorsByZoneIDs(zoneIDs []string) []model.Operator {
	var result []model.Operator
	for _, zoneID := range zoneIDs {
		if ops, ok := s.operatorsByZone[zoneID]; ok {
			result = append(result, ops...)
		}
	}
	return result
}
