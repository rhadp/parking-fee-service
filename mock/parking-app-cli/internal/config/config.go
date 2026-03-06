package config

import "os"

const (
	DefaultParkingFeeServiceURL = "http://localhost:8080"
	DefaultUpdateServiceAddr    = "localhost:50051"
	DefaultParkingAdaptorAddr   = "localhost:50052"
	DefaultDataBrokerAddr       = "localhost:55556"
)

// ParkingFeeServiceURL returns the configured URL for PARKING_FEE_SERVICE.
func ParkingFeeServiceURL() string {
	if v := os.Getenv("PARKING_FEE_SERVICE_URL"); v != "" {
		return v
	}
	return DefaultParkingFeeServiceURL
}

// UpdateServiceAddr returns the configured gRPC address for UPDATE_SERVICE.
func UpdateServiceAddr() string {
	if v := os.Getenv("UPDATE_SERVICE_ADDR"); v != "" {
		return v
	}
	return DefaultUpdateServiceAddr
}

// ParkingAdaptorAddr returns the configured gRPC address for PARKING_OPERATOR_ADAPTOR.
func ParkingAdaptorAddr() string {
	if v := os.Getenv("PARKING_ADAPTOR_ADDR"); v != "" {
		return v
	}
	return DefaultParkingAdaptorAddr
}
