package store

import "github.com/rhadp/parking-fee-service/backend/parking-fee-service/model"

// Store is an in-memory data store for zones and operators.
type Store struct {
	zones          map[string]model.Zone
	operators      map[string]model.Operator
	opsByZoneID    map[string][]model.Operator
}

// NewStore creates a new Store indexed by zone and operator IDs.
func NewStore(zones []model.Zone, operators []model.Operator) *Store {
	s := &Store{
		zones:       make(map[string]model.Zone, len(zones)),
		operators:   make(map[string]model.Operator, len(operators)),
		opsByZoneID: make(map[string][]model.Operator),
	}

	for _, z := range zones {
		s.zones[z.ID] = z
	}

	for _, op := range operators {
		s.operators[op.ID] = op
		s.opsByZoneID[op.ZoneID] = append(s.opsByZoneID[op.ZoneID], op)
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

// GetOperatorsByZoneIDs returns all operators whose zone_id matches any of the given IDs.
func (s *Store) GetOperatorsByZoneIDs(zoneIDs []string) []model.Operator {
	var result []model.Operator
	for _, zid := range zoneIDs {
		if ops, ok := s.opsByZoneID[zid]; ok {
			result = append(result, ops...)
		}
	}
	return result
}

// AllZones returns all zones in the store.
func (s *Store) AllZones() []model.Zone {
	zones := make([]model.Zone, 0, len(s.zones))
	for _, z := range s.zones {
		zones = append(zones, z)
	}
	return zones
}
