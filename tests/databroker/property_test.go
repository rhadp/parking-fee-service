package databroker_test

import (
	"context"
	"testing"
	"time"

	kuksa "github.com/rhadp/parking-fee-service/gen/kuksa"
)

// TestPropertySignalCompleteness verifies that all 8 expected signals (5 standard
// + 3 custom) are present in the DATA_BROKER metadata with correct data types.
// Test Spec: TS-02-P1
// Requirement: 02-REQ-5.1, 02-REQ-5.2, 02-REQ-6.1, 02-REQ-6.2, 02-REQ-6.3
func TestPropertySignalCompleteness(t *testing.T) {
	skipIfTCPUnreachable(t)
	_, client := dialTCP(t)

	foundCount := 0
	for _, sig := range allSignals {
		t.Run(sig.Path, func(t *testing.T) {
			md := listMetadataOrFail(t, client, sig.Path)
			if len(md) == 0 {
				t.Errorf("signal not found: %s", sig.Path)
				return
			}
			if md[0].DataType != sig.DataType {
				t.Errorf("signal %s: expected type %v, got %v",
					sig.Path, sig.DataType, md[0].DataType)
			}
			foundCount++
		})
	}
	if foundCount != 8 {
		t.Errorf("expected 8 signals, found %d", foundCount)
	}
}

// TestPropertyWriteReadRoundtrip verifies that for any signal, setting a value
// and immediately getting it returns the same value (write-read idempotency).
// Test Spec: TS-02-P2
// Requirement: 02-REQ-8.1, 02-REQ-9.1
func TestPropertyWriteReadRoundtrip(t *testing.T) {
	skipIfTCPUnreachable(t)
	_, client := dialTCP(t)

	// Test values for each data type.
	type testCase struct {
		name  string
		path  string
		value *kuksa.Value
		check func(t *testing.T, dp *kuksa.Datapoint)
	}

	cases := []testCase{
		{
			name:  "bool/true",
			path:  "Vehicle.Cabin.Door.Row1.DriverSide.IsLocked",
			value: boolValue(true),
			check: func(t *testing.T, dp *kuksa.Datapoint) {
				if dp.GetValue().GetBool() != true {
					t.Errorf("expected true, got %v", dp.GetValue().GetBool())
				}
			},
		},
		{
			name:  "bool/false",
			path:  "Vehicle.Cabin.Door.Row1.DriverSide.IsOpen",
			value: boolValue(false),
			check: func(t *testing.T, dp *kuksa.Datapoint) {
				if dp.GetValue().GetBool() != false {
					t.Errorf("expected false, got %v", dp.GetValue().GetBool())
				}
			},
		},
		{
			name:  "float/0.0",
			path:  "Vehicle.Speed",
			value: floatValue(0.0),
			check: func(t *testing.T, dp *kuksa.Datapoint) {
				if dp.GetValue().GetFloat() != 0.0 {
					t.Errorf("expected 0.0, got %v", dp.GetValue().GetFloat())
				}
			},
		},
		{
			name:  "float/50.0",
			path:  "Vehicle.Speed",
			value: floatValue(50.0),
			check: func(t *testing.T, dp *kuksa.Datapoint) {
				if dp.GetValue().GetFloat() != 50.0 {
					t.Errorf("expected 50.0, got %v", dp.GetValue().GetFloat())
				}
			},
		},
		{
			name:  "float/999.9",
			path:  "Vehicle.Speed",
			value: floatValue(999.9),
			check: func(t *testing.T, dp *kuksa.Datapoint) {
				if got := dp.GetValue().GetFloat(); got != 999.9 {
					// Float comparison with tolerance for IEEE 754.
					diff := got - 999.9
					if diff < -0.1 || diff > 0.1 {
						t.Errorf("expected ~999.9, got %v", got)
					}
				}
			},
		},
		{
			name:  "double/48.1351",
			path:  "Vehicle.CurrentLocation.Latitude",
			value: doubleValue(48.1351),
			check: func(t *testing.T, dp *kuksa.Datapoint) {
				if dp.GetValue().GetDouble() != 48.1351 {
					t.Errorf("expected 48.1351, got %v", dp.GetValue().GetDouble())
				}
			},
		},
		{
			name:  "double/-122.4194",
			path:  "Vehicle.CurrentLocation.Longitude",
			value: doubleValue(-122.4194),
			check: func(t *testing.T, dp *kuksa.Datapoint) {
				if dp.GetValue().GetDouble() != -122.4194 {
					t.Errorf("expected -122.4194, got %v", dp.GetValue().GetDouble())
				}
			},
		},
		{
			name:  "string/json_command",
			path:  "Vehicle.Command.Door.Lock",
			value: stringValue(`{"command_id":"x"}`),
			check: func(t *testing.T, dp *kuksa.Datapoint) {
				expected := `{"command_id":"x"}`
				if dp.GetValue().GetString_() != expected {
					t.Errorf("expected %q, got %q", expected, dp.GetValue().GetString_())
				}
			},
		},
		{
			name:  "string/empty_json",
			path:  "Vehicle.Command.Door.Response",
			value: stringValue("{}"),
			check: func(t *testing.T, dp *kuksa.Datapoint) {
				if dp.GetValue().GetString_() != "{}" {
					t.Errorf("expected %q, got %q", "{}", dp.GetValue().GetString_())
				}
			},
		},
		{
			name:  "bool/parking_session",
			path:  "Vehicle.Parking.SessionActive",
			value: boolValue(true),
			check: func(t *testing.T, dp *kuksa.Datapoint) {
				if dp.GetValue().GetBool() != true {
					t.Errorf("expected true, got %v", dp.GetValue().GetBool())
				}
			},
		},
	}

	for _, tc := range cases {
		t.Run(tc.name, func(t *testing.T) {
			publishValue(t, client, tc.path, tc.value)
			dp := getValueOrFail(t, client, tc.path)
			tc.check(t, dp)
		})
	}
}

