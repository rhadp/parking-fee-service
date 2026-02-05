module github.com/sdv-parking-demo/backend

go 1.24.0

// Backend services for SDV Parking Demo System
// - parking-fee-service: Go service for parking operations
// - cloud-gateway: Go MQTT broker/router for vehicle-to-cloud communication

require (
	golang.org/x/net v0.47.0 // indirect
	golang.org/x/sys v0.38.0 // indirect
	golang.org/x/text v0.31.0 // indirect
	google.golang.org/genproto/googleapis/rpc v0.0.0-20251029180050-ab9386a59fda // indirect
	google.golang.org/grpc v1.78.0 // indirect
	google.golang.org/protobuf v1.36.11 // indirect
)
