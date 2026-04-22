package handler

import (
	"net/http"

	"github.com/rhadp/parking-fee-service/backend/parking-fee-service/model"
	"github.com/rhadp/parking-fee-service/backend/parking-fee-service/store"
)

// NewOperatorHandler returns an HTTP handler for operator lookup by location.
func NewOperatorHandler(_ *store.Store, _ []model.Zone, _ float64) http.HandlerFunc {
	return func(w http.ResponseWriter, r *http.Request) {
		http.Error(w, "not implemented", http.StatusInternalServerError)
	}
}

// NewAdapterHandler returns an HTTP handler for adapter metadata retrieval.
func NewAdapterHandler(_ *store.Store) http.HandlerFunc {
	return func(w http.ResponseWriter, r *http.Request) {
		http.Error(w, "not implemented", http.StatusInternalServerError)
	}
}

// HealthHandler returns an HTTP handler for the health check endpoint.
func HealthHandler() http.HandlerFunc {
	return func(w http.ResponseWriter, r *http.Request) {
		http.Error(w, "not implemented", http.StatusInternalServerError)
	}
}
