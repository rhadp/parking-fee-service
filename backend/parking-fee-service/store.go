package main

// Store holds in-memory data for zones, operators, and adapter metadata.
type Store struct {
	zones                    []Zone
	operators                []OperatorConfig
	proximityThresholdMeters float64
}

// NewStore creates a new Store from the given configuration.
func NewStore(cfg *Config) *Store {
	// TODO: implement
	return nil
}

// FindOperatorsByLocation returns all operators whose zone contains or is near the given coordinates.
func (s *Store) FindOperatorsByLocation(lat, lon float64) []Operator {
	// TODO: implement
	return nil
}

// GetAdapterMetadata returns the adapter metadata for the given operator ID.
func (s *Store) GetAdapterMetadata(operatorID string) (*AdapterMetadata, bool) {
	// TODO: implement
	return nil, false
}
