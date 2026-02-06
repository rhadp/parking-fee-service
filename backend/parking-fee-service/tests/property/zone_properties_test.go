// Package property contains property-based tests for the parking-fee-service.
package property

import (
	"encoding/json"
	"net/http"
	"net/http/httptest"
	"strconv"
	"testing"

	"github.com/leanovate/gopter"
	"github.com/leanovate/gopter/gen"
	"github.com/leanovate/gopter/prop"

	"github.com/sdv-parking-demo/backend/parking-fee-service/internal/handler"
	"github.com/sdv-parking-demo/backend/parking-fee-service/internal/model"
	"github.com/sdv-parking-demo/backend/parking-fee-service/internal/service"
	"github.com/sdv-parking-demo/backend/parking-fee-service/internal/store"
)

// testDemoZone is the demo zone used for testing.
var testDemoZone = model.Zone{
	ZoneID:          "demo-zone-001",
	OperatorName:    "Demo Parking Operator",
	HourlyRate:      2.50,
	Currency:        "USD",
	AdapterImageRef: "us-docker.pkg.dev/sdv-demo/adapters/demo-operator:v1.0.0",
	AdapterChecksum: "sha256:0000000000000000000000000000000000000000000000000000000000000000",
	Bounds: model.Bounds{
		MinLat: 37.0,
		MaxLat: 38.0,
		MinLng: -123.0,
		MaxLng: -122.0,
	},
}

// TestProperty1_ZoneContainment verifies that coordinates within bounds return zone,
// and coordinates outside bounds return nil.
// Feature: parking-fee-service, Property 1: Zone Containment
func TestProperty1_ZoneContainment(t *testing.T) {
	parameters := gopter.DefaultTestParameters()
	parameters.MinSuccessfulTests = 100

	properties := gopter.NewProperties(parameters)

	zoneStore := store.NewZoneStore([]model.Zone{testDemoZone})
	zoneService := service.NewZoneService(zoneStore)

	properties.Property("coordinates within bounds return zone, outside return nil", prop.ForAll(
		func(lat, lng float64) bool {
			zone := zoneService.FindZoneByLocation(lat, lng)
			if testDemoZone.Bounds.ContainsPoint(lat, lng) {
				return zone != nil && zone.ZoneID == testDemoZone.ZoneID
			}
			return zone == nil
		},
		gen.Float64Range(-90, 90),
		gen.Float64Range(-180, 180),
	))

	properties.TestingRun(t)
}

// TestProperty2_ZoneResponseCompleteness verifies that successful zone lookups
// return responses with all required fields present and non-empty.
// Feature: parking-fee-service, Property 2: Zone Response Completeness
func TestProperty2_ZoneResponseCompleteness(t *testing.T) {
	parameters := gopter.DefaultTestParameters()
	parameters.MinSuccessfulTests = 100

	properties := gopter.NewProperties(parameters)

	zoneStore := store.NewZoneStore([]model.Zone{testDemoZone})
	zoneService := service.NewZoneService(zoneStore)
	zoneHandler := handler.NewZoneHandler(zoneService, nil)

	// Generate coordinates within the demo zone bounds
	properties.Property("successful zone lookup has all required fields", prop.ForAll(
		func(lat, lng float64) bool {
			req := httptest.NewRequest("GET", "/api/v1/zones", nil)
			q := req.URL.Query()
			q.Add("lat", floatToString(lat))
			q.Add("lng", floatToString(lng))
			req.URL.RawQuery = q.Encode()

			rec := httptest.NewRecorder()
			zoneHandler.HandleGetZone(rec, req)

			if rec.Code != http.StatusOK {
				return true // Only test successful responses
			}

			var response model.ZoneResponse
			if err := json.Unmarshal(rec.Body.Bytes(), &response); err != nil {
				return false
			}

			return response.ZoneID != "" &&
				response.OperatorName != "" &&
				response.HourlyRate > 0 &&
				response.Currency != "" &&
				response.AdapterImageRef != "" &&
				response.AdapterChecksum != ""
		},
		gen.Float64Range(37.0, 38.0),   // Within demo zone lat
		gen.Float64Range(-123.0, -122.0), // Within demo zone lng
	))

	properties.TestingRun(t)
}

// TestProperty3_InvalidCoordinateValidation verifies that out-of-range coordinates
// return HTTP 400.
// Feature: parking-fee-service, Property 3: Invalid Coordinate Validation
func TestProperty3_InvalidCoordinateValidation(t *testing.T) {
	parameters := gopter.DefaultTestParameters()
	parameters.MinSuccessfulTests = 100

	properties := gopter.NewProperties(parameters)

	zoneStore := store.NewZoneStore([]model.Zone{testDemoZone})
	zoneService := service.NewZoneService(zoneStore)
	zoneHandler := handler.NewZoneHandler(zoneService, nil)

	// Test invalid latitudes (outside -90 to 90)
	properties.Property("invalid latitude returns 400", prop.ForAll(
		func(lat float64) bool {
			req := httptest.NewRequest("GET", "/api/v1/zones", nil)
			q := req.URL.Query()
			q.Add("lat", floatToString(lat))
			q.Add("lng", "0")
			req.URL.RawQuery = q.Encode()

			rec := httptest.NewRecorder()
			zoneHandler.HandleGetZone(rec, req)

			return rec.Code == http.StatusBadRequest
		},
		gen.OneGenOf(
			gen.Float64Range(-180, -90.01),
			gen.Float64Range(90.01, 180),
		),
	))

	// Test invalid longitudes (outside -180 to 180)
	properties.Property("invalid longitude returns 400", prop.ForAll(
		func(lng float64) bool {
			req := httptest.NewRequest("GET", "/api/v1/zones", nil)
			q := req.URL.Query()
			q.Add("lat", "0")
			q.Add("lng", floatToString(lng))
			req.URL.RawQuery = q.Encode()

			rec := httptest.NewRecorder()
			zoneHandler.HandleGetZone(rec, req)

			return rec.Code == http.StatusBadRequest
		},
		gen.OneGenOf(
			gen.Float64Range(-360, -180.01),
			gen.Float64Range(180.01, 360),
		),
	))

	properties.TestingRun(t)
}

// floatToString converts a float64 to a string.
func floatToString(f float64) string {
	return strconv.FormatFloat(f, 'f', -1, 64)
}
