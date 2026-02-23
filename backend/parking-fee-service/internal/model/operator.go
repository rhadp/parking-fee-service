// Package model defines the data types for the parking fee service.
package model

// Point represents a GPS coordinate.
type Point struct {
	Lat float64 `json:"lat"`
	Lon float64 `json:"lon"`
}

// Rate represents a parking rate.
type Rate struct {
	AmountPerHour float64 `json:"amount_per_hour"`
	Currency      string  `json:"currency"`
}

// Adapter holds adapter provisioning metadata.
type Adapter struct {
	ImageRef       string `json:"image_ref"`
	ChecksumSHA256 string `json:"checksum_sha256"`
	Version        string `json:"version"`
}

// Zone defines a geofenced parking zone.
type Zone struct {
	ID      string  `json:"zone_id"`
	Name    string  `json:"name"`
	Polygon []Point `json:"polygon"`
}

// Operator represents a parking operator with zone, rate, and adapter info.
type Operator struct {
	ID      string  `json:"operator_id"`
	Name    string  `json:"name"`
	Zone    Zone    `json:"zone"`
	Rate    Rate    `json:"rate"`
	Adapter Adapter `json:"adapter"`
}

// OperatorsConfig is the top-level JSON configuration structure.
type OperatorsConfig struct {
	Operators []Operator `json:"operators"`
}
