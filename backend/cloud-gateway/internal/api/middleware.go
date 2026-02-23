// Package api provides the HTTP handlers and middleware for the CLOUD_GATEWAY
// REST API. It implements bearer token authentication, vehicle command
// processing, status queries, and health checks.
package api

import (
	"encoding/json"
	"net/http"
	"strings"
)

// AuthMiddleware returns an HTTP middleware that validates bearer tokens.
// Requests with a missing, malformed, or invalid Authorization header receive
// a 401 Unauthorized response. The middleware compares the token against the
// configured validToken value.
func AuthMiddleware(validToken string, next http.Handler) http.Handler {
	return http.HandlerFunc(func(w http.ResponseWriter, r *http.Request) {
		authHeader := r.Header.Get("Authorization")
		if authHeader == "" {
			writeJSON(w, http.StatusUnauthorized, map[string]string{"error": "unauthorized"})
			return
		}

		if !strings.HasPrefix(authHeader, "Bearer ") {
			writeJSON(w, http.StatusUnauthorized, map[string]string{"error": "unauthorized"})
			return
		}

		token := strings.TrimPrefix(authHeader, "Bearer ")
		if token != validToken {
			writeJSON(w, http.StatusUnauthorized, map[string]string{"error": "unauthorized"})
			return
		}

		next.ServeHTTP(w, r)
	})
}

// writeJSON encodes v as JSON and writes it to the response with the given
// status code and Content-Type: application/json header.
func writeJSON(w http.ResponseWriter, statusCode int, v interface{}) {
	w.Header().Set("Content-Type", "application/json")
	w.WriteHeader(statusCode)
	json.NewEncoder(w).Encode(v)
}
