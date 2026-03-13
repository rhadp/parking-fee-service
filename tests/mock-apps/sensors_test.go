package mockapps

import (
	"context"
	"math"
	"os"
	"testing"
	"time"

	kuksav2 "github.com/parking-fee-service/proto/kuksa/val/v2"
	"google.golang.org/grpc"
	"google.golang.org/grpc/credentials/insecure"
)

// readDoubleSignal reads a double-typed VSS signal from DATA_BROKER.
func readDoubleSignal(t *testing.T, conn *grpc.ClientConn, signalPath string) float64 {
	t.Helper()
	client := kuksav2.NewVALClient(conn)
	ctx, cancel := context.WithTimeout(context.Background(), 5*time.Second)
	defer cancel()
	resp, err := client.GetValue(ctx, &kuksav2.GetValueRequest{
		SignalId: &kuksav2.SignalID{
			Signal: &kuksav2.SignalID_Path{Path: signalPath},
		},
	})
	if err != nil {
		t.Fatalf("GetValue(%s) failed: %v", signalPath, err)
	}
	dp := resp.GetDataPoint()
	if dp == nil {
		t.Fatalf("GetValue(%s) returned nil datapoint", signalPath)
	}
	v := dp.GetValue()
	if v == nil {
		t.Fatalf("GetValue(%s) returned nil value", signalPath)
	}
	tv, ok := v.GetTypedValue().(*kuksav2.Value_Double)
	if !ok {
		t.Fatalf("GetValue(%s) returned unexpected type: %T", signalPath, v.GetTypedValue())
	}
	return tv.Double
}

// readFloatSignal reads a float-typed VSS signal from DATA_BROKER.
func readFloatSignal(t *testing.T, conn *grpc.ClientConn, signalPath string) float32 {
	t.Helper()
	client := kuksav2.NewVALClient(conn)
	ctx, cancel := context.WithTimeout(context.Background(), 5*time.Second)
	defer cancel()
	resp, err := client.GetValue(ctx, &kuksav2.GetValueRequest{
		SignalId: &kuksav2.SignalID{
			Signal: &kuksav2.SignalID_Path{Path: signalPath},
		},
	})
	if err != nil {
		t.Fatalf("GetValue(%s) failed: %v", signalPath, err)
	}
	dp := resp.GetDataPoint()
	if dp == nil {
		t.Fatalf("GetValue(%s) returned nil datapoint", signalPath)
	}
	v := dp.GetValue()
	if v == nil {
		t.Fatalf("GetValue(%s) returned nil value", signalPath)
	}
	tv, ok := v.GetTypedValue().(*kuksav2.Value_Float)
	if !ok {
		t.Fatalf("GetValue(%s) returned unexpected type: %T", signalPath, v.GetTypedValue())
	}
	return tv.Float
}

// readBoolSignal reads a bool-typed VSS signal from DATA_BROKER.
func readBoolSignal(t *testing.T, conn *grpc.ClientConn, signalPath string) bool {
	t.Helper()
	client := kuksav2.NewVALClient(conn)
	ctx, cancel := context.WithTimeout(context.Background(), 5*time.Second)
	defer cancel()
	resp, err := client.GetValue(ctx, &kuksav2.GetValueRequest{
		SignalId: &kuksav2.SignalID{
			Signal: &kuksav2.SignalID_Path{Path: signalPath},
		},
	})
	if err != nil {
		t.Fatalf("GetValue(%s) failed: %v", signalPath, err)
	}
	dp := resp.GetDataPoint()
	if dp == nil {
		t.Fatalf("GetValue(%s) returned nil datapoint", signalPath)
	}
	v := dp.GetValue()
	if v == nil {
		t.Fatalf("GetValue(%s) returned nil value", signalPath)
	}
	tv, ok := v.GetTypedValue().(*kuksav2.Value_Bool)
	if !ok {
		t.Fatalf("GetValue(%s) returned unexpected type: %T", signalPath, v.GetTypedValue())
	}
	return tv.Bool
}

