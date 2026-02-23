package mqtt

import (
	"testing"
)

func TestNewClient(t *testing.T) {
	c := NewClient("tcp://localhost:1883", "test-client")
	if c == nil {
		t.Fatal("expected non-nil client")
	}
	if c.subs == nil {
		t.Error("expected initialized subs map")
	}
}

func TestClient_IsConnected_InitiallyFalse(t *testing.T) {
	c := NewClient("tcp://localhost:19999", "test-client")
	if c.IsConnected() {
		t.Error("expected IsConnected to be false before Connect")
	}
}

func TestClient_PublishBeforeConnect(t *testing.T) {
	c := NewClient("tcp://localhost:19999", "test-client")
	err := c.Publish("test/topic", []byte("hello"))
	if err == nil {
		t.Error("expected error when publishing before connect")
	}
}

func TestClient_SubscribeBeforeConnect(t *testing.T) {
	c := NewClient("tcp://localhost:19999", "test-client")
	// Subscribe queues the handler even without connection
	err := c.Subscribe("test/topic", func(topic string, payload []byte) {})
	if err == nil {
		t.Error("expected error when subscribing before connect (client not initialized)")
	}
}

func TestClient_DisconnectSafe(t *testing.T) {
	c := NewClient("tcp://localhost:19999", "test-client")
	// Disconnect should not panic even without a connection
	c.Disconnect()
	if c.IsConnected() {
		t.Error("expected IsConnected to be false after Disconnect")
	}
}

func TestClient_ConnectUnreachableBroker(t *testing.T) {
	c := NewClient("tcp://localhost:19999", "test-client")
	// Connect should not return error even if broker is unreachable
	// (because ConnectRetry handles retries in the background)
	err := c.Connect()
	if err != nil {
		t.Errorf("expected no error from Connect with unreachable broker, got %v", err)
	}
	c.Disconnect()
}
