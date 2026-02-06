// Package property contains property-based tests for the parking-fee-service.
package property

import (
	"bytes"
	"database/sql"
	"encoding/json"
	"math"
	"net/http"
	"net/http/httptest"
	"testing"
	"time"

	"github.com/google/uuid"
	"github.com/gorilla/mux"
	"github.com/leanovate/gopter"
	"github.com/leanovate/gopter/gen"
	"github.com/leanovate/gopter/prop"
	_ "modernc.org/sqlite"

	"github.com/sdv-parking-demo/backend/parking-fee-service/internal/handler"
	"github.com/sdv-parking-demo/backend/parking-fee-service/internal/model"
	"github.com/sdv-parking-demo/backend/parking-fee-service/internal/service"
	"github.com/sdv-parking-demo/backend/parking-fee-service/internal/store"
)

func setupParkingTest(t *testing.T) (*service.ParkingService, *handler.ParkingHandler, *mux.Router) {
	db, err := sql.Open("sqlite", ":memory:")
	if err != nil {
		t.Fatalf("failed to open test database: %v", err)
	}

	sessionStore := store.NewSessionStore(db)
	if err := sessionStore.InitSchema(); err != nil {
		t.Fatalf("failed to init schema: %v", err)
	}

	parkingService := service.NewParkingService(sessionStore, nil, 2.50)
	parkingHandler := handler.NewParkingHandler(parkingService, nil)

	router := mux.NewRouter()
	router.HandleFunc("/api/v1/parking/start", parkingHandler.HandleStartSession).Methods("POST")
	router.HandleFunc("/api/v1/parking/stop", parkingHandler.HandleStopSession).Methods("POST")
	router.HandleFunc("/api/v1/parking/status/{session_id}", parkingHandler.HandleGetStatus).Methods("GET")

	return parkingService, parkingHandler, router
}

// TestProperty8_SessionCreationRoundTrip verifies that created session is retrievable via status endpoint.
// Feature: parking-fee-service, Property 8: Session Creation Round-Trip
func TestProperty8_SessionCreationRoundTrip(t *testing.T) {
	parameters := gopter.DefaultTestParameters()
	parameters.MinSuccessfulTests = 100

	properties := gopter.NewProperties(parameters)

	properties.Property("created session is retrievable via status endpoint", prop.ForAll(
		func(vehicleID, zoneID string) bool {
			_, _, router := setupParkingTest(t)

			// Start session
			startReq := model.StartSessionRequest{
				VehicleID: vehicleID,
				ZoneID:    zoneID,
				Latitude:  37.5,
				Longitude: -122.5,
				Timestamp: time.Now().Format(time.RFC3339),
			}
			body, _ := json.Marshal(startReq)
			req := httptest.NewRequest("POST", "/api/v1/parking/start", bytes.NewReader(body))
			rec := httptest.NewRecorder()
			router.ServeHTTP(rec, req)

			if rec.Code != http.StatusOK {
				return false
			}

			var startResp model.StartSessionResponse
			if err := json.Unmarshal(rec.Body.Bytes(), &startResp); err != nil {
				return false
			}

			// Get status
			req = httptest.NewRequest("GET", "/api/v1/parking/status/"+startResp.SessionID, nil)
			rec = httptest.NewRecorder()
			router.ServeHTTP(rec, req)

			if rec.Code != http.StatusOK {
				return false
			}

			var status model.SessionStatus
			if err := json.Unmarshal(rec.Body.Bytes(), &status); err != nil {
				return false
			}

			return status.SessionID == startResp.SessionID
		},
		gen.AlphaString().SuchThat(func(s string) bool { return len(s) > 0 }),
		gen.AlphaString().SuchThat(func(s string) bool { return len(s) > 0 }),
	))

	properties.TestingRun(t)
}

// TestProperty9_SessionStartIdempotency verifies that duplicate start returns existing session.
// Feature: parking-fee-service, Property 9: Session Start Idempotency
func TestProperty9_SessionStartIdempotency(t *testing.T) {
	parameters := gopter.DefaultTestParameters()
	parameters.MinSuccessfulTests = 100

	properties := gopter.NewProperties(parameters)

	properties.Property("duplicate start returns existing session", prop.ForAll(
		func(vehicleID, zoneID string) bool {
			parkingService, _, _ := setupParkingTest(t)

			req := &model.StartSessionRequest{
				VehicleID: vehicleID,
				ZoneID:    zoneID,
				Latitude:  37.5,
				Longitude: -122.5,
				Timestamp: time.Now().Format(time.RFC3339),
			}

			session1, isExisting1, err1 := parkingService.StartSession(req)
			if err1 != nil {
				return false
			}

			session2, isExisting2, err2 := parkingService.StartSession(req)
			if err2 != nil {
				return false
			}

			return !isExisting1 && isExisting2 && session1.SessionID == session2.SessionID
		},
		gen.AlphaString().SuchThat(func(s string) bool { return len(s) > 0 }),
		gen.AlphaString().SuchThat(func(s string) bool { return len(s) > 0 }),
	))

	properties.TestingRun(t)
}

