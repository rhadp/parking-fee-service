// Package config provides environment variable and flag parsing for parking-app-cli.
package config

import "os"

const (
	DefaultParkingFeeServiceURL = "http://localhost:8080"
	DefaultUpdateServiceAddr    = "localhost:50051"
	DefaultParkingAdaptorAddr   = "localhost:50052"
	DefaultDataBrokerAddr       = "localhost:55556"
)

// GetEnv returns the value of the environment variable named by the key,
// or the fallback value if the variable is not set.
func GetEnv(key, fallback string) string {
	if v := os.Getenv(key); v != "" {
		return v
	}
	return fallback
}

// ParkingFeeServiceURL returns the configured PARKING_FEE_SERVICE URL.
func ParkingFeeServiceURL() string {
	return GetEnv("PARKING_FEE_SERVICE_URL", DefaultParkingFeeServiceURL)
}

// UpdateServiceAddr returns the configured UPDATE_SERVICE gRPC address.
func UpdateServiceAddr() string {
	return GetEnv("UPDATE_SERVICE_ADDR", DefaultUpdateServiceAddr)
}

// ParkingAdaptorAddr returns the configured PARKING_OPERATOR_ADAPTOR gRPC address.
func ParkingAdaptorAddr() string {
	return GetEnv("PARKING_ADAPTOR_ADDR", DefaultParkingAdaptorAddr)
}
