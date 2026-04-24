// Package model defines the core data types for the parking-fee-service.
package model

// Coordinate represents a geographic point with latitude and longitude.
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

// Rate represents a parking fee structure.
type Rate struct {
	Type     string  `json:"type"`     // "per-hour" | "flat-fee"
	Amount   float64 `json:"amount"`
	Currency string  `json:"currency"` // e.g. "EUR"
}

// AdapterMeta holds information needed to download a parking operator adapter.
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

// OperatorResponse is the public API representation of an operator.
// It excludes the Adapter field to prevent leaking adapter metadata
// in operator lookup responses.
type OperatorResponse struct {
	ID     string `json:"id"`
	Name   string `json:"name"`
	ZoneID string `json:"zone_id"`
	Rate   Rate   `json:"rate"`
}

// Config holds the service configuration loaded from a JSON file.
type Config struct {
	Port               int        `json:"port"`
	ProximityThreshold float64    `json:"proximity_threshold_meters"`
	Zones              []Zone     `json:"zones"`
	Operators          []Operator `json:"operators"`
}
