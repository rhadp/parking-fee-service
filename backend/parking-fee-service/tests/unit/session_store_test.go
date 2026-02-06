// Package unit contains unit tests for the parking-fee-service.
package unit

import (
	"database/sql"
	"testing"
	"time"

	_ "modernc.org/sqlite"

	"github.com/sdv-parking-demo/backend/parking-fee-service/internal/model"
	"github.com/sdv-parking-demo/backend/parking-fee-service/internal/store"
)

func setupTestDB(t *testing.T) *sql.DB {
	db, err := sql.Open("sqlite", ":memory:")
	if err != nil {
		t.Fatalf("failed to open test database: %v", err)
	}
	return db
}

// TestSessionStore_InitSchema tests that schema initialization works.
func TestSessionStore_InitSchema(t *testing.T) {
	db := setupTestDB(t)
	defer db.Close()

	sessionStore := store.NewSessionStore(db)

	if sessionStore.IsInitialized() {
		t.Error("store should not be initialized before InitSchema")
	}

	err := sessionStore.InitSchema()
	if err != nil {
		t.Fatalf("InitSchema failed: %v", err)
	}

	if !sessionStore.IsInitialized() {
		t.Error("store should be initialized after InitSchema")
	}
}

// TestSessionStore_SaveGet tests save and get round-trip.
func TestSessionStore_SaveGet(t *testing.T) {
	db := setupTestDB(t)
	defer db.Close()

	sessionStore := store.NewSessionStore(db)
	if err := sessionStore.InitSchema(); err != nil {
		t.Fatalf("InitSchema failed: %v", err)
	}

	session := &model.Session{
		SessionID:  "sess-123",
		VehicleID:  "vehicle-001",
		ZoneID:     "zone-001",
		Latitude:   37.5,
		Longitude:  -122.5,
		StartTime:  time.Now().Truncate(time.Second),
		HourlyRate: 2.50,
		State:      model.SessionStateActive,
	}

	// Save
	err := sessionStore.Save(session)
	if err != nil {
		t.Fatalf("Save failed: %v", err)
	}

	// Get
	retrieved := sessionStore.Get("sess-123")
	if retrieved == nil {
		t.Fatal("Get returned nil")
	}

	if retrieved.SessionID != session.SessionID {
		t.Errorf("SessionID mismatch: got %s, want %s", retrieved.SessionID, session.SessionID)
	}
	if retrieved.VehicleID != session.VehicleID {
		t.Errorf("VehicleID mismatch: got %s, want %s", retrieved.VehicleID, session.VehicleID)
	}
	if retrieved.State != session.State {
		t.Errorf("State mismatch: got %s, want %s", retrieved.State, session.State)
	}
}

// TestSessionStore_Update tests that update modifies existing session.
func TestSessionStore_Update(t *testing.T) {
	db := setupTestDB(t)
	defer db.Close()

	sessionStore := store.NewSessionStore(db)
	if err := sessionStore.InitSchema(); err != nil {
		t.Fatalf("InitSchema failed: %v", err)
	}

	session := &model.Session{
		SessionID:  "sess-456",
		VehicleID:  "vehicle-002",
		ZoneID:     "zone-001",
		Latitude:   37.5,
		Longitude:  -122.5,
		StartTime:  time.Now().Truncate(time.Second),
		HourlyRate: 2.50,
		State:      model.SessionStateActive,
	}

	// Save initial
	if err := sessionStore.Save(session); err != nil {
		t.Fatalf("Save failed: %v", err)
	}

	// Update to stopped
	endTime := time.Now().Add(time.Hour).Truncate(time.Second)
	totalCost := 2.50
	paymentStatus := "success"
	session.EndTime = &endTime
	session.State = model.SessionStateStopped
	session.TotalCost = &totalCost
	session.PaymentStatus = &paymentStatus

	if err := sessionStore.Update(session); err != nil {
		t.Fatalf("Update failed: %v", err)
	}

	// Verify update
	retrieved := sessionStore.Get("sess-456")
	if retrieved == nil {
		t.Fatal("Get returned nil after update")
	}

	if retrieved.State != model.SessionStateStopped {
		t.Errorf("State not updated: got %s, want %s", retrieved.State, model.SessionStateStopped)
	}
	if retrieved.EndTime == nil {
		t.Error("EndTime should not be nil after update")
	}
	if retrieved.TotalCost == nil || *retrieved.TotalCost != totalCost {
		t.Errorf("TotalCost not updated correctly")
	}
}

// TestSessionStore_GetActiveByVehicle tests finding active session for vehicle.
func TestSessionStore_GetActiveByVehicle(t *testing.T) {
	db := setupTestDB(t)
	defer db.Close()

	sessionStore := store.NewSessionStore(db)
	if err := sessionStore.InitSchema(); err != nil {
		t.Fatalf("InitSchema failed: %v", err)
	}

	// Create an active session
	activeSession := &model.Session{
		SessionID:  "sess-active",
		VehicleID:  "vehicle-003",
		ZoneID:     "zone-001",
		Latitude:   37.5,
		Longitude:  -122.5,
		StartTime:  time.Now().Truncate(time.Second),
		HourlyRate: 2.50,
		State:      model.SessionStateActive,
	}

	if err := sessionStore.Save(activeSession); err != nil {
		t.Fatalf("Save failed: %v", err)
	}

	// Find by vehicle
	found := sessionStore.GetActiveByVehicle("vehicle-003")
	if found == nil {
		t.Fatal("GetActiveByVehicle returned nil")
	}

	if found.SessionID != activeSession.SessionID {
		t.Errorf("SessionID mismatch: got %s, want %s", found.SessionID, activeSession.SessionID)
	}

	// Should not find for different vehicle
	notFound := sessionStore.GetActiveByVehicle("vehicle-other")
	if notFound != nil {
		t.Error("GetActiveByVehicle should return nil for non-existent vehicle")
	}
}

// TestSessionStore_Ping tests database connection verification.
func TestSessionStore_Ping(t *testing.T) {
	db := setupTestDB(t)
	defer db.Close()

	sessionStore := store.NewSessionStore(db)

	err := sessionStore.Ping()
	if err != nil {
		t.Errorf("Ping failed: %v", err)
	}
}
