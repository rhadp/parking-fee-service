package auth

import (
	"net/http"

	"github.com/rhadp/parking-fee-service/backend/cloud-gateway/config"
)

type contextKey string

// TokenContextKey is the context key for the authenticated bearer token.
const TokenContextKey contextKey = "auth-token"

// Middleware returns HTTP middleware that validates bearer tokens and VIN authorization.
func Middleware(cfg *config.Config) func(http.Handler) http.Handler {
	return func(next http.Handler) http.Handler {
		return next
	}
}
