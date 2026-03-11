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
func (ts *TokenStore) ValidateToken(token, vin string) (bool, error) {
	associatedVIN, exists := ts.tokens[token]
	if !exists {
		return false, nil
	}
	return associatedVIN == vin, nil
}

// LookupToken returns the VIN associated with the token, or ("", false) if not found.
func (ts *TokenStore) LookupToken(token string) (string, bool) {
	vin, exists := ts.tokens[token]
	return vin, exists
}

// AuthMiddleware returns an HTTP middleware that validates bearer tokens.
func AuthMiddleware(tokenStore *TokenStore, knownVINs map[string]bool) func(http.Handler) http.Handler {
	return func(next http.Handler) http.Handler {
		return http.HandlerFunc(func(w http.ResponseWriter, r *http.Request) {
			// Extract VIN from path
			vin := r.PathValue("vin")
			if vin == "" {
				next.ServeHTTP(w, r)
				return
			}

			// Check Authorization header
			authHeader := r.Header.Get("Authorization")
			if authHeader == "" || !strings.HasPrefix(authHeader, "Bearer ") {
				writeError(w, http.StatusUnauthorized, "missing or invalid authorization header")
				return
			}

			token := strings.TrimPrefix(authHeader, "Bearer ")

			// Look up the token
			associatedVIN, exists := tokenStore.LookupToken(token)
			if !exists {
				writeError(w, http.StatusUnauthorized, "invalid token")
				return
			}

			// Check if the VIN is known
			if !knownVINs[vin] {
				writeError(w, http.StatusNotFound, "unknown vehicle")
				return
			}

			// Check if the token is authorized for this VIN
			if associatedVIN != vin {
				writeError(w, http.StatusForbidden, "token not authorized for this vehicle")
				return
			}

			next.ServeHTTP(w, r)
		})
	}
}
