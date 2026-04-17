// Package model defines the core data types for the PARKING_FEE_SERVICE.
package model

// Coordinate represents a geographic latitude/longitude pair.
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

// Rate describes the pricing model for a parking operator.
type Rate struct {
	Type     string  `json:"type"`     // "per-hour" | "flat-fee"
	Amount   float64 `json:"amount"`
	Currency string  `json:"currency"` // e.g. "EUR"
}

// AdapterMeta holds the OCI image reference and integrity metadata for an
// operator's PARKING_OPERATOR_ADAPTOR.
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

// OperatorResponse is the wire format returned by the operator lookup endpoint.
// The adapter field is intentionally excluded so that clients must call
// /operators/{id}/adapter separately.
type OperatorResponse struct {
	ID     string `json:"id"`
	Name   string `json:"name"`
	ZoneID string `json:"zone_id"`
	Rate   Rate   `json:"rate"`
}

// Config is the top-level configuration structure loaded from the JSON config
// file (or from built-in defaults when no file is present).
type Config struct {
	Port               int        `json:"port"`
	ProximityThreshold float64    `json:"proximity_threshold_meters"`
	Zones              []Zone     `json:"zones"`
	Operators          []Operator `json:"operators"`
}