// TestProperty10_SessionStopResponseCompleteness verifies that stop response has all fields and end_time > start_time.
// Feature: parking-fee-service, Property 10: Session Stop Response Completeness
func TestProperty10_SessionStopResponseCompleteness(t *testing.T) {
	parameters := gopter.DefaultTestParameters()
	parameters.MinSuccessfulTests = 100

	properties := gopter.NewProperties(parameters)

	properties.Property("stop response has all fields, end_time > start_time", prop.ForAll(
		func(vehicleID, zoneID string) bool {
			_, _, router := setupParkingTest(t)

			// Start session
			startReq := model.StartSessionRequest{
				VehicleID: vehicleID,
				ZoneID:    zoneID,
				Latitude:  37.5,
				Longitude: -122.5,
				Timestamp: time.Now().Add(-time.Hour).Format(time.RFC3339),
			}
			body, _ := json.Marshal(startReq)
			req := httptest.NewRequest("POST", "/api/v1/parking/start", bytes.NewReader(body))
			rec := httptest.NewRecorder()
			router.ServeHTTP(rec, req)

			var startResp model.StartSessionResponse
			json.Unmarshal(rec.Body.Bytes(), &startResp)

			// Stop session
			stopReq := model.StopSessionRequest{
				SessionID: startResp.SessionID,
				Timestamp: time.Now().Format(time.RFC3339),
			}
			body, _ = json.Marshal(stopReq)
			req = httptest.NewRequest("POST", "/api/v1/parking/stop", bytes.NewReader(body))
			rec = httptest.NewRecorder()
			router.ServeHTTP(rec, req)

			if rec.Code != http.StatusOK {
				return false
			}

			var stopResp model.StopSessionResponse
			if err := json.Unmarshal(rec.Body.Bytes(), &stopResp); err != nil {
				return false
			}

			// Check all fields present
			if stopResp.SessionID == "" || stopResp.StartTime == "" || stopResp.EndTime == "" ||
				stopResp.PaymentStatus == "" {
				return false
			}

			// Check end_time > start_time
			startTime, _ := time.Parse(time.RFC3339, stopResp.StartTime)
			endTime, _ := time.Parse(time.RFC3339, stopResp.EndTime)

			return endTime.After(startTime)
		},
		gen.AlphaString().SuchThat(func(s string) bool { return len(s) > 0 }),
		gen.AlphaString().SuchThat(func(s string) bool { return len(s) > 0 }),
	))

	properties.TestingRun(t)
}

// TestProperty11_SessionStopIdempotency verifies that duplicate stop returns same result.
// Feature: parking-fee-service, Property 11: Session Stop Idempotency
func TestProperty11_SessionStopIdempotency(t *testing.T) {
	parameters := gopter.DefaultTestParameters()
	parameters.MinSuccessfulTests = 100

	properties := gopter.NewProperties(parameters)

	properties.Property("duplicate stop returns same result", prop.ForAll(
		func(vehicleID, zoneID string) bool {
			parkingService, _, _ := setupParkingTest(t)

			startReq := &model.StartSessionRequest{
				VehicleID: vehicleID,
				ZoneID:    zoneID,
				Timestamp: time.Now().Format(time.RFC3339),
			}
			session, _, _ := parkingService.StartSession(startReq)

			stopReq := &model.StopSessionRequest{
				SessionID: session.SessionID,
				Timestamp: time.Now().Add(time.Hour).Format(time.RFC3339),
			}

			result1, _ := parkingService.StopSession(stopReq)
			result2, _ := parkingService.StopSession(stopReq)

			return result1.TotalCost != nil && result2.TotalCost != nil &&
				*result1.TotalCost == *result2.TotalCost &&
				result1.EndTime.Equal(*result2.EndTime)
		},
		gen.AlphaString().SuchThat(func(s string) bool { return len(s) > 0 }),
		gen.AlphaString().SuchThat(func(s string) bool { return len(s) > 0 }),
	))

	properties.TestingRun(t)
}

// TestProperty12_CostCalculationCorrectness verifies cost = (duration/3600) * rate, rounded.
// Feature: parking-fee-service, Property 12: Cost Calculation Correctness
func TestProperty12_CostCalculationCorrectness(t *testing.T) {
	parameters := gopter.DefaultTestParameters()
	parameters.MinSuccessfulTests = 100

	properties := gopter.NewProperties(parameters)

	properties.Property("cost equals duration * rate / 3600", prop.ForAll(
		func(durationSeconds int64, hourlyRate float64) bool {
			parkingService := service.NewParkingService(nil, nil, hourlyRate)

			calculatedCost := parkingService.CalculateCost(durationSeconds)
			expectedCost := math.Round((float64(durationSeconds)/3600.0)*hourlyRate*100) / 100

			return math.Abs(calculatedCost-expectedCost) < 0.01
		},
		gen.Int64Range(0, 86400),
		gen.Float64Range(0.5, 50.0),
	))

	properties.TestingRun(t)
}

