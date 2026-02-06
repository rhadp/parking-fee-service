// Package service provides business logic for the parking-fee-service.
package service

import (
	"sort"

	"github.com/sdv-parking-demo/backend/parking-fee-service/internal/model"
	"github.com/sdv-parking-demo/backend/parking-fee-service/internal/store"
)

// AdapterService provides business logic for adapter registry operations.
type AdapterService struct {
	store *store.AdapterStore
}

// NewAdapterService creates a new AdapterService with the given store.
func NewAdapterService(store *store.AdapterStore) *AdapterService {
	return &AdapterService{
		store: store,
	}
}

// ListAdapters returns all adapters sorted alphabetically by operator name.
func (s *AdapterService) ListAdapters() []model.AdapterSummary {
	adapters := s.store.List()

	// Sort by operator name
	sort.Slice(adapters, func(i, j int) bool {
		return adapters[i].OperatorName < adapters[j].OperatorName
	})

	// Convert to summaries
	summaries := make([]model.AdapterSummary, len(adapters))
	for i, adapter := range adapters {
		summaries[i] = model.AdapterSummary{
			AdapterID:    adapter.AdapterID,
			OperatorName: adapter.OperatorName,
			Version:      adapter.Version,
			ImageRef:     adapter.ImageRef,
		}
	}
	return summaries
}

// GetAdapter returns the adapter with the given ID.
// Returns nil if the adapter is not found.
func (s *AdapterService) GetAdapter(adapterID string) *model.Adapter {
	return s.store.Get(adapterID)
}
