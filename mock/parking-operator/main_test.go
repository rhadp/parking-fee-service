package main

import "testing"

// TestNewSessionStore verifies the session store can be created.
func TestNewSessionStore(t *testing.T) {
	store := NewSessionStore()
	if store == nil {
		t.Fatal("expected non-nil session store")
	}
}

// TestNewHandler verifies the handler can be created.
func TestNewHandler(t *testing.T) {
	store := NewSessionStore()
	h := NewHandler(store)
	if h == nil {
		t.Fatal("expected non-nil handler")
	}
}
