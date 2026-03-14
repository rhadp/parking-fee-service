// Package handler provides HTTP request handlers for the cloud-gateway service.
package handler

import (
	"net/http"

	"github.com/rhadp/parking-fee-service/backend/cloud-gateway/auth"
	"github.com/rhadp/parking-fee-service/backend/cloud-gateway/model"
	"github.com/rhadp/parking-fee-service/backend/cloud-gateway/store"
)

// NATSPublisher abstracts NATS publishing for testability.
type NATSPublisher interface {
	PublishCommand(vin string, cmd model.Command, bearerToken string) error
}

// NewCommandHandler returns an http.HandlerFunc for command submission.
func NewCommandHandler(s *store.Store, pub NATSPublisher, a *auth.Authenticator) http.HandlerFunc {
	return func(w http.ResponseWriter, r *http.Request) {
		http.Error(w, "not implemented", http.StatusInternalServerError)
	}
}

// NewStatusHandler returns an http.HandlerFunc for command status queries.
func NewStatusHandler(s *store.Store, a *auth.Authenticator) http.HandlerFunc {
	return func(w http.ResponseWriter, r *http.Request) {
		http.Error(w, "not implemented", http.StatusInternalServerError)
	}
}

// HealthHandler returns an http.HandlerFunc for health checks.
func HealthHandler() http.HandlerFunc {
	return func(w http.ResponseWriter, r *http.Request) {
		http.Error(w, "not implemented", http.StatusInternalServerError)
	}
}
