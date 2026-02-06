package property

import (
	"bytes"
	"encoding/json"
	"log/slog"
	"net/http"
	"net/http/httptest"
	"os"
	"testing"
	"time"

	"github.com/gorilla/mux"
	"github.com/leanovate/gopter"
	"github.com/leanovate/gopter/gen"
	"github.com/leanovate/gopter/prop"

	"github.com/sdv-parking-demo/backend/cloud-gateway/internal/handler"
	"github.com/sdv-parking-demo/backend/cloud-gateway/internal/middleware"
	"github.com/sdv-parking-demo/backend/cloud-gateway/internal/model"
	"github.com/sdv-parking-demo/backend/cloud-gateway/internal/service"
	"github.com/sdv-parking-demo/backend/cloud-gateway/internal/store"
)

// genValidCommandType generates valid command types.
func genValidCommandType() gopter.Gen {
	return gen.OneConstOf("lock", "unlock")
}

// genInvalidCommandType generates invalid command types.
func genInvalidCommandType() gopter.Gen {
	return gen.AnyString().SuchThat(func(s string) bool {
		return s != "lock" && s != "unlock"
	})
}

// genValidDoors generates valid doors values.
func genValidDoors() gopter.Gen {
	return gen.OneConstOf([]string{"driver"}, []string{"all"}, []string{"driver", "all"})
}

// TestCommandTypeValidation tests Property 3: Command Type Validation.
func TestCommandTypeValidation(t *testing.T) {
	// Feature: cloud-gateway, Property 3: Command Type Validation
	// Validates: Requirements 2.6

	parameters := gopter.DefaultTestParameters()
	parameters.MinSuccessfulTests = 100

	properties := gopter.NewProperties(parameters)

	logger := slog.New(slog.NewJSONHandler(os.Stdout, &slog.HandlerOptions{Level: slog.LevelError}))
	mockMQTT := &mockMQTTClient{}
	cmdStore := store.NewCommandStore(100)
	cmdService := service.NewCommandService(cmdStore, mockMQTT, nil, logger, 30*time.Second, "TEST_VIN")
	cmdHandler := handler.NewCommandHandler(cmdService, nil, logger, "TEST_VIN")

	router := mux.NewRouter()
	router.HandleFunc("/api/v1/vehicles/{vin}/commands", cmdHandler.HandleSubmitCommand).Methods("POST")
	router.Use(middleware.RequestIDMiddleware)

	properties.Property("valid command types are accepted", prop.ForAll(
		func(commandType string) bool {
			body := map[string]interface{}{
				"command_type": commandType,
				"doors":        []string{"all"},
				"auth_token":   "valid-token",
			}
			jsonBody, _ := json.Marshal(body)

			req := httptest.NewRequest("POST", "/api/v1/vehicles/TEST_VIN/commands", bytes.NewReader(jsonBody))
			req.Header.Set("Content-Type", "application/json")
			w := httptest.NewRecorder()

			router.ServeHTTP(w, req)

			// Valid types should be accepted (202)
			return w.Code == http.StatusAccepted
		},
		genValidCommandType(),
	))

	properties.Property("invalid command types are rejected with INVALID_COMMAND_TYPE", prop.ForAll(
		func(commandType string) bool {
			body := map[string]interface{}{
				"command_type": commandType,
				"doors":        []string{"all"},
				"auth_token":   "valid-token",
			}
			jsonBody, _ := json.Marshal(body)

			req := httptest.NewRequest("POST", "/api/v1/vehicles/TEST_VIN/commands", bytes.NewReader(jsonBody))
			req.Header.Set("Content-Type", "application/json")
			w := httptest.NewRecorder()

			router.ServeHTTP(w, req)

			if w.Code != http.StatusBadRequest {
				return false
			}

			var resp model.ErrorResponse
			json.Unmarshal(w.Body.Bytes(), &resp)
			return resp.ErrorCode == model.ErrInvalidCommandType
		},
		genInvalidCommandType(),
	))

	properties.TestingRun(t)
}

