package main

import "net/http"

// TokenStore maps bearer tokens to VINs.
type TokenStore struct {
	tokens map[string]string // token -> VIN
}

// NewTokenStore creates a new TokenStore with the given token-to-VIN mappings.
func NewTokenStore(tokens map[string]string) *TokenStore {
	return &TokenStore{tokens: tokens}
}

// ValidateToken checks if the token is valid and authorized for the given VIN.
// Returns (true, nil) if valid, (false, error) with details otherwise.
func (ts *TokenStore) ValidateToken(token, vin string) (bool, error) {
	// Stub: not yet implemented
	return false, nil
}

// AuthMiddleware returns HTTP middleware that validates bearer tokens.
func AuthMiddleware(tokenStore *TokenStore, knownVINs map[string]bool) func(http.Handler) http.Handler {
	// Stub: not yet implemented
	return func(next http.Handler) http.Handler {
		return next
	}
}
