package auth

import (
	"net/http"

	"parking-fee-service/backend/cloud-gateway/model"
)

// Middleware returns an HTTP middleware that validates bearer tokens
// and enforces VIN authorization for vehicle endpoints.
func Middleware(cfg *model.Config) func(http.Handler) http.Handler {
	panic("not implemented")
}