// TestAuthTokenValidation tests Property 4: Auth Token Validation.
func TestAuthTokenValidation(t *testing.T) {
	// Feature: cloud-gateway, Property 4: Auth Token Validation
	// Validates: Requirements 2.7

	parameters := gopter.DefaultTestParameters()
	parameters.MinSuccessfulTests = 100

	properties := gopter.NewProperties(parameters)

	logger := slog.New(slog.NewJSONHandler(os.Stdout, &slog.HandlerOptions{Level: slog.LevelError}))
	mockMQTT := &mockMQTTClient{}
	cmdStore := store.NewCommandStore(100)
	cmdService := service.NewCommandService(cmdStore, mockMQTT, nil, logger, 30*time.Second, "TEST_VIN")
	cmdHandler := handler.NewCommandHandler(cmdService, nil, logger, "TEST_VIN")

	router := mux.NewRouter()
	router.HandleFunc("/api/v1/vehicles/{vin}/commands", cmdHandler.HandleSubmitCommand).Methods("POST")
	router.Use(middleware.RequestIDMiddleware)

	properties.Property("missing auth token is rejected with MISSING_AUTH_TOKEN", prop.ForAll(
		func() bool {
			body := map[string]interface{}{
				"command_type": "lock",
				"doors":        []string{"all"},
				// No auth_token
			}
			jsonBody, _ := json.Marshal(body)

			req := httptest.NewRequest("POST", "/api/v1/vehicles/TEST_VIN/commands", bytes.NewReader(jsonBody))
			req.Header.Set("Content-Type", "application/json")
			w := httptest.NewRecorder()

			router.ServeHTTP(w, req)

			if w.Code != http.StatusBadRequest {
				return false
			}

			var resp model.ErrorResponse
			json.Unmarshal(w.Body.Bytes(), &resp)
			return resp.ErrorCode == model.ErrMissingAuthToken
		},
	))

	properties.Property("non-empty auth token is accepted", prop.ForAll(
		func(token string) bool {
			if token == "" {
				return true // Skip empty tokens
			}

			body := map[string]interface{}{
				"command_type": "lock",
				"doors":        []string{"all"},
				"auth_token":   token,
			}
			jsonBody, _ := json.Marshal(body)

			req := httptest.NewRequest("POST", "/api/v1/vehicles/TEST_VIN/commands", bytes.NewReader(jsonBody))
			req.Header.Set("Content-Type", "application/json")
			w := httptest.NewRecorder()

			router.ServeHTTP(w, req)

			// Non-empty auth token should be accepted (202)
			return w.Code == http.StatusAccepted
		},
		gen.AlphaString().SuchThat(func(s string) bool { return len(s) > 0 }),
	))

	properties.TestingRun(t)
}

// TestCommandStatusResponseCompleteness tests Property 5: Command Status Response Completeness.
func TestCommandStatusResponseCompleteness(t *testing.T) {
	// Feature: cloud-gateway, Property 5: Command Status Response Completeness
	// Validates: Requirements 3.2, 3.3, 3.4

	parameters := gopter.DefaultTestParameters()
	parameters.MinSuccessfulTests = 100

	properties := gopter.NewProperties(parameters)

	logger := slog.New(slog.NewJSONHandler(os.Stdout, &slog.HandlerOptions{Level: slog.LevelError}))
	mockMQTT := &mockMQTTClient{}

	properties.Property("command status response contains all required fields", prop.ForAll(
		func(commandType string, doors []string) bool {
			cmdStore := store.NewCommandStore(100)
			cmdService := service.NewCommandService(cmdStore, mockMQTT, nil, logger, 30*time.Second, "TEST_VIN")
			cmdHandler := handler.NewCommandHandler(cmdService, nil, logger, "TEST_VIN")

			router := mux.NewRouter()
			router.HandleFunc("/api/v1/vehicles/{vin}/commands", cmdHandler.HandleSubmitCommand).Methods("POST")
			router.HandleFunc("/api/v1/vehicles/{vin}/commands/{command_id}", cmdHandler.HandleGetCommandStatus).Methods("GET")
			router.Use(middleware.RequestIDMiddleware)

			// First create a command
			body := map[string]interface{}{
				"command_type": commandType,
				"doors":        doors,
				"auth_token":   "valid-token",
			}
			jsonBody, _ := json.Marshal(body)

			req := httptest.NewRequest("POST", "/api/v1/vehicles/TEST_VIN/commands", bytes.NewReader(jsonBody))
			req.Header.Set("Content-Type", "application/json")
			w := httptest.NewRecorder()
			router.ServeHTTP(w, req)

			if w.Code != http.StatusAccepted {
				return false
			}

			var submitResp model.SubmitCommandResponse
			json.Unmarshal(w.Body.Bytes(), &submitResp)

			// Now get the command status
			req = httptest.NewRequest("GET", "/api/v1/vehicles/TEST_VIN/commands/"+submitResp.CommandID, nil)
			w = httptest.NewRecorder()
			router.ServeHTTP(w, req)

			if w.Code != http.StatusOK {
				return false
			}

			var statusResp model.CommandStatusResponse
			json.Unmarshal(w.Body.Bytes(), &statusResp)

			// Verify required fields
			return statusResp.CommandID != "" &&
				statusResp.CommandType != "" &&
				statusResp.Status != "" &&
				statusResp.CreatedAt != "" &&
				statusResp.RequestID != ""
		},
		genValidCommandType(),
		genValidDoors(),
	))

	properties.TestingRun(t)
}

