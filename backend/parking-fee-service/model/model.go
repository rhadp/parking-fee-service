// Package model defines core data types for the parking fee service.
package model

// Coordinate represents a geographic point.
type Coordinate struct {
	Lat float64 `json:"lat"`
	Lon float64 `json:"lon"`
}

// Zone represents a named geographic area defined by a geofence polygon.
type Zone struct {
	ID      string       `json:"id"`
	Name    string       `json:"name"`
	Polygon []Coordinate `json:"polygon"`
}

// Rate represents pricing information for a parking operator.
type Rate struct {
	Type     string  `json:"type"`
	Amount   float64 `json:"amount"`
	Currency string  `json:"currency"`
}

// AdapterMeta contains metadata needed to download a parking operator adapter.
type AdapterMeta struct {
	ImageRef       string `json:"image_ref"`
	ChecksumSHA256 string `json:"checksum_sha256"`
	Version        string `json:"version"`
}

// Operator represents a parking service provider.
type Operator struct {
	ID      string      `json:"id"`
	Name    string      `json:"name"`
	ZoneID  string      `json:"zone_id"`
	Rate    Rate        `json:"rate"`
	Adapter AdapterMeta `json:"adapter"`
}

// OperatorResponse is the API response type for operator lookup (excludes adapter).
type OperatorResponse struct {
	ID     string `json:"id"`
	Name   string `json:"name"`
	ZoneID string `json:"zone_id"`
	Rate   Rate   `json:"rate"`
}

// Config represents the service configuration.
type Config struct {
	Port               int        `json:"port"`
	ProximityThreshold float64    `json:"proximity_threshold_meters"`
	Zones              []Zone     `json:"zones"`
	Operators          []Operator `json:"operators"`
}
