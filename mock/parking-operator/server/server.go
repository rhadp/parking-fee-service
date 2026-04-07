// Package server implements the mock parking-operator HTTP server.
// It provides in-memory session management with start/stop/status endpoints.
package server

import (
	"net/http"
)

// Server is the mock parking-operator HTTP server.
type Server struct {
	mux *http.ServeMux
}

// New creates a new parking-operator server.
// Stub: will be fully implemented in task group 3.
func New() *Server {
	return &Server{
		mux: http.NewServeMux(),
	}
}

// ServeHTTP implements the http.Handler interface.
func (s *Server) ServeHTTP(w http.ResponseWriter, r *http.Request) {
	s.mux.ServeHTTP(w, r)
}
