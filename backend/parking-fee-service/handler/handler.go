// Package handler implements HTTP request handlers for the parking-fee-service.
package handler

import (
	"net/http"

	"parking-fee-service/backend/parking-fee-service/model"
	"parking-fee-service/backend/parking-fee-service/store"
)

// NewOperatorHandler returns an HTTP handler for GET /operators.
// It parses lat/lon query params, finds matching zones, and returns operators.
func NewOperatorHandler(s *store.Store, zones []model.Zone, threshold float64) http.HandlerFunc {
	panic("not implemented")
}

// NewAdapterHandler returns an HTTP handler for GET /operators/{id}/adapter.
func NewAdapterHandler(s *store.Store) http.HandlerFunc {
	panic("not implemented")
}

// HealthHandler returns an HTTP handler for GET /health.
func HealthHandler() http.HandlerFunc {
	panic("not implemented")
}