// brokerConn returns a gRPC connection to DATA_BROKER.
func brokerConn(t *testing.T) *grpc.ClientConn {
	t.Helper()
	addr := brokerGRPCAddr()
	conn, err := grpc.NewClient(addr, grpc.WithTransportCredentials(insecure.NewCredentials()))
	if err != nil {
		t.Fatalf("grpc.NewClient(%s) failed: %v", addr, err)
	}
	t.Cleanup(func() { conn.Close() })
	return conn
}

// sensorEnv returns the environment for a sensor process with the given broker address.
func sensorEnv(brokerAddr string) []string {
	env := os.Environ()
	// Remove any existing DATA_BROKER_ADDR
	filtered := env[:0]
	for _, e := range env {
		if len(e) < 16 || e[:16] != "DATA_BROKER_ADDR" {
			filtered = append(filtered, e)
		}
	}
	return append(filtered, "DATA_BROKER_ADDR=http://"+brokerAddr)
}

// TS-09-1: Location Sensor Writes Lat/Lon
// Requirement: 09-REQ-1.1
func TestLocationSensorWritesToBroker(t *testing.T) {
	skipIfBrokerUnavailable(t)

	binPath := buildRustBinary(t, "location-sensor")
	addr := brokerGRPCAddr()
	env := sensorEnv(addr)

	const lat = 48.1351
	const lon = 11.5820

	code, _, stderr := runSensor(t, binPath, env,
		"--lat=48.1351",
		"--lon=11.5820",
		"--broker-addr=http://"+addr,
	)
	if code != 0 {
		t.Fatalf("location-sensor exited with code %d; stderr: %s", code, stderr)
	}

	conn := brokerConn(t)

	gotLat := readDoubleSignal(t, conn, "Vehicle.CurrentLocation.Latitude")
	if math.Abs(gotLat-lat) > 1e-6 {
		t.Errorf("Latitude: want %v, got %v", lat, gotLat)
	}

	gotLon := readDoubleSignal(t, conn, "Vehicle.CurrentLocation.Longitude")
	if math.Abs(gotLon-lon) > 1e-6 {
		t.Errorf("Longitude: want %v, got %v", lon, gotLon)
	}
}

// TS-09-2: Speed Sensor Writes Speed
// Requirement: 09-REQ-1.2
func TestSpeedSensorWritesToBroker(t *testing.T) {
	skipIfBrokerUnavailable(t)

	binPath := buildRustBinary(t, "speed-sensor")
	addr := brokerGRPCAddr()

	const speed float32 = 60.5

	code, _, stderr := runSensor(t, binPath, sensorEnv(addr),
		"--speed=60.5",
		"--broker-addr=http://"+addr,
	)
	if code != 0 {
		t.Fatalf("speed-sensor exited with code %d; stderr: %s", code, stderr)
	}

	conn := brokerConn(t)
	gotSpeed := readFloatSignal(t, conn, "Vehicle.Speed")
	if math.Abs(float64(gotSpeed-speed)) > 0.01 {
		t.Errorf("Speed: want %v, got %v", speed, gotSpeed)
	}
}

// TS-09-3: Door Sensor Writes Open/Closed
// Requirement: 09-REQ-1.3
func TestDoorSensorWritesToBroker(t *testing.T) {
	skipIfBrokerUnavailable(t)

	binPath := buildRustBinary(t, "door-sensor")
	addr := brokerGRPCAddr()
	conn := brokerConn(t)

	const signalPath = "Vehicle.Cabin.Door.Row1.DriverSide.IsOpen"

	// Test --open writes true.
	code, _, stderr := runSensor(t, binPath, sensorEnv(addr),
		"--open",
		"--broker-addr=http://"+addr,
	)
	if code != 0 {
		t.Fatalf("door-sensor --open exited with code %d; stderr: %s", code, stderr)
	}
	if got := readBoolSignal(t, conn, signalPath); !got {
		t.Errorf("after --open: want IsOpen=true, got false")
	}

	// Test --closed writes false.
	code, _, stderr = runSensor(t, binPath, sensorEnv(addr),
		"--closed",
		"--broker-addr=http://"+addr,
	)
	if code != 0 {
		t.Fatalf("door-sensor --closed exited with code %d; stderr: %s", code, stderr)
	}
	if got := readBoolSignal(t, conn, signalPath); got {
		t.Errorf("after --closed: want IsOpen=false, got true")
	}
}

