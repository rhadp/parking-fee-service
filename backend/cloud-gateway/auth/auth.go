// Package auth provides HTTP middleware for bearer token authentication
// and VIN authorization.
package auth

import (
	"encoding/json"
	"net/http"
	"strings"

	"github.com/rhadp/parking-fee-service/backend/cloud-gateway/config"
	"github.com/rhadp/parking-fee-service/backend/cloud-gateway/model"
)

// Middleware returns an HTTP middleware that validates bearer tokens and
// enforces VIN authorization based on the provided configuration.
// It extracts the bearer token from the Authorization header, validates it
// against the config, and checks that the token's VIN matches the VIN in
// the URL path. The /health endpoint is not protected.
func Middleware(cfg *model.Config) func(http.Handler) http.Handler {
	return func(next http.Handler) http.Handler {
		return http.HandlerFunc(func(w http.ResponseWriter, r *http.Request) {
			// Extract bearer token from Authorization header
			authHeader := r.Header.Get("Authorization")
			if authHeader == "" || !strings.HasPrefix(authHeader, "Bearer ") {
				writeError(w, http.StatusUnauthorized, "unauthorized")
				return
			}

			token := strings.TrimPrefix(authHeader, "Bearer ")

			// Validate token against config
			tokenVIN, ok := config.GetVINForToken(cfg, token)
			if !ok {
				writeError(w, http.StatusUnauthorized, "unauthorized")
				return
			}

			// Extract VIN from URL path: /vehicles/{vin}/commands...
			pathVIN := extractVINFromPath(r.URL.Path)

			// Check VIN authorization
			if pathVIN != tokenVIN {
				writeError(w, http.StatusForbidden, "forbidden")
				return
			}

			next.ServeHTTP(w, r)
		})
	}
}

// extractVINFromPath extracts the VIN from a URL path of the form
// /vehicles/{vin}/commands or /vehicles/{vin}/commands/{command_id}.
func extractVINFromPath(path string) string {
	// Split path: ["", "vehicles", "{vin}", "commands", ...]
	parts := strings.Split(strings.TrimPrefix(path, "/"), "/")
	if len(parts) >= 2 && parts[0] == "vehicles" {
		return parts[1]
	}
	return ""
}

// writeError writes a JSON error response with the given status code and message.
func writeError(w http.ResponseWriter, status int, message string) {
	w.Header().Set("Content-Type", "application/json")
	w.WriteHeader(status)
	json.NewEncoder(w).Encode(map[string]string{"error": message})
}