// TestCommandNotFound tests Property 6: Command Not Found.
func TestCommandNotFound(t *testing.T) {
	// Feature: cloud-gateway, Property 6: Command Not Found
	// Validates: Requirements 3.5

	parameters := gopter.DefaultTestParameters()
	parameters.MinSuccessfulTests = 100

	properties := gopter.NewProperties(parameters)

	logger := slog.New(slog.NewJSONHandler(os.Stdout, &slog.HandlerOptions{Level: slog.LevelError}))
	mockMQTT := &mockMQTTClient{}
	cmdStore := store.NewCommandStore(100)
	cmdService := service.NewCommandService(cmdStore, mockMQTT, nil, logger, 30*time.Second, "TEST_VIN")
	cmdHandler := handler.NewCommandHandler(cmdService, nil, logger, "TEST_VIN")

	router := mux.NewRouter()
	router.HandleFunc("/api/v1/vehicles/{vin}/commands/{command_id}", cmdHandler.HandleGetCommandStatus).Methods("GET")
	router.Use(middleware.RequestIDMiddleware)

	properties.Property("non-existent command returns 404 with COMMAND_NOT_FOUND", prop.ForAll(
		func(commandID string) bool {
			req := httptest.NewRequest("GET", "/api/v1/vehicles/TEST_VIN/commands/"+commandID, nil)
			w := httptest.NewRecorder()

			router.ServeHTTP(w, req)

			if w.Code != http.StatusNotFound {
				return false
			}

			var resp model.ErrorResponse
			json.Unmarshal(w.Body.Bytes(), &resp)
			return resp.ErrorCode == model.ErrCommandNotFound
		},
		gen.AlphaString().SuchThat(func(s string) bool { return len(s) > 0 }),
	))

	properties.TestingRun(t)
}

