package main

// Store holds in-memory data for zones, operators, and adapter metadata.
type Store struct {
	zones                    []Zone
	operators                []OperatorConfig
	adaptersByOperator       map[string]AdapterMetadata
	proximityThresholdMeters float64
}

// NewStore creates a new Store from the given configuration.
func NewStore(cfg *Config) *Store {
	s := &Store{
		zones:                    cfg.Zones,
		operators:                cfg.Operators,
		adaptersByOperator:       make(map[string]AdapterMetadata),
		proximityThresholdMeters: cfg.Settings.ProximityThresholdMeters,
	}
	if s.proximityThresholdMeters == 0 {
		s.proximityThresholdMeters = DefaultProximityThresholdMeters
	}
	for _, op := range cfg.Operators {
		s.adaptersByOperator[op.ID] = op.Adapter
	}
	return s
}

// FindOperatorsByLocation returns operators whose zone contains or is near the given coordinates.
func (s *Store) FindOperatorsByLocation(lat, lon float64) []Operator {
	point := LatLon{Lat: lat, Lon: lon}

	// Build a set of matching zone IDs
	matchingZones := make(map[string]bool)
	for _, zone := range s.zones {
		if PointInOrNearPolygon(point, zone.Polygon, s.proximityThresholdMeters) {
			matchingZones[zone.ID] = true
		}
	}

	// Collect operators whose zone matched
	var result []Operator
	for _, opCfg := range s.operators {
		if matchingZones[opCfg.ZoneID] {
			result = append(result, opCfg.Operator)
		}
	}
	return result
}

// GetAdapterMetadata returns the adapter metadata for the given operator ID.
func (s *Store) GetAdapterMetadata(operatorID string) (*AdapterMetadata, bool) {
	meta, ok := s.adaptersByOperator[operatorID]
	if !ok {
		return nil, false
	}
	return &meta, true
}
