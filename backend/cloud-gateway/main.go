// Package main implements the CLOUD_GATEWAY service.
//
// CLOUD_GATEWAY provides a REST API for vehicle remote operations (lock,
// unlock, status) and vehicle pairing. It connects to an MQTT broker
// (Mosquitto) for vehicle communication and maintains an in-memory vehicle
// state store.
package main

import (
	"context"
	"flag"
	"log"
	"net/http"
	"os"
	"os/signal"
	"syscall"
	"time"

	"github.com/rhadp/parking-fee-service/backend/cloud-gateway/api"
	mqttclient "github.com/rhadp/parking-fee-service/backend/cloud-gateway/mqtt"
	"github.com/rhadp/parking-fee-service/backend/cloud-gateway/state"
)

func main() {
	listenAddr := flag.String("listen-addr", envOrDefault("LISTEN_ADDR", ":8081"), "REST API listen address")
	mqttAddr := flag.String("mqtt-addr", envOrDefault("MQTT_ADDR", "localhost:1883"), "MQTT broker address")
	flag.Parse()

	store := state.NewStore()

	// Connect to MQTT broker. The MQTT client subscribes to vehicle
	// response/telemetry/registration topics and updates the state store.
	mqttClient, err := mqttclient.NewClient(*mqttAddr, store)
	if err != nil {
		log.Fatalf("cloud-gateway: failed to connect to MQTT broker at %s: %v", *mqttAddr, err)
	}
	defer mqttClient.Disconnect()

	mux := newServeMux(store, mqttClient)

	srv := &http.Server{
		Addr:    *listenAddr,
		Handler: mux,
	}

	// Channel to listen for OS signals.
	sigCh := make(chan os.Signal, 1)
	signal.Notify(sigCh, syscall.SIGINT, syscall.SIGTERM)

	// Start server in a goroutine.
	go func() {
		log.Printf("cloud-gateway starting on %s (mqtt=%s)", *listenAddr, *mqttAddr)
		if err := srv.ListenAndServe(); err != nil && err != http.ErrServerClosed {
			log.Fatalf("cloud-gateway failed to start: %v", err)
		}
	}()

	// Wait for signal.
	sig := <-sigCh
	log.Printf("cloud-gateway received signal %v, shutting down", sig)

	ctx, cancel := context.WithTimeout(context.Background(), 5*time.Second)
	defer cancel()

	if err := srv.Shutdown(ctx); err != nil {
		log.Fatalf("cloud-gateway shutdown error: %v", err)
	}

	log.Println("cloud-gateway stopped")
}

// newServeMux creates the HTTP mux with all REST API routes.
// The publisher is used by lock/unlock handlers to publish MQTT commands.
func newServeMux(store *state.Store, publisher api.MQTTPublisher) *http.ServeMux {
	mux := http.NewServeMux()

	h := api.NewHandlers(store, publisher)
	h.RegisterRoutes(mux)

	return mux
}

// envOrDefault returns the value of the given environment variable, or the
// default value if the variable is not set.
func envOrDefault(key, defaultVal string) string {
	if v := os.Getenv(key); v != "" {
		return v
	}
	return defaultVal
}
