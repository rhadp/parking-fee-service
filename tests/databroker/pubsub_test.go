package databroker_test

// pubsub_test.go — subscription notification tests.
//
// These tests verify that the DATA_BROKER delivers signal update notifications
// to subscribers over TCP and UDS, and across transports.
//
// Subscribe request format (kuksa.val.v2.VAL/Subscribe):
//   {"entries": [{"signal_id": {"path": "Vehicle.Speed"}}]}
//
// Tests: TS-02-10, TS-02-11.
// Requirements: 02-REQ-10.1, 02-REQ-4.1.

import (
	"strings"
	"testing"
	"time"
)

// TestSubscriptionViaTCP verifies that a TCP subscriber receives a notification
// when a signal value changes (TS-02-10, 02-REQ-10.1).
//
// Approach: open a grpcurl Subscribe stream with a timeout, fire a PublishValue
// from a second client, wait for the output to contain the new value.
func TestSubscriptionViaTCP(t *testing.T) {
	requireTCPReachable(t)
	requireGrpcurl(t)

	signal := "Vehicle.Cabin.Door.Row1.DriverSide.IsLocked"
	// Reset to false first so the update to true is a genuine change.
	grpcurlTCP(t, "kuksa.val.v2.VAL/PublishValue",
		`{"signal_id": {"path": "`+signal+`"}, "data_point": {"value": {"bool": false}}}`)

	setter := func() {
		grpcurlTCP(t, "kuksa.val.v2.VAL/PublishValue",
			`{"signal_id": {"path": "`+signal+`"}, "data_point": {"value": {"bool": true}}}`)
	}

	out := grpcurlSubscribeTCP(t, signal, 4*time.Second, setter)
	if !strings.Contains(out, "true") {
		t.Errorf("expected subscription update with 'true' for %s, got:\n%s", signal, out)
	}
}

// TestSubscriptionCrossTransport verifies that a UDS subscriber receives a
// notification when a signal is set via TCP (TS-02-11, 02-REQ-10.1, 02-REQ-4.1).
func TestSubscriptionCrossTransport(t *testing.T) {
	requireTCPReachable(t)
	requireUDSSocket(t)
	requireGrpcurl(t)

	signal := "Vehicle.Parking.SessionActive"
	// Reset to false first.
	grpcurlTCP(t, "kuksa.val.v2.VAL/PublishValue",
		`{"signal_id": {"path": "`+signal+`"}, "data_point": {"value": {"bool": false}}}`)

	setter := func() {
		grpcurlTCP(t, "kuksa.val.v2.VAL/PublishValue",
			`{"signal_id": {"path": "`+signal+`"}, "data_point": {"value": {"bool": true}}}`)
	}

	// Subscribe via UDS, publish via TCP (setter).
	out := grpcurlSubscribeUDS(t, signal, 4*time.Second, setter)
	if !strings.Contains(out, "true") {
		t.Errorf("expected UDS subscriber to receive 'true' update set via TCP for %s, got:\n%s",
			signal, out)
	}
}
