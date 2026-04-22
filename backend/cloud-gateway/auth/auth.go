package auth

import (
	"context"
	"encoding/json"
	"net/http"
	"strings"

	"github.com/rhadp/parking-fee-service/backend/cloud-gateway/config"
)

type contextKey string

// TokenContextKey is the context key for the authenticated bearer token.
const TokenContextKey contextKey = "auth_token"

// VINContextKey is the context key for the authenticated VIN.
const VINContextKey contextKey = "auth_vin"

// writeJSONError writes a JSON error response with the given status code and message.
func writeJSONError(w http.ResponseWriter, statusCode int, message string) {
	w.Header().Set("Content-Type", "application/json")
	w.WriteHeader(statusCode)
	json.NewEncoder(w).Encode(map[string]string{"error": message})
}

// Middleware returns HTTP middleware that validates bearer tokens and
// enforces VIN authorization using the provided configuration.
func Middleware(cfg *config.Config) func(http.Handler) http.Handler {
	return func(next http.Handler) http.Handler {
		return http.HandlerFunc(func(w http.ResponseWriter, r *http.Request) {
			// Extract bearer token from Authorization header.
			authHeader := r.Header.Get("Authorization")
			if authHeader == "" || !strings.HasPrefix(authHeader, "Bearer ") {
				writeJSONError(w, http.StatusUnauthorized, "unauthorized")
				return
			}

			token := strings.TrimPrefix(authHeader, "Bearer ")

			// Validate token against configuration.
			tokenVIN, ok := cfg.GetVINForToken(token)
			if !ok {
				writeJSONError(w, http.StatusUnauthorized, "unauthorized")
				return
			}

			// Extract VIN from URL path and check authorization.
			pathVIN := r.PathValue("vin")
			if pathVIN != tokenVIN {
				writeJSONError(w, http.StatusForbidden, "forbidden")
				return
			}

			// Set token and VIN in request context for downstream handlers.
			ctx := context.WithValue(r.Context(), TokenContextKey, token)
			ctx = context.WithValue(ctx, VINContextKey, tokenVIN)
			next.ServeHTTP(w, r.WithContext(ctx))
		})
	}
}
