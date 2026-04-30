package model

// Coordinate represents a geographic point.
type Coordinate struct {
	Lat float64 `json:"lat"`
	Lon float64 `json:"lon"`
}

// Zone represents a geographic zone defined by a polygon.
type Zone struct {
	ID      string       `json:"id"`
	Name    string       `json:"name"`
	Polygon []Coordinate `json:"polygon"`
}

// Rate represents a parking rate.
type Rate struct {
	Type     string  `json:"type"`
	Amount   float64 `json:"amount"`
	Currency string  `json:"currency"`
}

// AdapterMeta contains adapter download metadata.
type AdapterMeta struct {
	ImageRef       string `json:"image_ref"`
	ChecksumSHA256 string `json:"checksum_sha256"`
	Version        string `json:"version"`
}

// Operator represents a parking operator.
type Operator struct {
	ID      string      `json:"id"`
	Name    string      `json:"name"`
	ZoneID  string      `json:"zone_id"`
	Rate    Rate        `json:"rate"`
	Adapter AdapterMeta `json:"adapter"`
}

// OperatorResponse is the API response for operator lookup (excludes adapter).
type OperatorResponse struct {
	ID     string `json:"id"`
	Name   string `json:"name"`
	ZoneID string `json:"zone_id"`
	Rate   Rate   `json:"rate"`
}

// Config holds the service configuration.
type Config struct {
	Port               int        `json:"port"`
	ProximityThreshold float64    `json:"proximity_threshold_meters"`
	Zones              []Zone     `json:"zones"`
	Operators          []Operator `json:"operators"`
}
