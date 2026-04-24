// Package server implements the mock parking-operator HTTP server.
// Stub: provides only the New() constructor returning a no-op handler.
package server

import (
	"net/http"
)

// New creates a new parking-operator HTTP handler.
// Stub: returns a handler that responds 404 to all requests.
func New() http.Handler {
	mux := http.NewServeMux()
	return mux
}