// TestVINValidationAcrossEndpoints tests Property 1: VIN Validation Across Endpoints.
func TestVINValidationAcrossEndpoints(t *testing.T) {
	// Feature: cloud-gateway, Property 1: VIN Validation Across Endpoints
	// Validates: Requirements 2.8, 3.6, 16.4

	parameters := gopter.DefaultTestParameters()
	parameters.MinSuccessfulTests = 100

	properties := gopter.NewProperties(parameters)

	logger := slog.New(slog.NewJSONHandler(os.Stdout, &slog.HandlerOptions{Level: slog.LevelError}))
	mockMQTT := &mockMQTTClient{}
	cmdStore := store.NewCommandStore(100)
	cmdService := service.NewCommandService(cmdStore, mockMQTT, nil, logger, 30*time.Second, "CONFIGURED_VIN")
	cmdHandler := handler.NewCommandHandler(cmdService, nil, logger, "CONFIGURED_VIN")
	parkingSessionService := service.NewParkingSessionService("http://localhost:8081", logger, "CONFIGURED_VIN")
	parkingHandler := handler.NewParkingSessionHandler(parkingSessionService, logger, "CONFIGURED_VIN")

	router := mux.NewRouter()
	router.HandleFunc("/api/v1/vehicles/{vin}/commands", cmdHandler.HandleSubmitCommand).Methods("POST")
	router.HandleFunc("/api/v1/vehicles/{vin}/commands/{command_id}", cmdHandler.HandleGetCommandStatus).Methods("GET")
	router.HandleFunc("/api/v1/vehicles/{vin}/parking-session", parkingHandler.HandleGetParkingSession).Methods("GET")
	router.Use(middleware.RequestIDMiddleware)

	properties.Property("invalid VIN returns 404 VEHICLE_NOT_FOUND on submit command", prop.ForAll(
		func(vin string) bool {
			if vin == "CONFIGURED_VIN" {
				return true // Skip the configured VIN
			}

			body := map[string]interface{}{
				"command_type": "lock",
				"doors":        []string{"all"},
				"auth_token":   "token",
			}
			jsonBody, _ := json.Marshal(body)

			req := httptest.NewRequest("POST", "/api/v1/vehicles/"+vin+"/commands", bytes.NewReader(jsonBody))
			req.Header.Set("Content-Type", "application/json")
			w := httptest.NewRecorder()

			router.ServeHTTP(w, req)

			if w.Code != http.StatusNotFound {
				return false
			}

			var resp model.ErrorResponse
			json.Unmarshal(w.Body.Bytes(), &resp)
			return resp.ErrorCode == model.ErrVehicleNotFound
		},
		gen.AlphaString().SuchThat(func(s string) bool { return len(s) > 0 && s != "CONFIGURED_VIN" }),
	))

	properties.Property("invalid VIN returns 404 VEHICLE_NOT_FOUND on get command status", prop.ForAll(
		func(vin string) bool {
			if vin == "CONFIGURED_VIN" {
				return true // Skip the configured VIN
			}

			req := httptest.NewRequest("GET", "/api/v1/vehicles/"+vin+"/commands/some-id", nil)
			w := httptest.NewRecorder()

			router.ServeHTTP(w, req)

			if w.Code != http.StatusNotFound {
				return false
			}

			var resp model.ErrorResponse
			json.Unmarshal(w.Body.Bytes(), &resp)
			return resp.ErrorCode == model.ErrVehicleNotFound
		},
		gen.AlphaString().SuchThat(func(s string) bool { return len(s) > 0 && s != "CONFIGURED_VIN" }),
	))

	properties.Property("invalid VIN returns 404 VEHICLE_NOT_FOUND on parking session", prop.ForAll(
		func(vin string) bool {
			if vin == "CONFIGURED_VIN" {
				return true // Skip the configured VIN
			}

			req := httptest.NewRequest("GET", "/api/v1/vehicles/"+vin+"/parking-session", nil)
			w := httptest.NewRecorder()

			router.ServeHTTP(w, req)

			if w.Code != http.StatusNotFound {
				return false
			}

			var resp model.ErrorResponse
			json.Unmarshal(w.Body.Bytes(), &resp)
			return resp.ErrorCode == model.ErrVehicleNotFound
		},
		gen.AlphaString().SuchThat(func(s string) bool { return len(s) > 0 && s != "CONFIGURED_VIN" }),
	))

	properties.TestingRun(t)
}

