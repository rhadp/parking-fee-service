package mqtt

import (
	"fmt"
	"log"
	"sync"
	"time"

	pahomqtt "github.com/eclipse/paho.mqtt.golang"
)

// MessageHandler is a callback function for incoming MQTT messages.
type MessageHandler func(topic string, payload []byte)

// Client wraps the paho MQTT client with connection management, automatic
// reconnection with exponential backoff, and simplified publish/subscribe.
type Client struct {
	client  pahomqtt.Client
	opts    *pahomqtt.ClientOptions
	mu      sync.Mutex
	subs    map[string]MessageHandler
	connected bool
}

// NewClient creates a new MQTT client configured with the given broker URL and
// client ID. The client is not connected until Connect() is called.
func NewClient(brokerURL, clientID string) *Client {
	c := &Client{
		subs: make(map[string]MessageHandler),
	}

	opts := pahomqtt.NewClientOptions().
		AddBroker(brokerURL).
		SetClientID(clientID).
		SetAutoReconnect(true).
		SetMaxReconnectInterval(30 * time.Second).
		SetConnectRetry(true).
		SetConnectRetryInterval(2 * time.Second).
		SetOrderMatters(false).
		SetOnConnectHandler(func(client pahomqtt.Client) {
			log.Printf("MQTT connected to %s", brokerURL)
			c.mu.Lock()
			c.connected = true
			// Re-subscribe after reconnection
			subs := make(map[string]MessageHandler, len(c.subs))
			for k, v := range c.subs {
				subs[k] = v
			}
			c.mu.Unlock()

			for topic, handler := range subs {
				h := handler
				token := client.Subscribe(topic, 1, func(_ pahomqtt.Client, msg pahomqtt.Message) {
					h(msg.Topic(), msg.Payload())
				})
				token.Wait()
				if token.Error() != nil {
					log.Printf("MQTT re-subscribe to %s failed: %v", topic, token.Error())
				} else {
					log.Printf("MQTT re-subscribed to %s", topic)
				}
			}
		}).
		SetConnectionLostHandler(func(_ pahomqtt.Client, err error) {
			log.Printf("MQTT connection lost: %v", err)
			c.mu.Lock()
			c.connected = false
			c.mu.Unlock()
		})

	c.opts = opts
	return c
}

// Connect initiates the MQTT connection. With ConnectRetry enabled, this
// returns immediately and connects in the background if the broker is
// unreachable. The REST API can start while MQTT connects asynchronously.
func (c *Client) Connect() error {
	c.client = pahomqtt.NewClient(c.opts)
	token := c.client.Connect()

	// Wait briefly for connection, but don't block indefinitely.
	// ConnectRetry handles retries in the background.
	if token.WaitTimeout(5 * time.Second) {
		if token.Error() != nil {
			log.Printf("MQTT initial connect failed (will retry in background): %v", token.Error())
			return nil // non-fatal: REST API starts anyway
		}
	} else {
		log.Printf("MQTT connect timed out (will retry in background)")
	}

	return nil
}

// IsConnected returns true if the MQTT client is currently connected.
func (c *Client) IsConnected() bool {
	c.mu.Lock()
	defer c.mu.Unlock()
	return c.connected
}

// Publish sends a message to the specified MQTT topic with QoS 1.
// Returns an error if the client is not connected or the publish fails.
func (c *Client) Publish(topic string, payload []byte) error {
	if c.client == nil {
		return fmt.Errorf("MQTT client not initialized")
	}
	if !c.client.IsConnectionOpen() {
		return fmt.Errorf("MQTT client not connected")
	}

	token := c.client.Publish(topic, 1, false, payload)
	token.Wait()
	return token.Error()
}

// Subscribe registers a handler for messages on the given MQTT topic with QoS 1.
// The handler is stored so it can be re-subscribed on reconnection.
func (c *Client) Subscribe(topic string, handler MessageHandler) error {
	c.mu.Lock()
	c.subs[topic] = handler
	c.mu.Unlock()

	if c.client == nil {
		return fmt.Errorf("MQTT client not initialized")
	}
	if !c.client.IsConnectionOpen() {
		// Subscription will be applied when connection succeeds
		log.Printf("MQTT not connected; subscription to %s queued for reconnect", topic)
		return nil
	}

	token := c.client.Subscribe(topic, 1, func(_ pahomqtt.Client, msg pahomqtt.Message) {
		handler(msg.Topic(), msg.Payload())
	})
	token.Wait()
	return token.Error()
}

// Disconnect cleanly disconnects from the MQTT broker.
func (c *Client) Disconnect() {
	if c.client != nil {
		c.client.Disconnect(1000)
	}
	c.mu.Lock()
	c.connected = false
	c.mu.Unlock()
}
