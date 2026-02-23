//go:build integration

// Integration tests for the CLOUD_GATEWAY service.
//
// These tests require a running Mosquitto broker on localhost:1883 (provided by
// `make infra-up`). They verify the full REST-MQTT-REST cycle including command
// correlation, multi-vehicle routing, and telemetry subscription.
//
// Run with: go test -v -count=1 -tags integration ./...
package main

import (
	"context"
	"encoding/json"
	"fmt"
	"io"
	"net"
	"net/http"
	"strings"
	"testing"
	"time"

	pahomqtt "github.com/eclipse/paho.mqtt.golang"

	"github.com/rhadp/parking-fee-service/backend/cloud-gateway/internal/api"
	"github.com/rhadp/parking-fee-service/backend/cloud-gateway/internal/bridge"
	"github.com/rhadp/parking-fee-service/backend/cloud-gateway/internal/mqtt"
)

const (
	integrationBrokerURL = "tcp://localhost:1883"
	integrationAuthToken = "demo-token"
)

// skipIfNoMosquitto skips the test if the MQTT broker is not reachable.
func skipIfNoMosquitto(t *testing.T) {
	t.Helper()
	conn, err := net.DialTimeout("tcp", "localhost:1883", 1*time.Second)
	if err != nil {
		t.Skip("Mosquitto MQTT broker not running on localhost:1883; skipping integration test")
	}
	conn.Close()
}

// freePort returns an available TCP port.
func freePort(t *testing.T) int {
	t.Helper()
	l, err := net.Listen("tcp", "localhost:0")
	if err != nil {
		t.Fatalf("could not find free port: %v", err)
	}
	port := l.Addr().(*net.TCPAddr).Port
	l.Close()
	return port
}

// startGateway creates and starts an in-process CLOUD_GATEWAY connected to the
// local Mosquitto broker. It returns the base URL and a cleanup function.
func startGateway(t *testing.T, commandTimeout time.Duration) (baseURL string, mqttClient *mqtt.Client) {
	t.Helper()

	port := freePort(t)

	// Initialize MQTT client
	clientID := fmt.Sprintf("integration-test-%d", port)
	mqttClient = mqtt.NewClient(integrationBrokerURL, clientID)

	// Initialize command tracker, telemetry cache, and bridge
	tracker := bridge.NewTracker(commandTimeout)
	cache := api.NewTelemetryCache()
	b := bridge.NewBridge(tracker, mqttClient)

	// Register MQTT subscription handlers
	mqttClient.Subscribe(mqtt.WildcardResponseTopic(), func(topic string, payload []byte) {
		b.HandleResponse(payload)
	})
	mqttClient.Subscribe(mqtt.WildcardTelemetryTopic(), func(topic string, payload []byte) {
		vin, ok := mqtt.ExtractVINFromTopic(topic)
		if !ok {
			return
		}
		var data struct {
			VIN       string `json:"vin"`
			Locked    bool   `json:"locked"`
			Timestamp int64  `json:"timestamp"`
		}
		if err := json.Unmarshal(payload, &data); err != nil {
			return
		}
		cache.Update(vin, api.TelemetryData{
			VIN:       vin,
			Locked:    data.Locked,
			Timestamp: data.Timestamp,
		})
	})

	// Connect MQTT
	if err := mqttClient.Connect(); err != nil {
		t.Fatalf("MQTT connect failed: %v", err)
	}

	// Wait for connection
	deadline := time.Now().Add(5 * time.Second)
	for !mqttClient.IsConnected() && time.Now().Before(deadline) {
		time.Sleep(100 * time.Millisecond)
	}
	if !mqttClient.IsConnected() {
		t.Fatal("MQTT client did not connect within 5 seconds")
	}

	// Create HTTP router and start server
	router := api.NewRouter(integrationAuthToken, tracker, mqttClient, cache)
	addr := fmt.Sprintf(":%d", port)
	server := &http.Server{
		Addr:    addr,
		Handler: router,
	}

	go func() {
		if err := server.ListenAndServe(); err != nil && err != http.ErrServerClosed {
			// Test may have ended already, don't fatal
			return
		}
	}()

	// Wait for HTTP server to start
	deadline = time.Now().Add(5 * time.Second)
	for time.Now().Before(deadline) {
		conn, err := net.DialTimeout("tcp", fmt.Sprintf("localhost:%d", port), 500*time.Millisecond)
		if err == nil {
			conn.Close()
			break
		}
		time.Sleep(100 * time.Millisecond)
	}

	t.Cleanup(func() {
		ctx, cancel := context.WithTimeout(context.Background(), 2*time.Second)
		defer cancel()
		server.Shutdown(ctx)
		mqttClient.Disconnect()
	})

	baseURL = fmt.Sprintf("http://localhost:%d", port)
	return baseURL, mqttClient
}