// TestErrorResponseFormatConsistency tests Property 11: Error Response Format Consistency.
func TestErrorResponseFormatConsistency(t *testing.T) {
	// Feature: cloud-gateway, Property 11: Error Response Format Consistency
	// Validates: Requirements 11.1, 11.2, 11.3

	parameters := gopter.DefaultTestParameters()
	parameters.MinSuccessfulTests = 100

	properties := gopter.NewProperties(parameters)

	logger := slog.New(slog.NewJSONHandler(os.Stdout, &slog.HandlerOptions{Level: slog.LevelError}))
	mockMQTT := &mockMQTTClient{}
	cmdStore := store.NewCommandStore(100)
	cmdService := service.NewCommandService(cmdStore, mockMQTT, nil, logger, 30*time.Second, "TEST_VIN")
	cmdHandler := handler.NewCommandHandler(cmdService, nil, logger, "TEST_VIN")

	router := mux.NewRouter()
	router.HandleFunc("/api/v1/vehicles/{vin}/commands", cmdHandler.HandleSubmitCommand).Methods("POST")
	router.HandleFunc("/api/v1/vehicles/{vin}/commands/{command_id}", cmdHandler.HandleGetCommandStatus).Methods("GET")
	router.Use(middleware.RequestIDMiddleware)

	properties.Property("all error responses have error_code, message, and request_id", prop.ForAll(
		func() bool {
			// Test various error scenarios
			errorCases := []struct {
				method string
				path   string
				body   string
			}{
				// Invalid VIN
				{"POST", "/api/v1/vehicles/WRONG_VIN/commands", `{"command_type":"lock","doors":["all"],"auth_token":"t"}`},
				// Invalid command type
				{"POST", "/api/v1/vehicles/TEST_VIN/commands", `{"command_type":"invalid","doors":["all"],"auth_token":"t"}`},
				// Missing auth token
				{"POST", "/api/v1/vehicles/TEST_VIN/commands", `{"command_type":"lock","doors":["all"]}`},
				// Command not found
				{"GET", "/api/v1/vehicles/TEST_VIN/commands/nonexistent", ""},
			}

			for _, tc := range errorCases {
				var req *http.Request
				if tc.body != "" {
					req = httptest.NewRequest(tc.method, tc.path, bytes.NewReader([]byte(tc.body)))
					req.Header.Set("Content-Type", "application/json")
				} else {
					req = httptest.NewRequest(tc.method, tc.path, nil)
				}
				w := httptest.NewRecorder()

				router.ServeHTTP(w, req)

				// Should be an error response
				if w.Code >= 200 && w.Code < 300 {
					continue // Not an error, skip
				}

				var resp model.ErrorResponse
				if err := json.Unmarshal(w.Body.Bytes(), &resp); err != nil {
					return false
				}

				// All error responses must have these fields
				if resp.ErrorCode == "" || resp.Message == "" || resp.RequestID == "" {
					return false
				}
			}

			return true
		},
	))

	properties.TestingRun(t)
}

// TestInterfaceIndependence tests Property 18: Interface Independence.
func TestInterfaceIndependence(t *testing.T) {
	// Feature: cloud-gateway, Property 18: Interface Independence
	// Validates: Requirements 15.7, 15.8

	parameters := gopter.DefaultTestParameters()
	parameters.MinSuccessfulTests = 100

	properties := gopter.NewProperties(parameters)

	properties.Property("REST endpoints work independently of MQTT status", prop.ForAll(
		func(mqttConnected bool) bool {
			mockMQTT := &mockMQTTClient{connected: mqttConnected}

			healthHandler := handler.NewHealthHandler(mockMQTT, "cloud-gateway")

			router := mux.NewRouter()
			router.HandleFunc("/health", healthHandler.HandleHealth).Methods("GET")
			router.HandleFunc("/ready", healthHandler.HandleReady).Methods("GET")

			// Health endpoint always works
			req := httptest.NewRequest("GET", "/health", nil)
			w := httptest.NewRecorder()
			router.ServeHTTP(w, req)

			if w.Code != http.StatusOK {
				return false
			}

			var healthResp model.HealthResponse
			json.Unmarshal(w.Body.Bytes(), &healthResp)
			if healthResp.Status != "healthy" {
				return false
			}

			// Ready endpoint reflects MQTT status
			req = httptest.NewRequest("GET", "/ready", nil)
			w = httptest.NewRecorder()
			router.ServeHTTP(w, req)

			var readyResp model.ReadyResponse
			json.Unmarshal(w.Body.Bytes(), &readyResp)

			if mqttConnected {
				return w.Code == http.StatusOK && readyResp.Status == "ready" && readyResp.MQTTConnected
			}
			return w.Code == http.StatusServiceUnavailable && readyResp.Status == "not_ready" && !readyResp.MQTTConnected
		},
		gen.Bool(),
	))

	properties.TestingRun(t)
}
