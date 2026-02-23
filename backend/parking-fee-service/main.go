package main

import (
	"fmt"
	"log"
	"net/http"

	"github.com/rhadp/parking-fee-service/backend/parking-fee-service/internal/config"
	"github.com/rhadp/parking-fee-service/backend/parking-fee-service/internal/handler"
	"github.com/rhadp/parking-fee-service/backend/parking-fee-service/internal/store"
)

func main() {
	cfg := config.LoadConfig()

	// Create the operator store — from file if configured, else embedded defaults.
	var s *store.Store
	if cfg.OperatorsConfigPath != "" {
		var err error
		s, err = store.NewStoreFromFile(cfg.OperatorsConfigPath)
		if err != nil {
			log.Fatalf("failed to load operator config: %v", err)
		}
	} else {
		s = store.NewDefaultStore()
	}

	router := handler.NewRouter(s, cfg.AuthTokens, cfg.FuzzinessMeters)

	addr := ":" + cfg.Port
	fmt.Printf("parking-fee-service listening on %s\n", addr)
	if err := http.ListenAndServe(addr, router); err != nil {
		log.Fatalf("server error: %v", err)
	}
}
