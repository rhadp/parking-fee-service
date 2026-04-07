// Package handler provides HTTP request handlers for the parking fee service.
package handler

import (
	"net/http"

	"parking-fee-service/backend/parking-fee-service/model"
	"parking-fee-service/backend/parking-fee-service/store"
)

// NewOperatorHandler returns an http.HandlerFunc that handles operator lookup by location.
func NewOperatorHandler(s *store.Store, zones []model.Zone, threshold float64) http.HandlerFunc {
	return func(w http.ResponseWriter, r *http.Request) {
		w.WriteHeader(http.StatusNotImplemented)
	}
}

// NewAdapterHandler returns an http.HandlerFunc that handles adapter metadata retrieval.
func NewAdapterHandler(s *store.Store) http.HandlerFunc {
	return func(w http.ResponseWriter, r *http.Request) {
		w.WriteHeader(http.StatusNotImplemented)
	}
}

// HealthHandler returns an http.HandlerFunc that handles health checks.
func HealthHandler() http.HandlerFunc {
	return func(w http.ResponseWriter, r *http.Request) {
		w.WriteHeader(http.StatusNotImplemented)
	}
}
