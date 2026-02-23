package api

import (
	"net/http"
	"sync"
)

// TelemetryData holds the cached telemetry state for a single vehicle.
type TelemetryData struct {
	VIN       string `json:"vin"`
	Locked    bool   `json:"locked"`
	Timestamp int64  `json:"timestamp"`
}

// TelemetryCache stores the latest telemetry data per VIN. It is safe for
// concurrent use.
type TelemetryCache struct {
	mu    sync.RWMutex
	cache map[string]TelemetryData
}

// NewTelemetryCache creates an empty telemetry cache.
func NewTelemetryCache() *TelemetryCache {
	return &TelemetryCache{
		cache: make(map[string]TelemetryData),
	}
}

// Update stores the latest telemetry data for the given VIN.
func (tc *TelemetryCache) Update(vin string, data TelemetryData) {
	tc.mu.Lock()
	defer tc.mu.Unlock()
	data.VIN = vin
	tc.cache[vin] = data
}

// Get retrieves the latest telemetry data for the given VIN.
// Returns the data and true if found, or zero value and false if not.
func (tc *TelemetryCache) Get(vin string) (TelemetryData, bool) {
	tc.mu.RLock()
	defer tc.mu.RUnlock()
	data, ok := tc.cache[vin]
	return data, ok
}

// StatusHandler handles GET /vehicles/{vin}/status requests. It reads the
// latest telemetry from the cache and returns it as JSON, or 404 if no
// telemetry is available for the requested VIN.
type StatusHandler struct {
	cache *TelemetryCache
}

// NewStatusHandler creates a handler with the given telemetry cache.
func NewStatusHandler(cache *TelemetryCache) *StatusHandler {
	return &StatusHandler{cache: cache}
}

// ServeHTTP processes GET /vehicles/{vin}/status requests.
func (h *StatusHandler) ServeHTTP(w http.ResponseWriter, r *http.Request) {
	vin := r.PathValue("vin")
	if vin == "" {
		writeJSON(w, http.StatusBadRequest, map[string]string{"error": "missing VIN in URL path"})
		return
	}

	data, ok := h.cache.Get(vin)
	if !ok {
		writeJSON(w, http.StatusNotFound, map[string]string{"error": "no status available for vehicle"})
		return
	}

	writeJSON(w, http.StatusOK, data)
}