// TestPropertyCrossTransportEquivalence verifies that for any signal, the value
// read via TCP equals the value read via UDS after a write on either transport.
// Test Spec: TS-02-P3
// Requirement: 02-REQ-4.1, 02-REQ-9.2
func TestPropertyCrossTransportEquivalence(t *testing.T) {
	skipIfTCPUnreachable(t)
	skipIfUDSUnreachable(t)
	_, tcpClient := dialTCP(t)
	_, udsClient := dialUDS(t)

	type testCase struct {
		name  string
		path  string
		val1  *kuksa.Value // Written via TCP
		val2  *kuksa.Value // Written via UDS
		check func(t *testing.T, dp *kuksa.Datapoint, expected *kuksa.Value)
	}

	cases := []testCase{
		{
			name: "bool",
			path: "Vehicle.Cabin.Door.Row1.DriverSide.IsLocked",
			val1: boolValue(true),
			val2: boolValue(false),
			check: func(t *testing.T, dp *kuksa.Datapoint, expected *kuksa.Value) {
				if dp.GetValue().GetBool() != expected.GetBool() {
					t.Errorf("expected %v, got %v", expected.GetBool(), dp.GetValue().GetBool())
				}
			},
		},
		{
			name: "float",
			path: "Vehicle.Speed",
			val1: floatValue(60.0),
			val2: floatValue(120.0),
			check: func(t *testing.T, dp *kuksa.Datapoint, expected *kuksa.Value) {
				if dp.GetValue().GetFloat() != expected.GetFloat() {
					t.Errorf("expected %v, got %v", expected.GetFloat(), dp.GetValue().GetFloat())
				}
			},
		},
		{
			name: "double",
			path: "Vehicle.CurrentLocation.Latitude",
			val1: doubleValue(52.5200),
			val2: doubleValue(40.7128),
			check: func(t *testing.T, dp *kuksa.Datapoint, expected *kuksa.Value) {
				if dp.GetValue().GetDouble() != expected.GetDouble() {
					t.Errorf("expected %v, got %v", expected.GetDouble(), dp.GetValue().GetDouble())
				}
			},
		},
		{
			name: "string",
			path: "Vehicle.Command.Door.Lock",
			val1: stringValue(`{"action":"lock"}`),
			val2: stringValue(`{"action":"unlock"}`),
			check: func(t *testing.T, dp *kuksa.Datapoint, expected *kuksa.Value) {
				if dp.GetValue().GetString_() != expected.GetString_() {
					t.Errorf("expected %q, got %q", expected.GetString_(), dp.GetValue().GetString_())
				}
			},
		},
	}

	for _, tc := range cases {
		t.Run(tc.name, func(t *testing.T) {
			// Write via TCP, read via both.
			publishValue(t, tcpClient, tc.path, tc.val1)
			dpTCP := getValueOrFail(t, tcpClient, tc.path)
			dpUDS := getValueOrFail(t, udsClient, tc.path)
			tc.check(t, dpTCP, tc.val1)
			tc.check(t, dpUDS, tc.val1)

			// Write via UDS, read via both.
			publishValue(t, udsClient, tc.path, tc.val2)
			dpTCP = getValueOrFail(t, tcpClient, tc.path)
			dpUDS = getValueOrFail(t, udsClient, tc.path)
			tc.check(t, dpTCP, tc.val2)
			tc.check(t, dpUDS, tc.val2)
		})
	}
}

