package main

import (
	"net/http"
	"strings"
)

// TokenStore maps bearer tokens to VINs.
type TokenStore struct {
	tokens map[string]string // token -> VIN
}

// NewTokenStore creates a new TokenStore with the given token-to-VIN mappings.
func NewTokenStore(tokens map[string]string) *TokenStore {
	return &TokenStore{tokens: tokens}
}

// ValidateToken checks if the token is valid and authorized for the given VIN.
// Returns (true, nil) if the token maps to the exact VIN.
func (ts *TokenStore) ValidateToken(token, vin string) (bool, error) {
	mappedVIN, ok := ts.tokens[token]
	if !ok {
		return false, nil
	}
	return mappedVIN == vin, nil
}

// LookupToken returns the VIN associated with a token, or empty string if not found.
func (ts *TokenStore) LookupToken(token string) (string, bool) {
	vin, ok := ts.tokens[token]
	return vin, ok
}

// AuthMiddleware returns HTTP middleware that validates bearer tokens.
// It extracts the token from the Authorization header, validates it against the
// token store, and checks that the token is authorized for the VIN in the URL path.
func AuthMiddleware(tokenStore *TokenStore, knownVINs map[string]bool) func(http.Handler) http.Handler {
	return func(next http.Handler) http.Handler {
		return http.HandlerFunc(func(w http.ResponseWriter, r *http.Request) {
			// Extract Authorization header
			authHeader := r.Header.Get("Authorization")
			if authHeader == "" || !strings.HasPrefix(authHeader, "Bearer ") {
				writeError(w, http.StatusUnauthorized, "missing or invalid authorization header")
				return
			}

			// Extract token
			token := strings.TrimPrefix(authHeader, "Bearer ")

			// Look up token
			tokenVIN, ok := tokenStore.LookupToken(token)
			if !ok {
				writeError(w, http.StatusUnauthorized, "invalid token")
				return
			}

			// Extract VIN from URL path
			vin := r.PathValue("vin")

			// Check VIN authorization
			if tokenVIN != vin {
				writeError(w, http.StatusForbidden, "token not authorized for this vehicle")
				return
			}

			next.ServeHTTP(w, r)
		})
	}
}
