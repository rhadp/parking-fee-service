package main

// LatLon represents a geographic coordinate.
type LatLon struct {
	Lat float64 `json:"lat"`
	Lon float64 `json:"lon"`
}

// RateType represents the billing model for a parking operator.
type RateType string

const (
	RatePerHour RateType = "per_hour"
	RateFlatFee RateType = "flat_fee"
)

// Zone represents a geographic area defined by a geofence polygon.
type Zone struct {
	ID      string   `json:"id"`
	Name    string   `json:"name"`
	Polygon []LatLon `json:"polygon"`
}

// Operator represents a parking service provider.
type Operator struct {
	ID           string   `json:"operator_id"`
	Name         string   `json:"name"`
	ZoneID       string   `json:"zone_id"`
	RateType     RateType `json:"rate_type"`
	RateAmount   float64  `json:"rate_amount"`
	RateCurrency string   `json:"rate_currency"`
}

// AdapterMetadata contains the OCI image reference and verification data.
type AdapterMetadata struct {
	ImageRef       string `json:"image_ref"`
	ChecksumSHA256 string `json:"checksum_sha256"`
	Version        string `json:"version"`
}

// ErrorResponse is the standard JSON error response body.
type ErrorResponse struct {
	Error string `json:"error"`
}
