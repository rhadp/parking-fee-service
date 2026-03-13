package main

// Rate represents the parking rate.
type Rate struct {
	RateType string  `json:"rate_type"` // "per_hour"
	Amount   float64 `json:"amount"`    // 2.50
	Currency string  `json:"currency"`  // "EUR"
}

// DefaultRate is the fixed rate used by the mock parking operator.
var DefaultRate = Rate{
	RateType: "per_hour",
	Amount:   2.50,
	Currency: "EUR",
}

// Session represents a parking session.
type Session struct {
	SessionID string `json:"session_id"`
	VehicleID string `json:"vehicle_id"`
	ZoneID    string `json:"zone_id"`
	Status    string `json:"status"`     // "active" | "stopped"
	StartTime int64  `json:"start_time"` // unix timestamp
	StopTime  int64  `json:"stop_time,omitempty"`
	Rate      Rate   `json:"rate"`
}

// StartRequest represents the JSON body for POST /parking/start.
type StartRequest struct {
	VehicleID string `json:"vehicle_id"`
	ZoneID    string `json:"zone_id"`
	Timestamp int64  `json:"timestamp"`
}

// StartResponse represents the JSON response for POST /parking/start.
type StartResponse struct {
	SessionID string `json:"session_id"`
	Status    string `json:"status"`
	Rate      Rate   `json:"rate"`
}

// StopRequest represents the JSON body for POST /parking/stop.
type StopRequest struct {
	SessionID string `json:"session_id"`
	Timestamp int64  `json:"timestamp"`
}

// StopResponse represents the JSON response for POST /parking/stop.
type StopResponse struct {
	SessionID       string  `json:"session_id"`
	Status          string  `json:"status"`
	DurationSeconds int64   `json:"duration_seconds"`
	TotalAmount     float64 `json:"total_amount"`
	Currency        string  `json:"currency"`
}

// ErrorResponse represents a JSON error response.
type ErrorResponse struct {
	Error string `json:"error"`
}
