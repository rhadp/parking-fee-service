// Package store provides an in-memory data store for zones and operators.
package store

import (
	"github.com/sdv-demo/parking-fee-service/backend/parking-fee-service/model"
)

// Store is an in-memory index of zones and operators for fast lookup.
type Store struct {
	zones     map[string]*model.Zone
	operators map[string]*model.Operator
	byZone    map[string][]*model.Operator
}

// NewStore creates a Store indexed from the given zones and operators slices.
// Zones are indexed by ID; operators are indexed both by ID and by zone_id.
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

// GetZone returns the zone with the given ID, or (nil, false) if not found.
func (s *Store) GetZone(id string) (*model.Zone, bool) {
	z, ok := s.zones[id]
	return z, ok
}

// GetOperator returns the operator with the given ID, or (nil, false) if not found.
func (s *Store) GetOperator(id string) (*model.Operator, bool) {
	op, ok := s.operators[id]
	return op, ok
}

// GetOperatorsByZoneIDs returns all operators whose zone_id is in zoneIDs.
// The order is deterministic: zone IDs are iterated in the order provided,
// and within each zone operators are returned in insertion order.
func (s *Store) GetOperatorsByZoneIDs(zoneIDs []string) []model.Operator {
	var result []model.Operator
	for _, id := range zoneIDs {
		for _, op := range s.byZone[id] {
			result = append(result, *op)
		}
	}
	return result
}
