package main

// Store holds in-memory data for zones, operators, and adapter metadata.
type Store struct {
	zones                    []Zone
	operators                []OperatorConfig
	adaptersByOperatorID     map[string]*AdapterMetadata
	proximityThresholdMeters float64
}

// NewStore creates a new Store from the given configuration.
func NewStore(cfg *Config) *Store {
	s := &Store{
		zones:                    cfg.Zones,
		operators:                cfg.Operators,
		adaptersByOperatorID:     make(map[string]*AdapterMetadata),
		proximityThresholdMeters: cfg.Settings.ProximityThresholdMeters,
	}
	if s.proximityThresholdMeters == 0 {
		s.proximityThresholdMeters = DefaultProximityThresholdMeters
	}
	for i := range cfg.Operators {
		adapter := cfg.Operators[i].Adapter
		s.adaptersByOperatorID[cfg.Operators[i].ID] = &adapter
	}
	return s
}

// FindOperatorsByLocation returns all operators whose zone contains or is near the given coordinates.
func (s *Store) FindOperatorsByLocation(lat, lon float64) []Operator {
	point := LatLon{Lat: lat, Lon: lon}
	var results []Operator

	// Build a map of matching zone IDs
	matchingZones := make(map[string]bool)
	for _, zone := range s.zones {
		if PointInOrNearPolygon(point, zone.Polygon, s.proximityThresholdMeters) {
			matchingZones[zone.ID] = true
		}
	}

	// Return operators whose zone matches
	for _, opCfg := range s.operators {
		if matchingZones[opCfg.ZoneID] {
			results = append(results, opCfg.Operator)
		}
	}

	return results
}

// GetAdapterMetadata returns the adapter metadata for the given operator ID.
func (s *Store) GetAdapterMetadata(operatorID string) (*AdapterMetadata, bool) {
	meta, ok := s.adaptersByOperatorID[operatorID]
	return meta, ok
}
