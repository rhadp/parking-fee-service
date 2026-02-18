package mqtt

import (
	"encoding/json"
	"log"

	"github.com/rhadp/parking-fee-service/backend/cloud-gateway/messages"
)

// handleCommandResponse processes a CommandResponse message received on
// vehicles/{vin}/command_responses. It updates the corresponding command
// entry in the state store.
//
// 03-REQ-2.3: Update vehicle state with command result.
// 03-REQ-2.E2: Log warning and discard if command_id is unknown.
func (c *Client) handleCommandResponse(vin string, payload []byte) {
	var resp messages.CommandResponse
	if err := json.Unmarshal(payload, &resp); err != nil {
		log.Printf("mqtt: invalid command_response JSON from VIN %s: %v", vin, err)
		return
	}

	updated := c.store.UpdateCommandResult(vin, resp.CommandID, string(resp.Result))
	if !updated {
		log.Printf("mqtt: warning: unknown command_id %s for VIN %s (discarding)", resp.CommandID, vin)
		return
	}

	// Also update vehicle lock state based on the command result.
	if resp.Result == messages.CommandResultSuccess {
		switch resp.Type {
		case messages.CommandTypeLock:
			locked := true
			c.store.UpdateState(vin, &locked, nil, nil, nil, nil, nil)
		case messages.CommandTypeUnlock:
			locked := false
			c.store.UpdateState(vin, &locked, nil, nil, nil, nil, nil)
		}
	}

	log.Printf("mqtt: command %s result=%s for VIN %s", resp.CommandID, resp.Result, vin)
}

// handleTelemetry processes a TelemetryMessage received on
// vehicles/{vin}/telemetry. It updates the vehicle's cached state.
//
// 03-REQ-2.4: Update cached vehicle state from telemetry.
func (c *Client) handleTelemetry(vin string, payload []byte) {
	var tel messages.TelemetryMessage
	if err := json.Unmarshal(payload, &tel); err != nil {
		log.Printf("mqtt: invalid telemetry JSON from VIN %s: %v", vin, err)
		return
	}

	c.store.UpdateState(
		vin,
		tel.IsLocked,
		tel.IsDoorOpen,
		tel.Speed,
		tel.Latitude,
		tel.Longitude,
		tel.ParkingSessionActive,
	)

	log.Printf("mqtt: telemetry update for VIN %s", vin)
}

// handleRegistration processes a RegistrationMessage received on
// vehicles/{vin}/registration. It registers the vehicle in the state store.
//
// 03-REQ-5.3: Store VIN and pairing PIN in vehicle registry.
func (c *Client) handleRegistration(vin string, payload []byte) {
	var reg messages.RegistrationMessage
	if err := json.Unmarshal(payload, &reg); err != nil {
		log.Printf("mqtt: invalid registration JSON from VIN %s: %v", vin, err)
		return
	}

	// Use the VIN from the message payload (should match the topic VIN).
	regVIN := reg.VIN
	if regVIN == "" {
		regVIN = vin
	}

	c.store.RegisterVehicle(regVIN, reg.PairingPIN)
	log.Printf("mqtt: registered vehicle VIN=%s", regVIN)
}

// handleStatusResponse processes a StatusResponse received on
// vehicles/{vin}/status_response. It updates the vehicle's cached state.
func (c *Client) handleStatusResponse(vin string, payload []byte) {
	var resp messages.StatusResponse
	if err := json.Unmarshal(payload, &resp); err != nil {
		log.Printf("mqtt: invalid status_response JSON from VIN %s: %v", vin, err)
		return
	}

	c.store.UpdateState(
		vin,
		resp.IsLocked,
		resp.IsDoorOpen,
		resp.Speed,
		resp.Latitude,
		resp.Longitude,
		resp.ParkingSessionActive,
	)

	log.Printf("mqtt: status response for VIN %s (request_id=%s)", vin, resp.RequestID)
}
