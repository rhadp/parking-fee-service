// Package handler provides HTTP request handlers for the parking-fee-service.
package handler

import (
	"net/http"

	"github.com/rhadp/parking-fee-service/backend/parking-fee-service/model"
	"github.com/rhadp/parking-fee-service/backend/parking-fee-service/store"
)

// NewOperatorHandler returns a handler for GET /operators?lat=&lon=.
// It validates coordinates, finds matching zones, and returns operator JSON.
func NewOperatorHandler(st *store.Store, zones []model.Zone, threshold float64) http.HandlerFunc {
	// stub: not implemented
	return func(w http.ResponseWriter, r *http.Request) {
	}
}

// NewAdapterHandler returns a handler for GET /operators/{id}/adapter.
// It looks up the operator by ID and returns adapter metadata JSON.
func NewAdapterHandler(st *store.Store) http.HandlerFunc {
	// stub: not implemented
	return func(w http.ResponseWriter, r *http.Request) {
	}
}

// HealthHandler returns a handler for GET /health that returns {"status":"ok"}.
func HealthHandler() http.HandlerFunc {
	// stub: not implemented
	return func(w http.ResponseWriter, r *http.Request) {
	}
}
