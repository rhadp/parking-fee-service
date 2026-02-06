package handler

import (
	"net/http"
	"time"

	"github.com/sdv-parking-demo/backend/cloud-gateway/internal/model"
	"github.com/sdv-parking-demo/backend/cloud-gateway/internal/mqtt"
)

// HealthHandler handles health and readiness HTTP requests.
type HealthHandler struct {
	mqttClient  mqtt.Client
	serviceName string
}

// NewHealthHandler creates a new HealthHandler.
func NewHealthHandler(mqttClient mqtt.Client, serviceName string) *HealthHandler {
	return &HealthHandler{
		mqttClient:  mqttClient,
		serviceName: serviceName,
	}
}

// HandleHealth handles GET /health.
// Returns healthy status with service name and timestamp.
func (h *HealthHandler) HandleHealth(w http.ResponseWriter, r *http.Request) {
	response := model.HealthResponse{
		Status:    "healthy",
		Service:   h.serviceName,
		Timestamp: time.Now().UTC().Format(time.RFC3339),
	}
	WriteJSON(w, http.StatusOK, response)
}

// HandleReady handles GET /ready.
// Returns ready/not ready status based on MQTT connection.
func (h *HealthHandler) HandleReady(w http.ResponseWriter, r *http.Request) {
	mqttConnected := h.mqttClient != nil && h.mqttClient.IsConnected()

	status := "ready"
	statusCode := http.StatusOK
	if !mqttConnected {
		status = "not_ready"
		statusCode = http.StatusServiceUnavailable
	}

	response := model.ReadyResponse{
		Status:        status,
		MQTTConnected: mqttConnected,
	}
	WriteJSON(w, statusCode, response)
}
