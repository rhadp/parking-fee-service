package api

import (
	"encoding/json"
	"fmt"
	"log"
	"net/http"
	"time"

	"github.com/google/uuid"
	"github.com/rhadp/parking-fee-service/backend/cloud-gateway/state"
)

// MQTTPublisher is an interface for publishing MQTT messages.
// This allows handlers to be tested without a real MQTT connection.
// In task group 3, the real MQTT client will implement this interface.
type MQTTPublisher interface {
	// PublishCommand publishes a lock/unlock command to MQTT.
	// Returns an error if the broker is unreachable.
	PublishCommand(vin, commandID, cmdType string) error
}

// noopPublisher is a placeholder that always succeeds.
// It will be replaced by a real MQTT publisher in task group 3.
type noopPublisher struct{}

func (p *noopPublisher) PublishCommand(vin, commandID, cmdType string) error {
	return nil
}

// Handlers holds dependencies for all REST endpoint handlers.
type Handlers struct {
	Store     *state.Store
	Publisher MQTTPublisher
}

// NewHandlers creates a Handlers with the given store and publisher.
// If publisher is nil, a no-op publisher is used.
func NewHandlers(store *state.Store, publisher MQTTPublisher) *Handlers {
	if publisher == nil {
		publisher = &noopPublisher{}
	}
	return &Handlers{
		Store:     store,
		Publisher: publisher,
	}
}

// RegisterRoutes registers all REST API routes on the given ServeMux.
func (h *Handlers) RegisterRoutes(mux *http.ServeMux) {
	mux.HandleFunc("GET /healthz", h.HandleHealthz)
	mux.HandleFunc("POST /api/v1/pair", h.HandlePair)

	// Protected endpoints — wrap with auth middleware.
	mux.Handle("POST /api/v1/vehicles/{vin}/lock",
		AuthMiddleware(h.Store, http.HandlerFunc(h.HandleLock)))
	mux.Handle("POST /api/v1/vehicles/{vin}/unlock",
		AuthMiddleware(h.Store, http.HandlerFunc(h.HandleUnlock)))
	mux.Handle("GET /api/v1/vehicles/{vin}/status",
		AuthMiddleware(h.Store, http.HandlerFunc(h.HandleStatus)))
}

// HandleHealthz returns 200 OK when the service is operational.
func (h *Handlers) HandleHealthz(w http.ResponseWriter, r *http.Request) {
	w.Header().Set("Content-Type", "application/json")
	w.WriteHeader(http.StatusOK)
	fmt.Fprintln(w, `{"status":"ok"}`)
}

// PairRequest is the JSON body for POST /api/v1/pair.
type PairRequest struct {
	VIN string `json:"vin"`
	PIN string `json:"pin"`
}

// PairResponse is the JSON response for a successful pairing.
type PairResponse struct {
	Token string `json:"token"`
	VIN   string `json:"vin"`
}

// HandlePair handles POST /api/v1/pair — pairs a companion app with a vehicle.
func (h *Handlers) HandlePair(w http.ResponseWriter, r *http.Request) {
	var req PairRequest
	if err := json.NewDecoder(r.Body).Decode(&req); err != nil {
		writeError(w, http.StatusBadRequest, "invalid request body", "BAD_REQUEST")
		return
	}

	if req.VIN == "" || req.PIN == "" {
		writeError(w, http.StatusBadRequest, "vin and pin are required", "BAD_REQUEST")
		return
	}

	token, err := h.Store.PairVehicle(req.VIN, req.PIN)
	if err == state.ErrVehicleNotFound {
		writeError(w, http.StatusNotFound, "vehicle not found", "NOT_FOUND")
		return
	}
	if err == state.ErrPINMismatch {
		writeError(w, http.StatusForbidden, "incorrect pairing PIN", "FORBIDDEN")
		return
	}
	if err != nil {
		log.Printf("error pairing vehicle %s: %v", req.VIN, err)
		writeError(w, http.StatusInternalServerError, "internal error", "INTERNAL_ERROR")
		return
	}

	w.Header().Set("Content-Type", "application/json")
	w.WriteHeader(http.StatusOK)
	json.NewEncoder(w).Encode(PairResponse{
		Token: token,
		VIN:   req.VIN,
	})
}

