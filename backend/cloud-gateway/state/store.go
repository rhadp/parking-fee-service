// Package state provides a thread-safe in-memory store for vehicle entries,
// pairing tokens, and command tracking. It is the central data structure for
// the CLOUD_GATEWAY service.
package state

import (
	"crypto/rand"
	"encoding/base64"
	"fmt"
	"sync"
	"time"
)

// VehicleEntry holds all known data for a single registered vehicle.
type VehicleEntry struct {
	VIN        string
	PairingPIN string
	PairToken  string // empty until paired

	// Latest known state (from telemetry/status responses).
	IsLocked             *bool
	IsDoorOpen           *bool
	Speed                *float64
	Latitude             *float64
	Longitude            *float64
	ParkingSessionActive *bool
	StateUpdatedAt       time.Time

	// Pending commands keyed by command_id.
	Commands map[string]*CommandEntry
}

// CommandEntry tracks the lifecycle of a single lock/unlock command.
type CommandEntry struct {
	CommandID string
	Type      string // "lock" or "unlock"
	Status    string // "accepted", "success", "rejected"
	Result    string // "", "SUCCESS", "REJECTED_SPEED", "REJECTED_DOOR_OPEN"
	CreatedAt time.Time
	UpdatedAt time.Time
}

// Store is a thread-safe in-memory vehicle registry.
type Store struct {
	mu       sync.RWMutex
	vehicles map[string]*VehicleEntry // keyed by VIN
	tokens   map[string]string        // token → VIN reverse lookup
}

// NewStore creates an empty Store.
func NewStore() *Store {
	return &Store{
		vehicles: make(map[string]*VehicleEntry),
		tokens:   make(map[string]string),
	}
}

// RegisterVehicle adds or updates a vehicle entry with the given VIN and
// pairing PIN. If the vehicle already exists, the pairing PIN is updated
// but existing state and commands are preserved.
func (s *Store) RegisterVehicle(vin, pairingPIN string) {
	s.mu.Lock()
	defer s.mu.Unlock()

	if entry, ok := s.vehicles[vin]; ok {
		entry.PairingPIN = pairingPIN
		return
	}

	s.vehicles[vin] = &VehicleEntry{
		VIN:        vin,
		PairingPIN: pairingPIN,
		Commands:   make(map[string]*CommandEntry),
	}
}

// GetVehicle returns a copy of the vehicle entry for the given VIN.
// Returns nil if the VIN is not registered.
func (s *Store) GetVehicle(vin string) *VehicleEntry {
	s.mu.RLock()
	defer s.mu.RUnlock()

	entry, ok := s.vehicles[vin]
	if !ok {
		return nil
	}

	// Return a shallow copy to prevent callers from modifying internal state.
	cp := *entry
	// Copy commands map.
	cp.Commands = make(map[string]*CommandEntry, len(entry.Commands))
	for k, v := range entry.Commands {
		cmdCopy := *v
		cp.Commands[k] = &cmdCopy
	}
	return &cp
}

// UpdateState updates the cached vehicle state from telemetry or status
// response data.
func (s *Store) UpdateState(vin string, isLocked, isDoorOpen *bool, speed, latitude, longitude *float64, parkingSessionActive *bool) {
	s.mu.Lock()
	defer s.mu.Unlock()

	entry, ok := s.vehicles[vin]
	if !ok {
		return
	}

	if isLocked != nil {
		entry.IsLocked = isLocked
	}
	if isDoorOpen != nil {
		entry.IsDoorOpen = isDoorOpen
	}
	if speed != nil {
		entry.Speed = speed
	}
	if latitude != nil {
		entry.Latitude = latitude
	}
	if longitude != nil {
		entry.Longitude = longitude
	}
	if parkingSessionActive != nil {
		entry.ParkingSessionActive = parkingSessionActive
	}
	entry.StateUpdatedAt = time.Now()
}

// AddCommand creates a new pending command for the given vehicle.
// Returns the created CommandEntry or an error if the VIN is not registered.
func (s *Store) AddCommand(vin, commandID, cmdType string) (*CommandEntry, error) {
	s.mu.Lock()
	defer s.mu.Unlock()

	entry, ok := s.vehicles[vin]
	if !ok {
		return nil, fmt.Errorf("vehicle %s not found", vin)
	}

	now := time.Now()
	cmd := &CommandEntry{
		CommandID: commandID,
		Type:      cmdType,
		Status:    "accepted",
		CreatedAt: now,
		UpdatedAt: now,
	}
	entry.Commands[commandID] = cmd

	cmdCopy := *cmd
	return &cmdCopy, nil
}

// UpdateCommandResult updates the result of a previously created command.
// Returns true if the command was found and updated, false otherwise.
func (s *Store) UpdateCommandResult(vin, commandID, result string) bool {
	s.mu.Lock()
	defer s.mu.Unlock()

	entry, ok := s.vehicles[vin]
	if !ok {
		return false
	}

	cmd, ok := entry.Commands[commandID]
	if !ok {
		return false
	}

	cmd.Result = result
	if result == "SUCCESS" {
		cmd.Status = "success"
	} else {
		cmd.Status = "rejected"
	}
	cmd.UpdatedAt = time.Now()
	return true
}

// PairVehicle validates the VIN and PIN, generates a bearer token, and
// stores it. Returns the token and VIN on success.
//
// Error cases:
//   - VIN not registered → returns ("", "", ErrVehicleNotFound)
//   - PIN mismatch → returns ("", "", ErrPINMismatch)
func (s *Store) PairVehicle(vin, pin string) (token string, err error) {
	s.mu.Lock()
	defer s.mu.Unlock()

	entry, ok := s.vehicles[vin]
	if !ok {
		return "", ErrVehicleNotFound
	}

	if entry.PairingPIN != pin {
		return "", ErrPINMismatch
	}

	token, err = generateToken()
	if err != nil {
		return "", fmt.Errorf("generating token: %w", err)
	}

	// Remove old token from reverse lookup if re-pairing.
	if entry.PairToken != "" {
		delete(s.tokens, entry.PairToken)
	}

	entry.PairToken = token
	s.tokens[token] = vin
	return token, nil
}

// ValidateToken checks whether the given bearer token is valid and
// associated with the specified VIN. Returns true if authorized.
func (s *Store) ValidateToken(token, vin string) bool {
	s.mu.RLock()
	defer s.mu.RUnlock()

	associatedVIN, ok := s.tokens[token]
	if !ok {
		return false
	}
	return associatedVIN == vin
}

// Sentinel errors for pairing operations.
var (
	ErrVehicleNotFound = fmt.Errorf("vehicle not found")
	ErrPINMismatch     = fmt.Errorf("PIN mismatch")
)

// generateToken produces a cryptographically random 32-byte base64-encoded
// bearer token.
func generateToken() (string, error) {
	b := make([]byte, 32)
	if _, err := rand.Read(b); err != nil {
		return "", err
	}
	return base64.URLEncoding.EncodeToString(b), nil
}
