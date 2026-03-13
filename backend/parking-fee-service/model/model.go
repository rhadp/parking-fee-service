// Package model defines the core data types for the parking-fee-service.
package model

// Coordinate represents a geographic point.
type Coordinate struct {
	Lat float64 `json:"lat"`
	Lon float64 `json:"lon"`
}

// Zone is a named geographic area defined by a geofence polygon.
type Zone struct {
	ID      string       `json:"id"`
	Name    string       `json:"name"`
	Polygon []Coordinate `json:"polygon"`
}

// Rate describes the pricing structure for a parking operator.
type Rate struct {
	Type     string  `json:"type"`     // "per-hour" | "flat-fee"
	Amount   float64 `json:"amount"`
	Currency string  `json:"currency"` // e.g. "EUR"
}

// AdapterMeta holds the information needed to download a PARKING_OPERATOR_ADAPTOR.
type AdapterMeta struct {
	ImageRef       string `json:"image_ref"`
	ChecksumSHA256 string `json:"checksum_sha256"`
	Version        string `json:"version"`
}

// Operator represents a parking service provider associated with a zone.
type Operator struct {
	ID      string      `json:"id"`
	Name    string      `json:"name"`
	ZoneID  string      `json:"zone_id"`
	Rate    Rate        `json:"rate"`
	Adapter AdapterMeta `json:"adapter"`
}

// Config is the top-level service configuration loaded from a JSON file.
type Config struct {
	Port               int        `json:"port"`
	ProximityThreshold float64    `json:"proximity_threshold_meters"`
	Zones              []Zone     `json:"zones"`
	Operators          []Operator `json:"operators"`
}

// OperatorResponse is the operator representation returned by the lookup endpoint.
// It intentionally omits the Adapter field (clients must call /operators/{id}/adapter).
type OperatorResponse struct {
	ID     string `json:"id"`
	Name   string `json:"name"`
	ZoneID string `json:"zone_id"`
	Rate   Rate   `json:"rate"`
}