// CommandAcceptedResponse is the JSON response for accepted lock/unlock commands.
type CommandAcceptedResponse struct {
	CommandID string `json:"command_id"`
	Status    string `json:"status"`
}

// HandleLock handles POST /api/v1/vehicles/{vin}/lock — sends a lock command.
func (h *Handlers) HandleLock(w http.ResponseWriter, r *http.Request) {
	h.handleCommand(w, r, "lock")
}

// HandleUnlock handles POST /api/v1/vehicles/{vin}/unlock — sends an unlock command.
func (h *Handlers) HandleUnlock(w http.ResponseWriter, r *http.Request) {
	h.handleCommand(w, r, "unlock")
}

// handleCommand is the shared implementation for lock and unlock commands.
func (h *Handlers) handleCommand(w http.ResponseWriter, r *http.Request, cmdType string) {
	vin := r.PathValue("vin")

	// Check if the vehicle exists.
	if v := h.Store.GetVehicle(vin); v == nil {
		writeError(w, http.StatusNotFound, "vehicle not found", "NOT_FOUND")
		return
	}

	commandID := uuid.New().String()

	cmd, err := h.Store.AddCommand(vin, commandID, cmdType)
	if err != nil {
		writeError(w, http.StatusNotFound, "vehicle not found", "NOT_FOUND")
		return
	}

	// Publish to MQTT (async — don't wait for response).
	if err := h.Publisher.PublishCommand(vin, commandID, cmdType); err != nil {
		log.Printf("MQTT publish failed for %s command %s on VIN %s: %v", cmdType, commandID, vin, err)
		writeError(w, http.StatusServiceUnavailable, "MQTT broker unreachable", "SERVICE_UNAVAILABLE")
		return
	}

	// Return 202 Accepted immediately (async command pattern).
	w.Header().Set("Content-Type", "application/json")
	w.WriteHeader(http.StatusAccepted)
	json.NewEncoder(w).Encode(CommandAcceptedResponse{
		CommandID: cmd.CommandID,
		Status:    "accepted",
	})
}

// VehicleStatusResponse is the JSON response for GET /api/v1/vehicles/{vin}/status.
type VehicleStatusResponse struct {
	VIN                  string               `json:"vin"`
	IsLocked             *bool                `json:"is_locked"`
	IsDoorOpen           *bool                `json:"is_door_open"`
	Speed                *float64             `json:"speed"`
	Latitude             *float64             `json:"latitude"`
	Longitude            *float64             `json:"longitude"`
	ParkingSessionActive *bool                `json:"parking_session_active"`
	LastCommand          *LastCommandResponse `json:"last_command,omitempty"`
	UpdatedAt            string               `json:"updated_at"`
}

// LastCommandResponse represents the most recent command in the status response.
type LastCommandResponse struct {
	CommandID string `json:"command_id"`
	Type      string `json:"type"`
	Status    string `json:"status"`
	Result    string `json:"result"`
}

// HandleStatus handles GET /api/v1/vehicles/{vin}/status — returns vehicle state.
func (h *Handlers) HandleStatus(w http.ResponseWriter, r *http.Request) {
	vin := r.PathValue("vin")

	v := h.Store.GetVehicle(vin)
	if v == nil {
		writeError(w, http.StatusNotFound, "vehicle not found", "NOT_FOUND")
		return
	}

	resp := VehicleStatusResponse{
		VIN:                  v.VIN,
		IsLocked:             v.IsLocked,
		IsDoorOpen:           v.IsDoorOpen,
		Speed:                v.Speed,
		Latitude:             v.Latitude,
		Longitude:            v.Longitude,
		ParkingSessionActive: v.ParkingSessionActive,
	}

	if !v.StateUpdatedAt.IsZero() {
		resp.UpdatedAt = v.StateUpdatedAt.Format(time.RFC3339)
	}

	// Find the most recent command.
	var latest *state.CommandEntry
	for _, cmd := range v.Commands {
		if latest == nil || cmd.UpdatedAt.After(latest.UpdatedAt) {
			latest = cmd
		}
	}
	if latest != nil {
		resp.LastCommand = &LastCommandResponse{
			CommandID: latest.CommandID,
			Type:      latest.Type,
			Status:    latest.Status,
			Result:    latest.Result,
		}
	}

	w.Header().Set("Content-Type", "application/json")
	w.WriteHeader(http.StatusOK)
	json.NewEncoder(w).Encode(resp)
}
