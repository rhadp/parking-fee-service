// Package auth handles bearer token validation and VIN authorization.
package auth

import (
	"github.com/rhadp/parking-fee-service/backend/cloud-gateway/model"
)

// Authenticator manages token-to-VIN mappings for authorization.
type Authenticator struct{}

// NewAuthenticator creates an Authenticator from token-to-VIN mappings.
func NewAuthenticator(tokens []model.TokenMapping) *Authenticator {
	return nil
}

// ValidateToken extracts a bearer token from an Authorization header.
// Returns the token string or an error if the header is missing or malformed.
func ValidateToken(header string) (string, error) {
	return "", nil
}

// AuthorizeVIN checks whether the given token is authorized for the given VIN.
func (a *Authenticator) AuthorizeVIN(token, vin string) bool {
	return false
}
