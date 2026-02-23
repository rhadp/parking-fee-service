// Package store provides an in-memory operator data store.
package store

import (
	"encoding/json"
	"fmt"
	"os"

	"github.com/rhadp/parking-fee-service/backend/parking-fee-service/internal/model"
)

// Store holds the operator dataset and provides lookup methods.
type Store struct {
	operators map[string]model.Operator
}

// NewDefaultStore creates a store loaded with the embedded default operator
// dataset.
func NewDefaultStore() *Store {
	return NewStoreFromOperators(defaultOperators())
}

// NewEmptyStore creates a store with no operators.
func NewEmptyStore() *Store {
	return &Store{operators: make(map[string]model.Operator)}
}

// NewStoreFromOperators creates a store populated with the given operators.
func NewStoreFromOperators(ops []model.Operator) *Store {
	m := make(map[string]model.Operator, len(ops))
	for _, op := range ops {
		m[op.ID] = op
	}
	return &Store{operators: m}
}

// NewStoreFromFile loads operator data from a JSON file at the given path.
// Returns a populated store or an error if the file is missing or malformed.
func NewStoreFromFile(path string) (*Store, error) {
	data, err := os.ReadFile(path)
	if err != nil {
		return nil, fmt.Errorf("failed to read operator config file: %w", err)
	}

	var cfg model.OperatorsConfig
	if err := json.Unmarshal(data, &cfg); err != nil {
		return nil, fmt.Errorf("failed to parse operator config file: %w", err)
	}

	return NewStoreFromOperators(cfg.Operators), nil
}

// ListOperators returns all operators in the store.
func (s *Store) ListOperators() []model.Operator {
	if s == nil {
		return nil
	}
	result := make([]model.Operator, 0, len(s.operators))
	for _, op := range s.operators {
		result = append(result, op)
	}
	return result
}

// GetOperator returns the operator with the given ID, or false if not found.
func (s *Store) GetOperator(id string) (model.Operator, bool) {
	if s == nil {
		return model.Operator{}, false
	}
	op, ok := s.operators[id]
	return op, ok
}