// newTestMQTTClient creates a separate MQTT client for simulating the vehicle
// side (CLOUD_GATEWAY_CLIENT). It connects to the broker and returns the client.
func newTestMQTTClient(t *testing.T, clientID string) pahomqtt.Client {
	t.Helper()

	opts := pahomqtt.NewClientOptions().
		AddBroker(integrationBrokerURL).
		SetClientID(clientID).
		SetAutoReconnect(false).
		SetOrderMatters(false)

	client := pahomqtt.NewClient(opts)
	token := client.Connect()
	if !token.WaitTimeout(5 * time.Second) {
		t.Fatal("test MQTT client connect timed out")
	}
	if token.Error() != nil {
		t.Fatalf("test MQTT client connect error: %v", token.Error())
	}

	t.Cleanup(func() {
		client.Disconnect(500)
	})

	return client
}

// httpPostJSONInteg sends a POST request with JSON body and auth token.
func httpPostJSONInteg(t *testing.T, url, body, token string, timeout time.Duration) (int, string) {
	t.Helper()

	ctx, cancel := context.WithTimeout(context.Background(), timeout)
	defer cancel()

	req, err := http.NewRequestWithContext(ctx, http.MethodPost, url, strings.NewReader(body))
	if err != nil {
		t.Fatalf("create request: %v", err)
	}
	req.Header.Set("Content-Type", "application/json")
	if token != "" {
		req.Header.Set("Authorization", "Bearer "+token)
	}

	resp, err := http.DefaultClient.Do(req)
	if err != nil {
		t.Fatalf("HTTP POST failed: %v", err)
	}
	defer resp.Body.Close()

	data, err := io.ReadAll(resp.Body)
	if err != nil {
		t.Fatalf("read response body: %v", err)
	}

	return resp.StatusCode, string(data)
}

// httpGetInteg sends a GET request with auth token.
func httpGetInteg(t *testing.T, url, token string, timeout time.Duration) (int, string) {
	t.Helper()

	ctx, cancel := context.WithTimeout(context.Background(), timeout)
	defer cancel()

	req, err := http.NewRequestWithContext(ctx, http.MethodGet, url, nil)
	if err != nil {
		t.Fatalf("create request: %v", err)
	}
	if token != "" {
		req.Header.Set("Authorization", "Bearer "+token)
	}

	resp, err := http.DefaultClient.Do(req)
	if err != nil {
		t.Fatalf("HTTP GET failed: %v", err)
	}
	defer resp.Body.Close()

	data, err := io.ReadAll(resp.Body)
	if err != nil {
		t.Fatalf("read response body: %v", err)
	}

	return resp.StatusCode, string(data)
}

// TestIntegration_EndToEnd verifies the full request-response cycle:
// HTTP POST -> MQTT publish -> simulated subscriber -> MQTT response -> HTTP response.
// Requirement: 03-REQ-6.1
func TestIntegration_EndToEnd(t *testing.T) {
	skipIfNoMosquitto(t)

	baseURL, _ := startGateway(t, 15*time.Second)

	// Create a simulated vehicle responder
	simClient := newTestMQTTClient(t, "sim-e2e")

	// Subscribe to commands and auto-respond with success
	cmdReceived := make(chan string, 1)
	token := simClient.Subscribe("vehicles/VIN12345/commands", 1,
		func(_ pahomqtt.Client, msg pahomqtt.Message) {
			var cmd map[string]interface{}
			if err := json.Unmarshal(msg.Payload(), &cmd); err != nil {
				return
			}
			cmdID, _ := cmd["command_id"].(string)
			cmdReceived <- cmdID

			// Publish success response
			resp := fmt.Sprintf(`{"command_id":"%s","status":"success","reason":"","timestamp":%d}`,
				cmdID, time.Now().Unix())
			simClient.Publish("vehicles/VIN12345/command_responses", 1, false, []byte(resp))
		})
	token.Wait()
	if token.Error() != nil {
		t.Fatalf("subscribe failed: %v", token.Error())
	}

	// Give time for subscription to propagate
	time.Sleep(500 * time.Millisecond)

	// Send a command via REST
	body := `{"command_id":"e2e-int-001","type":"lock","doors":["driver"]}`
	statusCode, respBody := httpPostJSONInteg(t, baseURL+"/vehicles/VIN12345/commands",
		body, integrationAuthToken, 20*time.Second)

	// Verify the command was received by the simulated subscriber
	select {
	case receivedID := <-cmdReceived:
		if receivedID != "e2e-int-001" {
			t.Errorf("simulated subscriber received command_id %q, expected 'e2e-int-001'", receivedID)
		}
	case <-time.After(5 * time.Second):
		t.Error("simulated subscriber did not receive the command within 5 seconds")
	}

	// Verify the REST response
	if statusCode != 200 {
		t.Errorf("expected HTTP 200, got %d; body: %s", statusCode, respBody)
	}

	var resp map[string]interface{}
	if err := json.Unmarshal([]byte(respBody), &resp); err != nil {
		t.Fatalf("response is not valid JSON: %v; body: %s", err, respBody)
	}
	if resp["command_id"] != "e2e-int-001" {
		t.Errorf("expected command_id 'e2e-int-001', got %v", resp["command_id"])
	}
	if resp["status"] != "success" {
		t.Errorf("expected status 'success', got %v", resp["status"])
	}
}

