package handler

import (
	"encoding/json"
	"net/http"
	"strconv"

	"github.com/rhadp/parking-fee-service/backend/parking-fee-service/geo"
	"github.com/rhadp/parking-fee-service/backend/parking-fee-service/model"
	"github.com/rhadp/parking-fee-service/backend/parking-fee-service/store"
)

// writeJSON writes a JSON response with the given status code.
func writeJSON(w http.ResponseWriter, status int, v interface{}) {
	w.Header().Set("Content-Type", "application/json")
	w.WriteHeader(status)
	json.NewEncoder(w).Encode(v) //nolint:errcheck
}

// writeError writes a JSON error response.
func writeError(w http.ResponseWriter, status int, message string) {
	writeJSON(w, status, map[string]string{"error": message})
}

// NewOperatorHandler returns an HTTP handler for operator lookup by location.
func NewOperatorHandler(s *store.Store, zones []model.Zone, threshold float64) http.HandlerFunc {
	return func(w http.ResponseWriter, r *http.Request) {
		q := r.URL.Query()
		latStr := q.Get("lat")
		lonStr := q.Get("lon")

		if latStr == "" || lonStr == "" {
			writeError(w, http.StatusBadRequest, "lat and lon query parameters are required")
			return
		}

		lat, err := strconv.ParseFloat(latStr, 64)
		if err != nil {
			writeError(w, http.StatusBadRequest, "invalid coordinates")
			return
		}

		lon, err := strconv.ParseFloat(lonStr, 64)
		if err != nil {
			writeError(w, http.StatusBadRequest, "invalid coordinates")
			return
		}

		if lat < -90 || lat > 90 || lon < -180 || lon > 180 {
			writeError(w, http.StatusBadRequest, "invalid coordinates")
			return
		}

		point := model.Coordinate{Lat: lat, Lon: lon}
		zoneIDs := geo.FindMatchingZones(point, zones, threshold)
		operators := s.GetOperatorsByZoneIDs(zoneIDs)

		resp := make([]model.OperatorResponse, 0, len(operators))
		for _, op := range operators {
			resp = append(resp, model.OperatorResponse{
				ID:     op.ID,
				Name:   op.Name,
				ZoneID: op.ZoneID,
				Rate:   op.Rate,
			})
		}

		writeJSON(w, http.StatusOK, resp)
	}
}

// NewAdapterHandler returns an HTTP handler for adapter metadata retrieval.
func NewAdapterHandler(s *store.Store) http.HandlerFunc {
	return func(w http.ResponseWriter, r *http.Request) {
		id := r.PathValue("id")

		op, found := s.GetOperator(id)
		if !found {
			writeError(w, http.StatusNotFound, "operator not found")
			return
		}

		writeJSON(w, http.StatusOK, op.Adapter)
	}
}

// HealthHandler returns an HTTP handler for the health check endpoint.
func HealthHandler() http.HandlerFunc {
	return func(w http.ResponseWriter, r *http.Request) {
		writeJSON(w, http.StatusOK, map[string]string{"status": "ok"})
	}
}
