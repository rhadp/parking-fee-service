// Package api implements the REST API handlers and middleware for the
// CLOUD_GATEWAY service.
package api

import (
	"encoding/json"
	"net/http"
	"strings"

	"github.com/rhadp/parking-fee-service/backend/cloud-gateway/state"
)

// AuthMiddleware returns HTTP middleware that validates Bearer tokens against
// the state store for the VIN extracted from the request path.
//
// The VIN is obtained from the "vin" path value (Go 1.22+ ServeMux pattern).
// If the token is missing, invalid, or not associated with the target VIN,
// the middleware responds with 401 Unauthorized.
func AuthMiddleware(store *state.Store, next http.Handler) http.Handler {
	return http.HandlerFunc(func(w http.ResponseWriter, r *http.Request) {
		token := extractBearerToken(r)
		if token == "" {
			writeError(w, http.StatusUnauthorized, "missing or invalid authorization header", "UNAUTHORIZED")
			return
		}

		vin := r.PathValue("vin")
		if vin == "" {
			writeError(w, http.StatusBadRequest, "missing VIN in path", "BAD_REQUEST")
			return
		}

		if !store.ValidateToken(token, vin) {
			writeError(w, http.StatusUnauthorized, "invalid or unauthorized token", "UNAUTHORIZED")
			return
		}

		next.ServeHTTP(w, r)
	})
}

// extractBearerToken extracts the token from the Authorization header.
// Returns an empty string if the header is missing or not a Bearer token.
func extractBearerToken(r *http.Request) string {
	auth := r.Header.Get("Authorization")
	if auth == "" {
		return ""
	}

	parts := strings.SplitN(auth, " ", 2)
	if len(parts) != 2 || !strings.EqualFold(parts[0], "bearer") {
		return ""
	}

	return strings.TrimSpace(parts[1])
}

// ErrorResponse is the JSON structure for error responses.
type ErrorResponse struct {
	Error string `json:"error"`
	Code  string `json:"code"`
}

// writeError sends a JSON error response with the given status code.
func writeError(w http.ResponseWriter, statusCode int, message, code string) {
	w.Header().Set("Content-Type", "application/json")
	w.WriteHeader(statusCode)
	json.NewEncoder(w).Encode(ErrorResponse{
		Error: message,
		Code:  code,
	})
}
