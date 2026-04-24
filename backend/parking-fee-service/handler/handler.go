// Package handler provides HTTP request handlers for the parking-fee-service
// REST API endpoints.
package handler

import (
	"net/http"

	"github.com/rhadp/parking-fee-service/backend/parking-fee-service/model"
	"github.com/rhadp/parking-fee-service/backend/parking-fee-service/store"
)

// NewOperatorHandler returns an http.HandlerFunc that handles operator
// lookup by location (GET /operators?lat=&lon=).
func NewOperatorHandler(s *store.Store, zones []model.Zone, threshold float64) http.HandlerFunc {
	return func(w http.ResponseWriter, r *http.Request) {}
}

// NewAdapterHandler returns an http.HandlerFunc that handles adapter
// metadata retrieval (GET /operators/{id}/adapter).
func NewAdapterHandler(s *store.Store) http.HandlerFunc {
	return func(w http.ResponseWriter, r *http.Request) {}
}

// HealthHandler returns an http.HandlerFunc that handles health checks
// (GET /health).
func HealthHandler() http.HandlerFunc {
	return func(w http.ResponseWriter, r *http.Request) {}
}
