// Package auth provides HTTP authentication middleware for the CLOUD_GATEWAY.
package auth

import (
	"context"
	"encoding/json"
	"net/http"
	"strings"

	"github.com/sdv-demo/parking-fee-service/backend/cloud-gateway/config"
)

// contextKey is an unexported type for context keys to avoid collisions.
type contextKey struct{ name string }

// TokenKey is the context key under which the authenticated bearer token is stored.
var TokenKey = &contextKey{"token"}

// writeError writes a JSON error response with the given status code and message.
func writeError(w http.ResponseWriter, status int, msg string) {
	w.Header().Set("Content-Type", "application/json")
	w.WriteHeader(status)
	body, _ := json.Marshal(map[string]string{"error": msg})
	_, _ = w.Write(body)
}

// extractBearerToken extracts the token from an "Authorization: Bearer <token>" header.
// Returns ("", false) if the header is missing or not in Bearer format.
func extractBearerToken(r *http.Request) (string, bool) {
	authHeader := r.Header.Get("Authorization")
	if authHeader == "" {
		return "", false
	}
	const prefix = "Bearer "
	if !strings.HasPrefix(authHeader, prefix) {
		return "", false
	}
	token := strings.TrimPrefix(authHeader, prefix)
	if token == "" {
		return "", false
	}
	return token, true
}

// extractVINFromPath extracts the VIN from URL paths of the form:
//   - /vehicles/{vin}/commands
//   - /vehicles/{vin}/commands/{command_id}
//
// Returns ("", false) if the path does not match either pattern.
// Uses manual path splitting so it works both through a ServeMux and
// with bare httptest.NewRequest (where r.PathValue would return "").
func extractVINFromPath(path string) (string, bool) {
	// Split on "/" — path starts with "/" so parts[0] is empty.
	parts := strings.Split(path, "/")
	// Expected: ["", "vehicles", "{vin}", "commands", ...]
	if len(parts) < 4 {
		return "", false
	}
	if parts[1] != "vehicles" {
		return "", false
	}
	if parts[3] != "commands" {
		return "", false
	}
	vin := parts[2]
	if vin == "" {
		return "", false
	}
	return vin, true
}

// Middleware returns an HTTP middleware that:
//   - Skips auth for /health
//   - Extracts and validates the bearer token (401 if missing/invalid)
//   - Checks that the token's VIN matches the VIN in the URL path (403 if mismatch)
//   - Stores the token in the request context for downstream handlers
//
// The VIN is extracted from the URL path by manual splitting so the middleware
// works both when requests go through a ServeMux and with bare httptest.NewRequest.
func Middleware(cfg *config.Config) func(http.Handler) http.Handler {
	return func(next http.Handler) http.Handler {
		return http.HandlerFunc(func(w http.ResponseWriter, r *http.Request) {
			// Skip auth for the health endpoint.
			if r.URL.Path == "/health" {
				next.ServeHTTP(w, r)
				return
			}

			// Extract bearer token from Authorization header.
			token, ok := extractBearerToken(r)
			if !ok {
				writeError(w, http.StatusUnauthorized, "unauthorized")
				return
			}

			// Validate token against configured mappings.
			configVIN, ok := cfg.GetVINForToken(token)
			if !ok {
				writeError(w, http.StatusUnauthorized, "unauthorized")
				return
			}

			// Extract VIN from URL path and compare.
			pathVIN, ok := extractVINFromPath(r.URL.Path)
			if !ok || pathVIN != configVIN {
				writeError(w, http.StatusForbidden, "forbidden")
				return
			}

			// Store the token in the context for downstream handlers.
			ctx := context.WithValue(r.Context(), TokenKey, token)
			next.ServeHTTP(w, r.WithContext(ctx))
		})
	}
}
