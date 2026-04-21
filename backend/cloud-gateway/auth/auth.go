package auth

import (
	"net/http"

	"github.com/rhadp/parking-fee-service/backend/cloud-gateway/config"
)

// Middleware returns an HTTP middleware that authenticates bearer tokens.
func Middleware(cfg *config.Config) func(http.Handler) http.Handler {
	return func(next http.Handler) http.Handler {
		return http.HandlerFunc(func(w http.ResponseWriter, r *http.Request) {
			// stub: pass-through, no authentication
			next.ServeHTTP(w, r)
		})
	}
}