// TestPropertySubscriptionDelivery verifies that for any active subscription,
// a value change is delivered to the subscriber.
// Test Spec: TS-02-P4
// Requirement: 02-REQ-10.1
func TestPropertySubscriptionDelivery(t *testing.T) {
	skipIfTCPUnreachable(t)

	type testCase struct {
		name  string
		path  string
		value *kuksa.Value
		check func(t *testing.T, entry *kuksa.Datapoint)
	}

	cases := []testCase{
		{
			name:  "bool",
			path:  "Vehicle.Parking.SessionActive",
			value: boolValue(true),
			check: func(t *testing.T, entry *kuksa.Datapoint) {
				if entry.GetValue().GetBool() != true {
					t.Errorf("expected true, got %v", entry.GetValue().GetBool())
				}
			},
		},
		{
			name:  "float",
			path:  "Vehicle.Speed",
			value: floatValue(88.8),
			check: func(t *testing.T, entry *kuksa.Datapoint) {
				if entry.GetValue().GetFloat() != 88.8 {
					t.Errorf("expected 88.8, got %v", entry.GetValue().GetFloat())
				}
			},
		},
		{
			name:  "string",
			path:  "Vehicle.Command.Door.Lock",
			value: stringValue(`{"test":"sub"}`),
			check: func(t *testing.T, entry *kuksa.Datapoint) {
				if entry.GetValue().GetString_() != `{"test":"sub"}` {
					t.Errorf("expected %q, got %q", `{"test":"sub"}`, entry.GetValue().GetString_())
				}
			},
		},
	}

	for _, tc := range cases {
		t.Run(tc.name, func(t *testing.T) {
			_, subClient := dialTCP(t)
			_, pubClient := dialTCP(t)

			ctx, cancel := context.WithTimeout(context.Background(), 10*time.Second)
			defer cancel()

			stream, err := subClient.Subscribe(ctx, &kuksa.SubscribeRequest{
				SignalPaths: []string{tc.path},
			})
			if err != nil {
				t.Fatalf("failed to subscribe to %s: %v", tc.path, err)
			}

			// Drain any initial notification.
			drainStream(stream, 1*time.Second)

			// Publish a new value.
			publishValue(t, pubClient, tc.path, tc.value)

			// Wait for the subscription notification.
			received := make(chan *kuksa.SubscribeResponse, 1)
			errCh := make(chan error, 1)
			go func() {
				resp, recvErr := stream.Recv()
				if recvErr != nil {
					errCh <- recvErr
					return
				}
				received <- resp
			}()

			select {
			case resp := <-received:
				entry, ok := resp.Entries[tc.path]
				if !ok {
					t.Errorf("subscription response missing %s", tc.path)
					return
				}
				tc.check(t, entry)
			case recvErr := <-errCh:
				t.Fatalf("subscription recv error: %v", recvErr)
			case <-ctx.Done():
				t.Fatalf("subscription timed out for %s", tc.path)
			}
		})
	}
}
