// Package auth handles bearer token validation and VIN authorization.
package auth

import (
	"errors"
	"strings"

	"github.com/rhadp/parking-fee-service/backend/cloud-gateway/model"
)

// Authenticator manages token-to-VIN mappings for authorization.
type Authenticator struct {
	// tokenToVIN maps each bearer token to the VIN it is authorized for.
	tokenToVIN map[string]string
}

// NewAuthenticator creates an Authenticator from token-to-VIN mappings.
func NewAuthenticator(tokens []model.TokenMapping) *Authenticator {
	m := make(map[string]string, len(tokens))
	for _, t := range tokens {
		m[t.Token] = t.VIN
	}
	return &Authenticator{tokenToVIN: m}
}

// ValidateToken extracts a bearer token from an Authorization header.
// The header must have the form "Bearer <token>" where <token> is non-empty.
// Returns the token string or an error if the header is missing or malformed.
func ValidateToken(header string) (string, error) {
	const prefix = "Bearer "
	if !strings.HasPrefix(header, prefix) {
		return "", errors.New("authorization header must start with 'Bearer '")
	}
	token := header[len(prefix):]
	if token == "" {
		return "", errors.New("bearer token must not be empty")
	}
	return token, nil
}

// AuthorizeVIN checks whether the given token is authorized for the given VIN.
func (a *Authenticator) AuthorizeVIN(token, vin string) bool {
	authorizedVIN, ok := a.tokenToVIN[token]
	if !ok {
		return false
	}
	return authorizedVIN == vin
}
