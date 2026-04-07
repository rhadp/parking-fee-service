package auth

import (
	"encoding/json"
	"net/http"
	"strings"

	"parking-fee-service/backend/cloud-gateway/config"
	"parking-fee-service/backend/cloud-gateway/model"
)

// Middleware returns an HTTP middleware that validates bearer tokens
// and enforces VIN authorization for vehicle endpoints.
func Middleware(cfg *model.Config) func(http.Handler) http.Handler {
	return func(next http.Handler) http.Handler {
		return http.HandlerFunc(func(w http.ResponseWriter, r *http.Request) {
			w.Header().Set("Content-Type", "application/json")

			// Extract bearer token from Authorization header.
			authHeader := r.Header.Get("Authorization")
			if authHeader == "" || !strings.HasPrefix(authHeader, "Bearer ") {
				w.WriteHeader(http.StatusUnauthorized)
				json.NewEncoder(w).Encode(map[string]string{"error": "unauthorized"})
				return
			}
			token := strings.TrimPrefix(authHeader, "Bearer ")

			// Validate token against config.
			vin, ok := config.GetVINForToken(cfg, token)
			if !ok {
				w.WriteHeader(http.StatusUnauthorized)
				json.NewEncoder(w).Encode(map[string]string{"error": "unauthorized"})
				return
			}

			// Extract VIN from URL path and check authorization.
			pathVIN := r.PathValue("vin")
			if pathVIN == "" {
				// Fallback: parse VIN from URL path manually.
				pathVIN = extractVINFromPath(r.URL.Path)
			}

			if pathVIN != "" && pathVIN != vin {
				w.WriteHeader(http.StatusForbidden)
				json.NewEncoder(w).Encode(map[string]string{"error": "forbidden"})
				return
			}

			next.ServeHTTP(w, r)
		})
	}
}

// extractVINFromPath extracts the VIN from URL paths like /vehicles/{vin}/commands.
func extractVINFromPath(path string) string {
	parts := strings.Split(strings.Trim(path, "/"), "/")
	if len(parts) >= 2 && parts[0] == "vehicles" {
		return parts[1]
	}
	return ""
}
