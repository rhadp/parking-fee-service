package main

import (
	"encoding/json"
	"log"
	"net/http"
	"strconv"
	"strings"
)

// writeJSON writes a JSON response with the given status code.
func writeJSON(w http.ResponseWriter, status int, data any) {
	w.Header().Set("Content-Type", "application/json")
	w.WriteHeader(status)
	if err := json.NewEncoder(w).Encode(data); err != nil {
		log.Printf("failed to encode JSON response: %v", err)
	}
}

// writeError writes a JSON error response.
func writeError(w http.ResponseWriter, status int, message string) {
	writeJSON(w, status, ErrorResponse{Error: message})
}

// recoveryMiddleware recovers from panics and returns HTTP 500 with a JSON error body.
func recoveryMiddleware(next http.Handler) http.Handler {
	return http.HandlerFunc(func(w http.ResponseWriter, r *http.Request) {
		defer func() {
			if err := recover(); err != nil {
				log.Printf("panic recovered: %v", err)
				writeError(w, http.StatusInternalServerError, "internal server error")
			}
		}()
		next.ServeHTTP(w, r)
	})
}

// HandleHealth handles GET /health requests.
func HandleHealth(w http.ResponseWriter, r *http.Request) {
	writeJSON(w, http.StatusOK, map[string]string{"status": "ok"})
}

// HandleOperatorLookup returns a handler for GET /operators?lat=...&lon=... requests.
func HandleOperatorLookup(store *Store) http.HandlerFunc {
	return func(w http.ResponseWriter, r *http.Request) {
		latStr := r.URL.Query().Get("lat")
		lonStr := r.URL.Query().Get("lon")

		if latStr == "" {
			writeError(w, http.StatusBadRequest, "missing required parameter: lat")
			return
		}
		if lonStr == "" {
			writeError(w, http.StatusBadRequest, "missing required parameter: lon")
			return
		}

		lat, err := strconv.ParseFloat(latStr, 64)
		if err != nil {
			writeError(w, http.StatusBadRequest, "invalid lat parameter: must be a number")
			return
		}
		lon, err := strconv.ParseFloat(lonStr, 64)
		if err != nil {
			writeError(w, http.StatusBadRequest, "invalid lon parameter: must be a number")
			return
		}

		if lat < -90 || lat > 90 {
			writeError(w, http.StatusBadRequest, "lat parameter out of range: must be between -90 and 90")
			return
		}
		if lon < -180 || lon > 180 {
			writeError(w, http.StatusBadRequest, "lon parameter out of range: must be between -180 and 180")
			return
		}

		operators := store.FindOperatorsByLocation(lat, lon)
		if operators == nil {
			operators = []Operator{}
		}
		writeJSON(w, http.StatusOK, operators)
	}
}

// HandleAdapterMetadata returns a handler for GET /operators/{id}/adapter requests.
func HandleAdapterMetadata(store *Store) http.HandlerFunc {
	return func(w http.ResponseWriter, r *http.Request) {
		// Extract operator ID from path: /operators/{id}/adapter
		path := strings.TrimPrefix(r.URL.Path, "/operators/")
		id := strings.TrimSuffix(path, "/adapter")

		meta, ok := store.GetAdapterMetadata(id)
		if !ok {
			writeError(w, http.StatusNotFound, "operator not found: "+id)
			return
		}
		writeJSON(w, http.StatusOK, meta)
	}
}

// handleNotFound returns a 404 JSON error for undefined routes.
func handleNotFound(w http.ResponseWriter, r *http.Request) {
	writeError(w, http.StatusNotFound, "not found: "+r.URL.Path)
}

// NewRouter creates a configured HTTP handler with all routes and middleware.
func NewRouter(store *Store) http.Handler {
	mux := http.NewServeMux()

	mux.HandleFunc("GET /health", HandleHealth)
	mux.HandleFunc("GET /operators/{id}/adapter", HandleAdapterMetadata(store))
	mux.HandleFunc("GET /operators", HandleOperatorLookup(store))

	// Default handler for undefined routes
	handler := http.HandlerFunc(func(w http.ResponseWriter, r *http.Request) {
		// Check if the default mux handler would match
		_, pattern := mux.Handler(r)
		if pattern == "" {
			handleNotFound(w, r)
			return
		}
		mux.ServeHTTP(w, r)
	})

	return recoveryMiddleware(handler)
}
