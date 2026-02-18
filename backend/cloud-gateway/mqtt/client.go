// Package mqtt provides the MQTT client for CLOUD_GATEWAY. It connects to
// Mosquitto, subscribes to vehicle response/telemetry/registration topics,
// and publishes lock/unlock commands and status requests.
package mqtt

import (
	"encoding/json"
	"fmt"
	"log"
	"strings"
	"time"

	pahomqtt "github.com/eclipse/paho.mqtt.golang"

	"github.com/rhadp/parking-fee-service/backend/cloud-gateway/messages"
	"github.com/rhadp/parking-fee-service/backend/cloud-gateway/state"
)

// Client wraps a Paho MQTT client and provides publish/subscribe methods
// for the CLOUD_GATEWAY MQTT protocol.
type Client struct {
	paho pahomqtt.Client
	store *state.Store
}

// ClientOption configures a Client.
type ClientOption func(*clientConfig)

type clientConfig struct {
	clientID        string
	keepAlive       time.Duration
	connectTimeout  time.Duration
	onConnectFunc   func(pahomqtt.Client)
}

func defaultConfig() *clientConfig {
	return &clientConfig{
		clientID:       "cloud-gateway",
		keepAlive:      30 * time.Second,
		connectTimeout: 10 * time.Second,
	}
}

// WithClientID sets the MQTT client ID.
func WithClientID(id string) ClientOption {
	return func(c *clientConfig) {
		c.clientID = id
	}
}

// WithConnectTimeout sets the MQTT connection timeout.
func WithConnectTimeout(d time.Duration) ClientOption {
	return func(c *clientConfig) {
		c.connectTimeout = d
	}
}

// NewClient creates a new MQTT client connected to the given broker address.
// It subscribes to all required topics and begins processing incoming messages.
//
// The broker address should be in the form "host:port" (e.g. "localhost:1883").
func NewClient(brokerAddr string, store *state.Store, opts ...ClientOption) (*Client, error) {
	cfg := defaultConfig()
	for _, opt := range opts {
		opt(cfg)
	}

	c := &Client{
		store: store,
	}

	pahoOpts := pahomqtt.NewClientOptions().
		AddBroker("tcp://" + brokerAddr).
		SetClientID(cfg.clientID).
		SetKeepAlive(cfg.keepAlive).
		SetConnectTimeout(cfg.connectTimeout).
		SetAutoReconnect(true).
		SetMaxReconnectInterval(30 * time.Second).
		SetCleanSession(true).
		SetOrderMatters(false).
		SetOnConnectHandler(func(client pahomqtt.Client) {
			log.Printf("mqtt: connected to broker at %s", brokerAddr)
			// Re-subscribe on every (re)connect to ensure subscriptions
			// survive broker restarts (03-REQ-2.E1).
			c.subscribe(client)
			if cfg.onConnectFunc != nil {
				cfg.onConnectFunc(client)
			}
		}).
		SetConnectionLostHandler(func(_ pahomqtt.Client, err error) {
			log.Printf("mqtt: connection lost: %v (auto-reconnect enabled)", err)
		}).
		SetReconnectingHandler(func(_ pahomqtt.Client, _ *pahomqtt.ClientOptions) {
			log.Printf("mqtt: attempting to reconnect to broker")
		})

	c.paho = pahomqtt.NewClient(pahoOpts)

	token := c.paho.Connect()
	if !token.WaitTimeout(cfg.connectTimeout) {
		return nil, fmt.Errorf("mqtt: connection timed out after %v", cfg.connectTimeout)
	}
	if err := token.Error(); err != nil {
		return nil, fmt.Errorf("mqtt: connection failed: %w", err)
	}

	return c, nil
}

// subscribe registers all MQTT topic subscriptions.
// Called on initial connect and on every reconnect.
func (c *Client) subscribe(client pahomqtt.Client) {
	subs := map[string]byte{
		messages.SubCommandResponses: 2, // QoS 2
		messages.SubStatusResponse:   2, // QoS 2
		messages.SubTelemetry:        0, // QoS 0
		messages.SubRegistration:     2, // QoS 2
	}

	token := client.SubscribeMultiple(subs, c.handleMessage)
	go func() {
		if token.WaitTimeout(10 * time.Second) {
			if err := token.Error(); err != nil {
				log.Printf("mqtt: subscribe error: %v", err)
			} else {
				log.Printf("mqtt: subscribed to %d topics", len(subs))
			}
		} else {
			log.Printf("mqtt: subscribe timed out")
		}
	}()
}

// handleMessage routes incoming MQTT messages to the appropriate handler.
func (c *Client) handleMessage(_ pahomqtt.Client, msg pahomqtt.Message) {
	topic := msg.Topic()
	payload := msg.Payload()

	// Extract the VIN from the topic: vehicles/{vin}/{suffix}
	vin, suffix, ok := parseTopic(topic)
	if !ok {
		log.Printf("mqtt: ignoring message on unrecognized topic %q", topic)
		return
	}

	switch suffix {
	case "command_responses":
		c.handleCommandResponse(vin, payload)
	case "telemetry":
		c.handleTelemetry(vin, payload)
	case "registration":
		c.handleRegistration(vin, payload)
	case "status_response":
		c.handleStatusResponse(vin, payload)
	default:
		log.Printf("mqtt: ignoring message with unknown suffix %q on topic %q", suffix, topic)
	}
}

// PublishCommand publishes a lock/unlock command to the vehicle's command topic.
// It implements the api.MQTTPublisher interface.
func (c *Client) PublishCommand(vin, commandID, cmdType string) error {
	msg := messages.CommandMessage{
		CommandID: commandID,
		Type:      messages.CommandType(cmdType),
		Timestamp: time.Now().Unix(),
	}

	payload, err := json.Marshal(msg)
	if err != nil {
		return fmt.Errorf("mqtt: marshaling command: %w", err)
	}

	topic := messages.TopicFor(messages.TopicCommands, vin)
	token := c.paho.Publish(topic, 2, false, payload)

	if !token.WaitTimeout(5 * time.Second) {
		return fmt.Errorf("mqtt: publish to %s timed out", topic)
	}
	if err := token.Error(); err != nil {
		return fmt.Errorf("mqtt: publish to %s failed: %w", topic, err)
	}

	log.Printf("mqtt: published %s command %s to %s", cmdType, commandID, topic)
	return nil
}

// Disconnect cleanly disconnects from the MQTT broker.
func (c *Client) Disconnect() {
	c.paho.Disconnect(1000) // wait up to 1s for in-flight messages
	log.Printf("mqtt: disconnected")
}

// IsConnected returns true if the MQTT client is currently connected.
func (c *Client) IsConnected() bool {
	return c.paho.IsConnected()
}

// parseTopic extracts the VIN and suffix from an MQTT topic.
// Expected format: "vehicles/{vin}/{suffix}"
// Returns ("", "", false) if the topic does not match.
func parseTopic(topic string) (vin, suffix string, ok bool) {
	parts := strings.SplitN(topic, "/", 3)
	if len(parts) != 3 || parts[0] != "vehicles" {
		return "", "", false
	}
	return parts[1], parts[2], true
}