// TestIntegration_CommandCorrelation verifies command_id is preserved through
// the entire REST -> MQTT -> REST cycle.
// Requirement: 03-REQ-6.3
func TestIntegration_CommandCorrelation(t *testing.T) {
	skipIfNoMosquitto(t)

	baseURL, _ := startGateway(t, 15*time.Second)

	// Set up simulated responder that echoes command_id
	simClient := newTestMQTTClient(t, "sim-corr")
	token := simClient.Subscribe("vehicles/+/commands", 1,
		func(_ pahomqtt.Client, msg pahomqtt.Message) {
			var cmd map[string]interface{}
			if err := json.Unmarshal(msg.Payload(), &cmd); err != nil {
				return
			}
			cmdID, _ := cmd["command_id"].(string)

			// Extract VIN from topic
			parts := strings.Split(msg.Topic(), "/")
			if len(parts) < 2 {
				return
			}
			vin := parts[1]

			resp := fmt.Sprintf(`{"command_id":"%s","status":"success","reason":"","timestamp":%d}`,
				cmdID, time.Now().Unix())
			simClient.Publish(fmt.Sprintf("vehicles/%s/command_responses", vin), 1, false, []byte(resp))
		})
	token.Wait()
	time.Sleep(500 * time.Millisecond)

	// Send multiple commands with different IDs and verify correlation
	commandIDs := []string{
		"corr-test-aaa-111",
		"corr-test-bbb-222",
		"corr-test-ccc-333",
	}

	for _, cmdID := range commandIDs {
		body := fmt.Sprintf(`{"command_id":"%s","type":"unlock","doors":["driver"]}`, cmdID)
		statusCode, respBody := httpPostJSONInteg(t, baseURL+"/vehicles/VIN12345/commands",
			body, integrationAuthToken, 20*time.Second)

		if statusCode != 200 {
			t.Errorf("command %s: expected HTTP 200, got %d; body: %s", cmdID, statusCode, respBody)
			continue
		}

		var resp map[string]interface{}
		if err := json.Unmarshal([]byte(respBody), &resp); err != nil {
			t.Errorf("command %s: response is not valid JSON: %v", cmdID, err)
			continue
		}

		if resp["command_id"] != cmdID {
			t.Errorf("command correlation failed: sent %q, received %v", cmdID, resp["command_id"])
		}
	}
}

