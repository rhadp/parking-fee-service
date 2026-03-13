// Package handler provides HTTP handlers for the parking-fee-service REST API.
package handler

import (
	"net/http"

	"github.com/rhadp/parking-fee-service/backend/parking-fee-service/model"
	"github.com/rhadp/parking-fee-service/backend/parking-fee-service/store"
)

// NewOperatorHandler returns an HTTP handler for GET /operators?lat=&lon=.
// It finds operators whose zones match the given coordinate.
// STUB: returns an empty 200 response.
func NewOperatorHandler(s *store.Store, zones []model.Zone, threshold float64) http.HandlerFunc {
	return func(w http.ResponseWriter, r *http.Request) {
		// not implemented
	}
}

// NewAdapterHandler returns an HTTP handler for GET /operators/{id}/adapter.
// STUB: returns an empty 200 response.
func NewAdapterHandler(s *store.Store) http.HandlerFunc {
	return func(w http.ResponseWriter, r *http.Request) {
		// not implemented
	}
}

// HealthHandler returns an HTTP handler for GET /health.
// STUB: returns an empty 200 response.
func HealthHandler() http.HandlerFunc {
	return func(w http.ResponseWriter, r *http.Request) {
		// not implemented
	}
}
