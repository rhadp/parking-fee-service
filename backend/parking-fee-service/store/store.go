// Package store provides an in-memory data store for zones and operators.
package store

import (
	"parking-fee-service/backend/parking-fee-service/model"
)

// Store holds zones and operators indexed for fast lookup.
type Store struct {
	zones      map[string]model.Zone
	operators  map[string]model.Operator
	byZoneID   map[string][]model.Operator
}

// NewStore creates a new Store from the given zones and operators.
func NewStore(zones []model.Zone, operators []model.Operator) *Store {
	s := &Store{
		zones:    make(map[string]model.Zone, len(zones)),
		operators: make(map[string]model.Operator, len(operators)),
		byZoneID:  make(map[string][]model.Operator),
	}
	for _, z := range zones {
		s.zones[z.ID] = z
	}
	for _, op := range operators {
		s.operators[op.ID] = op
		s.byZoneID[op.ZoneID] = append(s.byZoneID[op.ZoneID], op)
	}
	return s
}

// GetZone returns the zone with the given ID.
func (s *Store) GetZone(id string) (*model.Zone, bool) {
	z, ok := s.zones[id]
	if !ok {
		return nil, false
	}
	return &z, true
}

// GetOperator returns the operator with the given ID.
func (s *Store) GetOperator(id string) (*model.Operator, bool) {
	op, ok := s.operators[id]
	if !ok {
		return nil, false
	}
	return &op, true
}

// GetOperatorsByZoneIDs returns all operators whose zone_id is in the given set.
func (s *Store) GetOperatorsByZoneIDs(zoneIDs []string) []model.Operator {
	var result []model.Operator
	for _, zid := range zoneIDs {
		if ops, ok := s.byZoneID[zid]; ok {
			result = append(result, ops...)
		}
	}
	return result
}