// TestIntegration_MultiVehicleRouting verifies commands for different VINs are
// routed to the correct MQTT topics.
// Requirement: 03-REQ-5.1
func TestIntegration_MultiVehicleRouting(t *testing.T) {
	skipIfNoMosquitto(t)

	baseURL, _ := startGateway(t, 15*time.Second)

	// Create a responder that subscribes to both VINs and tracks which
	// command_ids arrive on which topics.
	simClient := newTestMQTTClient(t, "sim-multi")

	type receivedCmd struct {
		VIN       string
		CommandID string
		Action    string
	}
	cmdChan := make(chan receivedCmd, 10)

	token := simClient.Subscribe("vehicles/+/commands", 1,
		func(_ pahomqtt.Client, msg pahomqtt.Message) {
			var cmd map[string]interface{}
			if err := json.Unmarshal(msg.Payload(), &cmd); err != nil {
				return
			}

			parts := strings.Split(msg.Topic(), "/")
			if len(parts) < 2 {
				return
			}
			vin := parts[1]
			cmdID, _ := cmd["command_id"].(string)
			action, _ := cmd["action"].(string)

			cmdChan <- receivedCmd{VIN: vin, CommandID: cmdID, Action: action}

			// Respond with success
			resp := fmt.Sprintf(`{"command_id":"%s","status":"success","reason":"","timestamp":%d}`,
				cmdID, time.Now().Unix())
			simClient.Publish(fmt.Sprintf("vehicles/%s/command_responses", vin), 1, false, []byte(resp))
		})
	token.Wait()
	time.Sleep(500 * time.Millisecond)

	// Send commands for two different VINs
	bodyA := `{"command_id":"multi-A","type":"lock","doors":["driver"]}`
	bodyB := `{"command_id":"multi-B","type":"unlock","doors":["driver"]}`

	// Send both commands (sequentially to avoid response confusion)
	statusA, _ := httpPostJSONInteg(t, baseURL+"/vehicles/VIN_ALPHA/commands",
		bodyA, integrationAuthToken, 20*time.Second)
	if statusA != 200 {
		t.Errorf("VIN_ALPHA: expected HTTP 200, got %d", statusA)
	}

	statusB, _ := httpPostJSONInteg(t, baseURL+"/vehicles/VIN_BETA/commands",
		bodyB, integrationAuthToken, 20*time.Second)
	if statusB != 200 {
		t.Errorf("VIN_BETA: expected HTTP 200, got %d", statusB)
	}

	// Collect received commands
	received := make(map[string]receivedCmd)
	timeout := time.After(5 * time.Second)
	for len(received) < 2 {
		select {
		case cmd := <-cmdChan:
			received[cmd.CommandID] = cmd
		case <-timeout:
			t.Fatalf("timed out waiting for commands; received %d of 2", len(received))
		}
	}

	// Verify routing
	if cmdA, ok := received["multi-A"]; ok {
		if cmdA.VIN != "VIN_ALPHA" {
			t.Errorf("multi-A: expected VIN 'VIN_ALPHA', got %q", cmdA.VIN)
		}
		if cmdA.Action != "lock" {
			t.Errorf("multi-A: expected action 'lock', got %q", cmdA.Action)
		}
	} else {
		t.Error("did not receive command multi-A")
	}

	if cmdB, ok := received["multi-B"]; ok {
		if cmdB.VIN != "VIN_BETA" {
			t.Errorf("multi-B: expected VIN 'VIN_BETA', got %q", cmdB.VIN)
		}
		if cmdB.Action != "unlock" {
			t.Errorf("multi-B: expected action 'unlock', got %q", cmdB.Action)
		}
	} else {
		t.Error("did not receive command multi-B")
	}
}

// TestIntegration_TelemetrySubscription verifies CLOUD_GATEWAY subscribes to
// the telemetry topic and caches data for the status endpoint.
// Requirement: 03-REQ-2.4
func TestIntegration_TelemetrySubscription(t *testing.T) {
	skipIfNoMosquitto(t)

	baseURL, _ := startGateway(t, 15*time.Second)

	// Publish telemetry from a test client (simulating vehicle)
	simClient := newTestMQTTClient(t, "sim-telem")
	telemetry := `{"vin":"VIN_TELEM","locked":true,"timestamp":1708700000}`
	pubToken := simClient.Publish("vehicles/VIN_TELEM/telemetry", 0, false, []byte(telemetry))
	pubToken.Wait()

	// Wait for telemetry to be processed
	time.Sleep(1 * time.Second)

	// Query status endpoint
	statusCode, respBody := httpGetInteg(t, baseURL+"/vehicles/VIN_TELEM/status",
		integrationAuthToken, 5*time.Second)

	if statusCode != 200 {
		t.Errorf("expected HTTP 200, got %d; body: %s", statusCode, respBody)
	}

	var resp map[string]interface{}
	if err := json.Unmarshal([]byte(respBody), &resp); err != nil {
		t.Fatalf("response is not valid JSON: %v; body: %s", err, respBody)
	}

	if resp["vin"] != "VIN_TELEM" {
		t.Errorf("expected vin 'VIN_TELEM', got %v", resp["vin"])
	}
	if resp["locked"] != true {
		t.Errorf("expected locked=true, got %v", resp["locked"])
	}
}

// TestIntegration_CommandTimeout verifies that a command without a response
// times out correctly with HTTP 504.
// Requirement: 03-REQ-2.E3
func TestIntegration_CommandTimeout(t *testing.T) {
	skipIfNoMosquitto(t)

	// Use a short timeout for the test
	baseURL, _ := startGateway(t, 2*time.Second)

	// Send a command but don't set up any responder
	body := `{"command_id":"timeout-int-001","type":"lock","doors":["driver"]}`
	statusCode, respBody := httpPostJSONInteg(t, baseURL+"/vehicles/VIN_NONE/commands",
		body, integrationAuthToken, 10*time.Second)

	if statusCode != 504 {
		t.Errorf("expected HTTP 504 (gateway timeout), got %d; body: %s", statusCode, respBody)
	}

	var resp map[string]interface{}
	if err := json.Unmarshal([]byte(respBody), &resp); err != nil {
		t.Fatalf("response is not valid JSON: %v; body: %s", err, respBody)
	}

	if resp["command_id"] != "timeout-int-001" {
		t.Errorf("expected command_id 'timeout-int-001', got %v", resp["command_id"])
	}
	if resp["status"] != "timeout" {
		t.Errorf("expected status 'timeout', got %v", resp["status"])
	}
}
