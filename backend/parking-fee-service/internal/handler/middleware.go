package handler

import (
	"encoding/json"
	"net/http"
	"strings"
)

// AuthMiddleware wraps an http.Handler and validates bearer token
// authentication. It checks the Authorization header for the "Bearer" scheme
// and verifies the token against the provided set of valid tokens.
//
// Protected endpoints must be wrapped with this middleware.
func AuthMiddleware(next http.Handler, validTokens []string) http.Handler {
	tokenSet := make(map[string]bool, len(validTokens))
	for _, t := range validTokens {
		tokenSet[t] = true
	}

	return http.HandlerFunc(func(w http.ResponseWriter, r *http.Request) {
		authHeader := r.Header.Get("Authorization")
		if authHeader == "" {
			writeJSONError(w, http.StatusUnauthorized, "missing authorization header")
			return
		}

		if !strings.HasPrefix(authHeader, "Bearer ") {
			writeJSONError(w, http.StatusUnauthorized, "invalid authorization scheme")
			return
		}

		token := authHeader[len("Bearer "):]
		if !tokenSet[token] {
			writeJSONError(w, http.StatusUnauthorized, "invalid token")
			return
		}

		next.ServeHTTP(w, r)
	})
}

// writeJSONError writes a JSON error response with the given status code and
// message.
func writeJSONError(w http.ResponseWriter, statusCode int, message string) {
	w.Header().Set("Content-Type", "application/json")
	w.WriteHeader(statusCode)
	json.NewEncoder(w).Encode(map[string]string{"error": message})
}
