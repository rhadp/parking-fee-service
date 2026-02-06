// Package property contains property-based tests for the parking-fee-service.
package property

import (
	"encoding/json"
	"net/http/httptest"
	"testing"

	"github.com/leanovate/gopter"
	"github.com/leanovate/gopter/gen"
	"github.com/leanovate/gopter/prop"

	"github.com/sdv-parking-demo/backend/parking-fee-service/internal/middleware"
	"github.com/sdv-parking-demo/backend/parking-fee-service/internal/model"
)

// TestProperty17_ErrorResponseFormatConsistency verifies all errors have error, message, request_id fields.
// Feature: parking-fee-service, Property 17: Error Response Format Consistency
func TestProperty17_ErrorResponseFormatConsistency(t *testing.T) {
	parameters := gopter.DefaultTestParameters()
	parameters.MinSuccessfulTests = 100

	properties := gopter.NewProperties(parameters)

	// Test that WriteError produces consistent format
	properties.Property("error responses have all required fields", prop.ForAll(
		func(code, message, requestID string) bool {
			response := model.ErrorResponse{
				Error:     code,
				Message:   message,
				RequestID: requestID,
			}

			data, err := json.Marshal(response)
			if err != nil {
				return false
			}

			var parsed map[string]interface{}
			if err := json.Unmarshal(data, &parsed); err != nil {
				return false
			}

			_, hasError := parsed["error"]
			_, hasMessage := parsed["message"]
			_, hasRequestID := parsed["request_id"]

			return hasError && hasMessage && hasRequestID
		},
		gen.AlphaString(),
		gen.AlphaString(),
		gen.AlphaString(),
	))

	// Test WriteError function produces valid JSON with correct fields
	properties.Property("WriteError produces valid error response", prop.ForAll(
		func(status int, code, message string) bool {
			// Create a request with request ID context
			req := httptest.NewRequest("GET", "/test", nil)
			ctx := req.Context()
			req = req.WithContext(ctx)

			rec := httptest.NewRecorder()
			middleware.WriteError(rec, req, status, code, message)

			if rec.Code != status {
				return false
			}

			var response model.ErrorResponse
			if err := json.Unmarshal(rec.Body.Bytes(), &response); err != nil {
				return false
			}

			return response.Error == code && response.Message == message
		},
		gen.IntRange(400, 599),
		gen.AlphaString().SuchThat(func(s string) bool { return len(s) > 0 }),
		gen.AlphaString().SuchThat(func(s string) bool { return len(s) > 0 }),
	))

	properties.TestingRun(t)
}
