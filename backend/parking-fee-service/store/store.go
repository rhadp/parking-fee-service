// Package store provides an in-memory data store for zones and operators.
package store

import "github.com/rhadp/parking-fee-service/backend/parking-fee-service/model"

// Store holds indexed zones and operators for fast lookup.
type Store struct {
	// zones indexed by zone ID.
	zones map[string]*model.Zone
	// operators indexed by operator ID.
	operators map[string]*model.Operator
	// operatorsByZone maps zone ID -> list of operators in that zone,
	// in the order they were passed to NewStore.
	operatorsByZone map[string][]*model.Operator
	// zoneOrder preserves zone ID order for deterministic GetOperatorsByZoneIDs results.
	zoneOrder []string
}

// NewStore creates a new Store indexed by zone ID and operator ID.
func NewStore(zones []model.Zone, operators []model.Operator) *Store {
	s := &Store{
		zones:           make(map[string]*model.Zone, len(zones)),
		operators:       make(map[string]*model.Operator, len(operators)),
		operatorsByZone: make(map[string][]*model.Operator),
		zoneOrder:       make([]string, 0, len(zones)),
	}

	for i := range zones {
		z := &zones[i]
		s.zones[z.ID] = z
		s.zoneOrder = append(s.zoneOrder, z.ID)
	}

	for i := range operators {
		op := &operators[i]
		s.operators[op.ID] = op
		s.operatorsByZone[op.ZoneID] = append(s.operatorsByZone[op.ZoneID], op)
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

// GetOperatorsByZoneIDs returns all operators whose zone_id is in the given set.
// Results are in deterministic order: zone ID order (as originally loaded), then
// operator order within each zone.
func (s *Store) GetOperatorsByZoneIDs(zoneIDs []string) []model.Operator {
	if len(zoneIDs) == 0 {
		return []model.Operator{}
	}

	// Build a set of requested zone IDs for O(1) membership tests.
	requested := make(map[string]bool, len(zoneIDs))
	for _, id := range zoneIDs {
		requested[id] = true
	}

	var result []model.Operator
	// Iterate in zone load order for determinism.
	for _, zoneID := range s.zoneOrder {
		if !requested[zoneID] {
			continue
		}
		for _, op := range s.operatorsByZone[zoneID] {
			result = append(result, *op)
		}
	}
	return result
}
