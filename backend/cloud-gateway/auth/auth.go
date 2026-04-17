// Package auth provides HTTP authentication middleware for the CLOUD_GATEWAY.
package auth

import (
	"net/http"

	"github.com/sdv-demo/parking-fee-service/backend/cloud-gateway/config"
)

// contextKey is an unexported type for context keys to avoid collisions.
type contextKey struct{ name string }

// TokenKey is the context key under which the authenticated bearer token is stored.
var TokenKey = &contextKey{"token"}

// Middleware returns an HTTP middleware that:
//   - Skips auth for /health
//   - Extracts and validates the bearer token (401 if missing/invalid)
//   - Checks that the token's VIN matches the VIN in the URL path (403 if mismatch)
//   - Stores the token in the request context for downstream handlers
//
// The VIN is extracted from the URL path by manual splitting so the middleware
// works both when requests go through a ServeMux and with bare httptest.NewRequest.
//
// STUB: calls next without performing any auth checks.
func Middleware(cfg *config.Config) func(http.Handler) http.Handler {
	return func(next http.Handler) http.Handler {
		return http.HandlerFunc(func(w http.ResponseWriter, r *http.Request) {
			// STUB: pass-through — no auth enforcement implemented yet.
			next.ServeHTTP(w, r)
		})
	}
}
