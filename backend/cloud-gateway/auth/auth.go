package auth

import (
	"net/http"

	"github.com/rhadp/parking-fee-service/backend/cloud-gateway/config"
)

type contextKey string

// TokenContextKey is the context key for the authenticated bearer token.
const TokenContextKey contextKey = "auth_token"

// VINContextKey is the context key for the authenticated VIN.
const VINContextKey contextKey = "auth_vin"

// Middleware returns HTTP middleware that validates bearer tokens and
// enforces VIN authorization using the provided configuration.
func Middleware(cfg *config.Config) func(http.Handler) http.Handler {
	return func(next http.Handler) http.Handler {
		return next // stub: passes through without auth
	}
}
