package auth

import (
	"net/http"
	"parking-fee-service/backend/cloud-gateway/model"
)

// contextKey is an unexported type for context keys in this package.
type contextKey string

// TokenContextKey is the context key for the bearer token.
const TokenContextKey contextKey = "token"

// Middleware returns an HTTP middleware that validates bearer tokens against the config.
func Middleware(cfg *model.Config) func(http.Handler) http.Handler {
	return func(next http.Handler) http.Handler {
		return http.HandlerFunc(func(w http.ResponseWriter, r *http.Request) {
			// Stub: pass through without validation
			next.ServeHTTP(w, r)
		})
	}
}
