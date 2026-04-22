package main

import "net/http"

// Server is the mock parking operator HTTP server.
// It manages in-memory parking sessions.
type Server struct {
	// Stub: session store and rate configuration will be added in task group 3.
}

// NewServer creates a new parking operator server.
func NewServer() *Server {
	return &Server{}
}

// Handler returns the HTTP handler for the parking operator server.
// Routes:
//   - POST /parking/start  — start a new parking session
//   - POST /parking/stop   — stop an active parking session
//   - GET  /parking/status/{session_id} — query session state
func (s *Server) Handler() http.Handler {
	// Stub: returns an empty mux. Will be implemented in task group 3.
	return http.NewServeMux()
}