// TestProperty13_MockPaymentAlwaysSucceeds verifies that payment_status is always "success".
// Feature: parking-fee-service, Property 13: Mock Payment Always Succeeds
func TestProperty13_MockPaymentAlwaysSucceeds(t *testing.T) {
	parameters := gopter.DefaultTestParameters()
	parameters.MinSuccessfulTests = 100

	properties := gopter.NewProperties(parameters)

	properties.Property("payment_status always success", prop.ForAll(
		func(vehicleID, zoneID string) bool {
			parkingService, _, _ := setupParkingTest(t)

			startReq := &model.StartSessionRequest{
				VehicleID: vehicleID,
				ZoneID:    zoneID,
				Timestamp: time.Now().Format(time.RFC3339),
			}
			session, _, _ := parkingService.StartSession(startReq)

			stopReq := &model.StopSessionRequest{
				SessionID: session.SessionID,
				Timestamp: time.Now().Add(time.Hour).Format(time.RFC3339),
			}

			result, _ := parkingService.StopSession(stopReq)

			return result.PaymentStatus != nil && *result.PaymentStatus == "success"
		},
		gen.AlphaString().SuchThat(func(s string) bool { return len(s) > 0 }),
		gen.AlphaString().SuchThat(func(s string) bool { return len(s) > 0 }),
	))

	properties.TestingRun(t)
}

// TestProperty14_SessionNotFound verifies that invalid session_id returns 404 for stop and status.
// Feature: parking-fee-service, Property 14: Session Not Found
func TestProperty14_SessionNotFound(t *testing.T) {
	parameters := gopter.DefaultTestParameters()
	parameters.MinSuccessfulTests = 100

	properties := gopter.NewProperties(parameters)

	properties.Property("invalid session_id returns 404", prop.ForAll(
		func(_ int) bool {
			_, _, router := setupParkingTest(t)
			invalidID := uuid.New().String()

			// Stop should return 404
			stopReq := model.StopSessionRequest{
				SessionID: invalidID,
				Timestamp: time.Now().Format(time.RFC3339),
			}
			body, _ := json.Marshal(stopReq)
			req := httptest.NewRequest("POST", "/api/v1/parking/stop", bytes.NewReader(body))
			rec := httptest.NewRecorder()
			router.ServeHTTP(rec, req)

			if rec.Code != http.StatusNotFound {
				return false
			}

			// Status should return 404
			req = httptest.NewRequest("GET", "/api/v1/parking/status/"+invalidID, nil)
			rec = httptest.NewRecorder()
			router.ServeHTTP(rec, req)

			return rec.Code == http.StatusNotFound
		},
		gen.IntRange(0, 100),
	))

	properties.TestingRun(t)
}

// TestProperty15_SessionStatusConsistency verifies state is "active" or "stopped" appropriately.
// Feature: parking-fee-service, Property 15: Session Status Consistency
func TestProperty15_SessionStatusConsistency(t *testing.T) {
	parameters := gopter.DefaultTestParameters()
	parameters.MinSuccessfulTests = 100

	properties := gopter.NewProperties(parameters)

	properties.Property("state is active or stopped appropriately", prop.ForAll(
		func(vehicleID, zoneID string) bool {
			_, _, router := setupParkingTest(t)

			// Start session
			startReq := model.StartSessionRequest{
				VehicleID: vehicleID,
				ZoneID:    zoneID,
				Latitude:  37.5,
				Longitude: -122.5,
				Timestamp: time.Now().Format(time.RFC3339),
			}
			body, _ := json.Marshal(startReq)
			req := httptest.NewRequest("POST", "/api/v1/parking/start", bytes.NewReader(body))
			rec := httptest.NewRecorder()
			router.ServeHTTP(rec, req)

			var startResp model.StartSessionResponse
			json.Unmarshal(rec.Body.Bytes(), &startResp)

			// Check status is active
			req = httptest.NewRequest("GET", "/api/v1/parking/status/"+startResp.SessionID, nil)
			rec = httptest.NewRecorder()
			router.ServeHTTP(rec, req)

			var status model.SessionStatus
			json.Unmarshal(rec.Body.Bytes(), &status)

			if status.State != "active" {
				return false
			}

			// Stop session
			stopReq := model.StopSessionRequest{
				SessionID: startResp.SessionID,
				Timestamp: time.Now().Format(time.RFC3339),
			}
			body, _ = json.Marshal(stopReq)
			req = httptest.NewRequest("POST", "/api/v1/parking/stop", bytes.NewReader(body))
			rec = httptest.NewRecorder()
			router.ServeHTTP(rec, req)

			// Check status is stopped
			req = httptest.NewRequest("GET", "/api/v1/parking/status/"+startResp.SessionID, nil)
			rec = httptest.NewRecorder()
			router.ServeHTTP(rec, req)

			json.Unmarshal(rec.Body.Bytes(), &status)

			return status.State == "stopped"
		},
		gen.AlphaString().SuchThat(func(s string) bool { return len(s) > 0 }),
		gen.AlphaString().SuchThat(func(s string) bool { return len(s) > 0 }),
	))

	properties.TestingRun(t)
}
