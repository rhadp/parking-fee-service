package store

import (
	"testing"
)

// --- TS-05-17: Operator data loaded from config ---

func TestStore_LoadFromJSON(t *testing.T) {
	s, err := NewStoreFromFile("../../testdata/operators.json")
	if err != nil {
		t.Fatalf("NewStoreFromFile returned error: %v", err)
	}
	if s == nil {
		t.Fatal("NewStoreFromFile returned nil store")
	}

	ops := s.ListOperators()
	if len(ops) < 1 {
		t.Errorf("expected at least 1 operator loaded from JSON, got %d", len(ops))
	}
}

// --- TS-05-18: At least two demo operators ---

func TestStore_DefaultOperators(t *testing.T) {
	s := NewDefaultStore()
	if s == nil {
		t.Fatal("NewDefaultStore() returned nil")
	}

	ops := s.ListOperators()
	if len(ops) < 2 {
		t.Fatalf("expected at least 2 default operators, got %d", len(ops))
	}

	// Verify distinct zone IDs
	zoneIDs := make(map[string]bool)
	for _, op := range ops {
		zoneIDs[op.Zone.ID] = true
	}
	if len(zoneIDs) < 2 {
		t.Errorf("expected at least 2 distinct zone IDs, got %d", len(zoneIDs))
	}
}

// --- TS-05-20: Default embedded dataset when no config ---

func TestStore_DefaultWhenNoConfig(t *testing.T) {
	s := NewDefaultStore()
	if s == nil {
		t.Fatal("NewDefaultStore() returned nil")
	}

	ops := s.ListOperators()
	if len(ops) < 2 {
		t.Fatalf("expected at least 2 operators, got %d", len(ops))
	}

	// Check that op-munich-01 is present
	found := false
	for _, op := range ops {
		if op.ID == "op-munich-01" {
			found = true
			break
		}
	}
	if !found {
		t.Error("expected op-munich-01 in default store")
	}
}

// --- TS-05-E11: Malformed config file prevents startup ---

func TestEdge_MalformedConfig(t *testing.T) {
	s, err := NewStoreFromFile("../../testdata/invalid.json")
	if err == nil {
		t.Error("expected error for malformed JSON config, got nil")
	}
	if s != nil {
		t.Error("expected nil store for malformed JSON config")
	}
}

// --- TS-05-E12: Missing config file prevents startup ---

func TestEdge_MissingConfigFile(t *testing.T) {
	s, err := NewStoreFromFile("../../testdata/does_not_exist.json")
	if err == nil {
		t.Error("expected error for missing config file, got nil")
	}
	if s != nil {
		t.Error("expected nil store for missing config file")
	}
}
