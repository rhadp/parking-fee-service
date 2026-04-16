package auth

import (
	"context"
	"encoding/json"
	"net/http"
	"strings"

	"parking-fee-service/backend/cloud-gateway/model"
)

// contextKey is an unexported type for context keys in this package.
type contextKey string

// TokenContextKey is the context key for the bearer token.
const TokenContextKey contextKey = "token"

// vinFromPath extracts the VIN from a URL path like /vehicles/{vin}/commands[/{id}].
func vinFromPath(path string) string {
	parts := strings.Split(path, "/")
	// parts: ["", "vehicles", "{vin}", ...]
	if len(parts) >= 3 && parts[1] == "vehicles" {
		return parts[2]
	}
	return ""
}

// writeError writes a JSON error response with Content-Type: application/json.
func writeError(w http.ResponseWriter, status int, msg string) {
	w.Header().Set("Content-Type", "application/json")
	w.WriteHeader(status)
	_ = json.NewEncoder(w).Encode(map[string]string{"error": msg})
}

// getVINForToken looks up the VIN for a bearer token in the config.
func getVINForToken(cfg *model.Config, token string) (string, bool) {
	for _, tm := range cfg.Tokens {
		if tm.Token == token {
			return tm.VIN, true
		}
	}
	return "", false
}

// Middleware returns an HTTP middleware that validates bearer tokens against the config.
// It extracts the bearer token from the Authorization header, validates it, and checks
// that the token's associated VIN matches the VIN in the URL path.
// Requests to /health bypass authentication.
func Middleware(cfg *model.Config) func(http.Handler) http.Handler {
	return func(next http.Handler) http.Handler {
		return http.HandlerFunc(func(w http.ResponseWriter, r *http.Request) {
			// Skip auth for the health endpoint.
			if r.URL.Path == "/health" {
				next.ServeHTTP(w, r)
				return
			}

			// Extract bearer token from Authorization header.
			authHeader := r.Header.Get("Authorization")
			var token string
			if t, ok := strings.CutPrefix(authHeader, "Bearer "); ok {
				token = t
			}

			// Validate token against config.
			vin, ok := getVINForToken(cfg, token)
			if !ok {
				writeError(w, http.StatusUnauthorized, "unauthorized")
				return
			}

			// Extract VIN from URL path and verify it matches the token's VIN.
			pathVIN := vinFromPath(r.URL.Path)
			if pathVIN != vin {
				writeError(w, http.StatusForbidden, "forbidden")
				return
			}

			// Store token in context so handlers can access it.
			ctx := context.WithValue(r.Context(), TokenContextKey, token)
			next.ServeHTTP(w, r.WithContext(ctx))
		})
	}
}
