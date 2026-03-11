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
// Returns ("", error) for invalid tokens, (vin, nil) for valid tokens.
func (ts *TokenStore) ValidateToken(token, vin string) (bool, error) {
	// Stub - to be implemented
	return false, nil
}

// AuthMiddleware returns an HTTP middleware that validates bearer tokens.
func AuthMiddleware(tokenStore *TokenStore, knownVINs map[string]bool) func(http.Handler) http.Handler {
	// Stub - to be implemented
	return func(next http.Handler) http.Handler {
		return next
	}
}
