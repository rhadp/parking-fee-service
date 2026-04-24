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
const TokenContextKey contextKey = "auth-token"

// Middleware returns HTTP middleware that validates bearer tokens and VIN authorization.
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

			// Validate token against configured token-VIN mappings.
			vin, ok := cfg.GetVINForToken(token)
			if !ok {
				writeJSONError(w, http.StatusUnauthorized, "unauthorized")
				return
			}

			// Extract VIN from URL path and enforce VIN authorization.
			pathVIN := extractVINFromPath(r.URL.Path)
			if pathVIN != vin {
				writeJSONError(w, http.StatusForbidden, "forbidden")
				return
			}

			// Store token in context for downstream handlers.
			ctx := context.WithValue(r.Context(), TokenContextKey, token)
			next.ServeHTTP(w, r.WithContext(ctx))
		})
	}
}

// extractVINFromPath extracts the VIN from URL paths matching
// /vehicles/{vin}/commands or /vehicles/{vin}/commands/{command_id}.
func extractVINFromPath(path string) string {
	parts := strings.Split(strings.TrimPrefix(path, "/"), "/")
	// Expected: ["vehicles", "{vin}", "commands", ...]
	if len(parts) >= 2 && parts[0] == "vehicles" {
		return parts[1]
	}
	return ""
}

// writeJSONError writes a JSON error response with the given status code.
func writeJSONError(w http.ResponseWriter, statusCode int, message string) {
	w.Header().Set("Content-Type", "application/json")
	w.WriteHeader(statusCode)
	json.NewEncoder(w).Encode(map[string]string{"error": message})
}
