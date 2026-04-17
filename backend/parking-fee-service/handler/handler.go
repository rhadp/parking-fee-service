// Package handler implements the HTTP handlers for the PARKING_FEE_SERVICE.
package handler

import (
	"net/http"

	"github.com/sdv-demo/parking-fee-service/backend/parking-fee-service/model"
	"github.com/sdv-demo/parking-fee-service/backend/parking-fee-service/store"
)

// NewOperatorHandler returns an http.HandlerFunc for GET /operators?lat=&lon=.
// It parses and validates the lat/lon query parameters, finds matching zones
// via geo.FindMatchingZones, and returns a JSON array of OperatorResponse
// objects (adapter field excluded).
//
// This is a stub — returns 501 Not Implemented. Full implementation is in task group 4.
func NewOperatorHandler(s *store.Store, zones []model.Zone, threshold float64) http.HandlerFunc {
	return func(w http.ResponseWriter, r *http.Request) {
		http.Error(w, `{"error":"not implemented"}`, http.StatusNotImplemented)
	}
}

// NewAdapterHandler returns an http.HandlerFunc for GET /operators/{id}/adapter.
// It extracts the operator ID from the URL path and returns the adapter
// metadata as JSON.
//
// This is a stub — returns 501 Not Implemented. Full implementation is in task group 4.
func NewAdapterHandler(s *store.Store) http.HandlerFunc {
	return func(w http.ResponseWriter, r *http.Request) {
		http.Error(w, `{"error":"not implemented"}`, http.StatusNotImplemented)
	}
}

// HealthHandler returns an http.HandlerFunc for GET /health.
// It always responds with HTTP 200 and {"status":"ok"}.
//
// This is a stub — returns 501 Not Implemented. Full implementation is in task group 4.
func HealthHandler() http.HandlerFunc {
	return func(w http.ResponseWriter, r *http.Request) {
		http.Error(w, `{"error":"not implemented"}`, http.StatusNotImplemented)
	}
}
