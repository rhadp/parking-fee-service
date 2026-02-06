// Package property contains property-based tests for the parking-fee-service.
package property

import (
	"database/sql"
	"encoding/json"
	"net/http"
	"net/http/httptest"
	"testing"

	"github.com/leanovate/gopter"
	"github.com/leanovate/gopter/gen"
	"github.com/leanovate/gopter/prop"
	_ "modernc.org/sqlite"

	"github.com/sdv-parking-demo/backend/parking-fee-service/internal/handler"
	"github.com/sdv-parking-demo/backend/parking-fee-service/internal/model"
	"github.com/sdv-parking-demo/backend/parking-fee-service/internal/store"
)

// TestProperty16_ReadinessCheckDatabaseVerification verifies ready only when DB connection operational.
// Feature: parking-fee-service, Property 16: Readiness Check Database Verification
func TestProperty16_ReadinessCheckDatabaseVerification(t *testing.T) {
	parameters := gopter.DefaultTestParameters()
	parameters.MinSuccessfulTests = 100

	properties := gopter.NewProperties(parameters)

	properties.Property("ready only when DB connection operational", prop.ForAll(
		func(_ int) bool {
			// Test with valid DB - should return ready
			db, err := sql.Open("sqlite", ":memory:")
			if err != nil {
				return false
			}
			defer db.Close()

			sessionStore := store.NewSessionStore(db)
			if err := sessionStore.InitSchema(); err != nil {
				return false
			}

			healthHandler := handler.NewHealthHandler(sessionStore, nil)

			req := httptest.NewRequest("GET", "/ready", nil)
			rec := httptest.NewRecorder()
			healthHandler.HandleReady(rec, req)

			if rec.Code != http.StatusOK {
				return false
			}

			var response model.ReadyResponse
			if err := json.Unmarshal(rec.Body.Bytes(), &response); err != nil {
				return false
			}

			return response.Status == "ready"
		},
		gen.IntRange(0, 100),
	))

	properties.Property("not ready when session store is nil", prop.ForAll(
		func(_ int) bool {
			healthHandler := handler.NewHealthHandler(nil, nil)

			req := httptest.NewRequest("GET", "/ready", nil)
			rec := httptest.NewRecorder()
			healthHandler.HandleReady(rec, req)

			if rec.Code != http.StatusServiceUnavailable {
				return false
			}

			var response model.ReadyResponse
			if err := json.Unmarshal(rec.Body.Bytes(), &response); err != nil {
				return false
			}

			return response.Status == "not ready"
		},
		gen.IntRange(0, 100),
	))

	properties.TestingRun(t)
}
