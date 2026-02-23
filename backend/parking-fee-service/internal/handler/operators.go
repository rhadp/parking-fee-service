package handler

import (
	"encoding/json"
	"fmt"
	"net/http"
	"strconv"

	"github.com/rhadp/parking-fee-service/backend/parking-fee-service/internal/geo"
	"github.com/rhadp/parking-fee-service/backend/parking-fee-service/internal/model"
	"github.com/rhadp/parking-fee-service/backend/parking-fee-service/internal/store"
)

// operatorsHandler holds the dependencies for the GET /operators endpoint.
type operatorsHandler struct {
	store           *store.Store
	fuzzinessMeters float64
}

// operatorLookupResult is the JSON structure for a single operator in the
// lookup response.
type operatorLookupResult struct {
	OperatorID string     `json:"operator_id"`
	Name       string     `json:"name"`
	Zone       zoneResult `json:"zone"`
	Rate       rateResult `json:"rate"`
}

// zoneResult is the zone sub-object in the operator lookup response.
type zoneResult struct {
	ZoneID  string        `json:"zone_id"`
	Name    string        `json:"name"`
	Polygon []model.Point `json:"polygon"`
}

// rateResult is the rate sub-object in the operator lookup response.
type rateResult struct {
	AmountPerHour float64 `json:"amount_per_hour"`
	Currency      string  `json:"currency"`
}

// operatorsResponse is the top-level JSON response for GET /operators.
type operatorsResponse struct {
	Operators []operatorLookupResult `json:"operators"`
}

// ServeHTTP handles GET /operators?lat={lat}&lon={lon}.
func (h *operatorsHandler) ServeHTTP(w http.ResponseWriter, r *http.Request) {
	// Parse and validate lat parameter.
	latStr := r.URL.Query().Get("lat")
	if latStr == "" {
		writeJSONError(w, http.StatusBadRequest, "missing required query parameter: lat")
		return
	}
	lat, err := strconv.ParseFloat(latStr, 64)
	if err != nil {
		writeJSONError(w, http.StatusBadRequest,
			fmt.Sprintf("invalid value for lat: must be a number between -90 and 90"))
		return
	}
	if lat < -90 || lat > 90 {
		writeJSONError(w, http.StatusBadRequest,
			fmt.Sprintf("invalid value for lat: must be a number between -90 and 90"))
		return
	}

	// Parse and validate lon parameter.
	lonStr := r.URL.Query().Get("lon")
	if lonStr == "" {
		writeJSONError(w, http.StatusBadRequest, "missing required query parameter: lon")
		return
	}
	lon, err := strconv.ParseFloat(lonStr, 64)
	if err != nil {
		writeJSONError(w, http.StatusBadRequest,
			fmt.Sprintf("invalid value for lon: must be a number between -180 and 180"))
		return
	}
	if lon < -180 || lon > 180 {
		writeJSONError(w, http.StatusBadRequest,
			fmt.Sprintf("invalid value for lon: must be a number between -180 and 180"))
		return
	}

	// Find matching operators using geofence engine.
	allOps := h.store.ListOperators()
	matches := geo.FindMatches(lat, lon, allOps, h.fuzzinessMeters)

	// Build response.
	results := make([]operatorLookupResult, 0, len(matches))
	for _, op := range matches {
		results = append(results, operatorLookupResult{
			OperatorID: op.ID,
			Name:       op.Name,
			Zone: zoneResult{
				ZoneID:  op.Zone.ID,
				Name:    op.Zone.Name,
				Polygon: op.Zone.Polygon,
			},
			Rate: rateResult{
				AmountPerHour: op.Rate.AmountPerHour,
				Currency:      op.Rate.Currency,
			},
		})
	}

	w.Header().Set("Content-Type", "application/json")
	w.WriteHeader(http.StatusOK)
	json.NewEncoder(w).Encode(operatorsResponse{Operators: results})
}
