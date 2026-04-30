// Package auth provides HTTP middleware for bearer token authentication
// and VIN authorization.
package auth

import (
	"net/http"

	"github.com/rhadp/parking-fee-service/backend/cloud-gateway/model"
)

// Middleware returns an HTTP middleware that validates bearer tokens and
// enforces VIN authorization based on the provided configuration.
func Middleware(cfg *model.Config) func(http.Handler) http.Handler {
	// TODO: implement
	return func(next http.Handler) http.Handler {
		return http.HandlerFunc(func(w http.ResponseWriter, r *http.Request) {
			next.ServeHTTP(w, r)
		})
	}
}
