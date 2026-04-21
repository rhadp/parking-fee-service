package auth

import (
	"context"
	"encoding/json"
	"net/http"
	"strings"

	"github.com/rhadp/parking-fee-service/backend/cloud-gateway/config"
)

// contextKey is an unexported type for context keys in this package.
type contextKey int

const tokenKey contextKey = iota

// TokenFromContext retrieves the bearer token stored by the auth middleware.
func TokenFromContext(ctx context.Context) (string, bool) {
	t, ok := ctx.Value(tokenKey).(string)
	return t, ok
}

// writeJSONError writes a JSON error response with the given status code.
func writeJSONError(w http.ResponseWriter, status int, message string) {
	w.Header().Set("Content-Type", "application/json")
	w.WriteHeader(status)
	_ = json.NewEncoder(w).Encode(map[string]string{"error": message})
}

// Middleware returns an HTTP middleware that authenticates bearer tokens.
// It skips authentication for the /health endpoint.
func Middleware(cfg *config.Config) func(http.Handler) http.Handler {
	return func(next http.Handler) http.Handler {
		return http.HandlerFunc(func(w http.ResponseWriter, r *http.Request) {
			// Skip auth for health check.
			if r.URL.Path == "/health" {
				next.ServeHTTP(w, r)
				return
			}

			// Extract bearer token from Authorization header.
			authHeader := r.Header.Get("Authorization")
			token, found := strings.CutPrefix(authHeader, "Bearer ")
			if !found || token == "" {
				writeJSONError(w, http.StatusUnauthorized, "unauthorized")
				return
			}

			// Validate token against configuration.
			configVIN, ok := cfg.GetVINForToken(token)
			if !ok {
				writeJSONError(w, http.StatusUnauthorized, "unauthorized")
				return
			}

			// Check that the token's VIN matches the {vin} path parameter.
			pathVIN := r.PathValue("vin")
			if pathVIN != configVIN {
				writeJSONError(w, http.StatusForbidden, "forbidden")
				return
			}

			// Store token in context for downstream handlers.
			ctx := context.WithValue(r.Context(), tokenKey, token)
			next.ServeHTTP(w, r.WithContext(ctx))
		})
	}
}
