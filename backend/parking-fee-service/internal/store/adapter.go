// Package store provides data storage implementations for the parking-fee-service.
package store

import (
	"sync"

	"github.com/sdv-parking-demo/backend/parking-fee-service/internal/model"
)

// AdapterStore provides in-memory storage for parking operator adapters.
type AdapterStore struct {
	adapters map[string]model.Adapter
	mu       sync.RWMutex
}

// NewAdapterStore creates a new AdapterStore with the given adapters.
func NewAdapterStore(adapters []model.Adapter) *AdapterStore {
	store := &AdapterStore{
		adapters: make(map[string]model.Adapter),
	}
	for _, adapter := range adapters {
		store.adapters[adapter.AdapterID] = adapter
	}
	return store
}

// List returns all adapters in the store.
func (s *AdapterStore) List() []model.Adapter {
	s.mu.RLock()
	defer s.mu.RUnlock()

	result := make([]model.Adapter, 0, len(s.adapters))
	for _, adapter := range s.adapters {
		result = append(result, adapter)
	}
	return result
}

// Get returns the adapter with the given ID.
// Returns nil if the adapter is not found.
func (s *AdapterStore) Get(adapterID string) *model.Adapter {
	s.mu.RLock()
	defer s.mu.RUnlock()

	if adapter, ok := s.adapters[adapterID]; ok {
		return &adapter
	}
	return nil
}
