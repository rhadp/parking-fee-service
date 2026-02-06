// Package middleware provides HTTP middleware and utilities for the parking-fee-service.
package middleware

import (
	"fmt"

	"github.com/sdv-parking-demo/backend/parking-fee-service/internal/model"
)

// ValidateCoordinates validates that latitude and longitude are within valid ranges.
// Latitude must be between -90 and 90.
// Longitude must be between -180 and 180.
func ValidateCoordinates(lat, lng float64) error {
	if lat < -90 || lat > 90 {
		return fmt.Errorf("latitude must be between -90 and 90, got %f", lat)
	}
	if lng < -180 || lng > 180 {
		return fmt.Errorf("longitude must be between -180 and 180, got %f", lng)
	}
	return nil
}

// ValidateStartSessionRequest validates a start session request.
// Returns an error if any required field is missing or invalid.
func ValidateStartSessionRequest(req *model.StartSessionRequest) error {
	if req.VehicleID == "" {
		return fmt.Errorf("vehicle_id is required")
	}
	if req.ZoneID == "" {
		return fmt.Errorf("zone_id is required")
	}
	if req.Timestamp == "" {
		return fmt.Errorf("timestamp is required")
	}
	return ValidateCoordinates(req.Latitude, req.Longitude)
}

// ValidateStopSessionRequest validates a stop session request.
// Returns an error if any required field is missing.
func ValidateStopSessionRequest(req *model.StopSessionRequest) error {
	if req.SessionID == "" {
		return fmt.Errorf("session_id is required")
	}
	if req.Timestamp == "" {
		return fmt.Errorf("timestamp is required")
	}
	return nil
}
