package main

import (
	"encoding/json"
	"fmt"
	"net/http"
	"strconv"
)

// writeJSON writes a JSON response with the given status code.
func writeJSON(w http.ResponseWriter, status int, data any) {
	w.Header().Set("Content-Type", "application/json")
	w.WriteHeader(status)
	_ = json.NewEncoder(w).Encode(data)
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
				writeError(w, http.StatusInternalServerError, fmt.Sprintf("internal server error: %v", err))
			}
		}()
		next.ServeHTTP(w, r)
	})
}

// HandleHealth handles GET /health requests.
func HandleHealth(w http.ResponseWriter, r *http.Request) {
	writeJSON(w, http.StatusOK, map[string]string{"status": "ok"})
}

// HandleOperatorLookup returns a handler for GET /operators?lat=...&lon=...
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

// HandleAdapterMetadata returns a handler for GET /operators/{id}/adapter
func HandleAdapterMetadata(store *Store) http.HandlerFunc {
	return func(w http.ResponseWriter, r *http.Request) {
		operatorID := r.PathValue("id")

		metadata, ok := store.GetAdapterMetadata(operatorID)
		if !ok {
			writeError(w, http.StatusNotFound, fmt.Sprintf("operator not found: %s", operatorID))
			return
		}
		writeJSON(w, http.StatusOK, metadata)
	}
}

// handleNotFound returns a JSON 404 for undefined routes.
func handleNotFound(w http.ResponseWriter, r *http.Request) {
	writeError(w, http.StatusNotFound, fmt.Sprintf("not found: %s", r.URL.Path))
}

// NewRouter creates the HTTP router with all routes registered.
func NewRouter(store *Store) http.Handler {
	mux := http.NewServeMux()

	mux.HandleFunc("GET /health", HandleHealth)
	mux.HandleFunc("GET /operators", HandleOperatorLookup(store))
	mux.Handle("GET /operators/{id}/adapter", HandleAdapterMetadata(store))

	// Default handler for undefined routes
	mux.HandleFunc("/", handleNotFound)

	return recoveryMiddleware(mux)
}
