package main

import "testing"

func newTestStore(t *testing.T) *Store {
	t.Helper()
	cfg, err := LoadConfig("")
	if err != nil {
		t.Fatalf("failed to load default config: %v", err)
	}
	return NewStore(cfg)
}

// TestFindOperatorsByLocation tests that operators are found for coordinates inside a zone.
func TestFindOperatorsByLocation(t *testing.T) {
	store := newTestStore(t)

	// Inside zone-muc-central
	operators := store.FindOperatorsByLocation(48.1395, 11.5625)
	if len(operators) == 0 {
		t.Fatal("expected at least 1 operator for location inside zone-muc-central")
	}

	found := false
	for _, op := range operators {
		if op.ID == "muc-central" {
			found = true
			break
		}
	}
	if !found {
		t.Error("expected muc-central operator in results")
	}
}

// TestGetAdapterMetadata tests retrieving adapter metadata for a known operator.
func TestGetAdapterMetadata(t *testing.T) {
	store := newTestStore(t)

	metadata, ok := store.GetAdapterMetadata("muc-central")
	if !ok {
		t.Fatal("expected adapter metadata for muc-central")
	}
	if metadata == nil {
		t.Fatal("expected non-nil adapter metadata")
	}
	if metadata.ImageRef == "" {
		t.Error("expected non-empty image_ref")
	}
	if metadata.Version == "" {
		t.Error("expected non-empty version")
	}
}

// TestGetAdapterMetadataUnknown tests that unknown operator returns false.
func TestGetAdapterMetadataUnknown(t *testing.T) {
	store := newTestStore(t)

	_, ok := store.GetAdapterMetadata("nonexistent-operator")
	if ok {
		t.Error("expected false for unknown operator")
	}
}
