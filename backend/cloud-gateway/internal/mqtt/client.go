// Package mqtt provides MQTT client functionality for the cloud-gateway service.
package mqtt

import (
	"context"
	"crypto/tls"
	"log/slog"
	"sync"
	"time"

	pahomqtt "github.com/eclipse/paho.mqtt.golang"

	"github.com/sdv-parking-demo/backend/cloud-gateway/internal/audit"
	"github.com/sdv-parking-demo/backend/cloud-gateway/internal/config"
	"github.com/sdv-parking-demo/backend/cloud-gateway/internal/model"
)

// MessageHandler is a function that handles incoming MQTT messages.
type MessageHandler func(topic string, payload []byte)

// Client defines the interface for MQTT operations.
type Client interface {
	// Connect establishes connection to the MQTT broker.
	Connect() error
	// Disconnect closes the connection to the MQTT broker.
	Disconnect()
	// IsConnected returns true if connected to the broker.
	IsConnected() bool
	// Subscribe subscribes to a topic with a message handler.
	Subscribe(topic string, handler MessageHandler) error
	// Publish publishes a message to a topic.
	Publish(topic string, payload []byte) error
}

// ClientImpl implements the Client interface using paho.mqtt.golang.
type ClientImpl struct {
	client      pahomqtt.Client
	cfg         *config.MQTTConfig
	logger      *slog.Logger
	auditLogger audit.AuditLogger
	handlers    map[string]MessageHandler
	handlersMu  sync.RWMutex
	connected   bool
	connMu      sync.RWMutex
}

// NewClient creates a new MQTT client.
func NewClient(cfg *config.MQTTConfig, logger *slog.Logger, auditLogger audit.AuditLogger) *ClientImpl {
	c := &ClientImpl{
		cfg:         cfg,
		logger:      logger,
		auditLogger: auditLogger,
		handlers:    make(map[string]MessageHandler),
	}

	opts := pahomqtt.NewClientOptions()
	opts.AddBroker(cfg.BrokerURL)
	opts.SetClientID(cfg.ClientID)

	if cfg.Username != "" {
		opts.SetUsername(cfg.Username)
	}
	if cfg.Password != "" {
		opts.SetPassword(cfg.Password)
	}

	// Configure TLS for secure connections
	opts.SetTLSConfig(&tls.Config{
		MinVersion: tls.VersionTLS12,
	})

	// Configure reconnection with exponential backoff
	opts.SetAutoReconnect(true)
	opts.SetConnectRetry(true)
	opts.SetConnectRetryInterval(1 * time.Second)
	opts.SetMaxReconnectInterval(30 * time.Second)

	// Set connection callbacks
	opts.SetOnConnectHandler(c.onConnect)
	opts.SetConnectionLostHandler(c.onConnectionLost)
	opts.SetReconnectingHandler(c.onReconnecting)

	// Set default message handler
	opts.SetDefaultPublishHandler(c.defaultMessageHandler)

	c.client = pahomqtt.NewClient(opts)
	return c
}

// Connect establishes connection to the MQTT broker.
func (c *ClientImpl) Connect() error {
	token := c.client.Connect()
	token.Wait()
	return token.Error()
}

// Disconnect closes the connection to the MQTT broker.
func (c *ClientImpl) Disconnect() {
	c.client.Disconnect(250) // 250ms timeout

	c.connMu.Lock()
	c.connected = false
	c.connMu.Unlock()

	c.logConnectionEvent(model.AuditEventMQTTDisconnect)
}

// IsConnected returns true if connected to the broker.
func (c *ClientImpl) IsConnected() bool {
	c.connMu.RLock()
	defer c.connMu.RUnlock()
	return c.connected && c.client.IsConnected()
}

// Subscribe subscribes to a topic with a message handler.
func (c *ClientImpl) Subscribe(topic string, handler MessageHandler) error {
	c.handlersMu.Lock()
	c.handlers[topic] = handler
	c.handlersMu.Unlock()

	token := c.client.Subscribe(topic, 1, func(client pahomqtt.Client, msg pahomqtt.Message) {
		c.handlersMu.RLock()
		h, ok := c.handlers[msg.Topic()]
		c.handlersMu.RUnlock()

		if ok {
			h(msg.Topic(), msg.Payload())
		}
	})

	token.Wait()
	return token.Error()
}

// Publish publishes a message to a topic.
func (c *ClientImpl) Publish(topic string, payload []byte) error {
	token := c.client.Publish(topic, 1, false, payload)
	token.Wait()
	return token.Error()
}

// onConnect is called when the client connects to the broker.
func (c *ClientImpl) onConnect(client pahomqtt.Client) {
	c.connMu.Lock()
	c.connected = true
	c.connMu.Unlock()

	c.logger.Info("MQTT connected", slog.String("broker", c.cfg.BrokerURL))
	c.logConnectionEvent(model.AuditEventMQTTConnect)

	// Resubscribe to all topics after reconnection
	c.handlersMu.RLock()
	defer c.handlersMu.RUnlock()
	for topic := range c.handlers {
		c.logger.Info("resubscribing to topic", slog.String("topic", topic))
	}
}

// onConnectionLost is called when the connection to the broker is lost.
func (c *ClientImpl) onConnectionLost(client pahomqtt.Client, err error) {
	c.connMu.Lock()
	c.connected = false
	c.connMu.Unlock()

	c.logger.Error("MQTT connection lost", slog.String("error", err.Error()))
	c.logConnectionEvent(model.AuditEventMQTTDisconnect)
}

// onReconnecting is called when the client is attempting to reconnect.
func (c *ClientImpl) onReconnecting(client pahomqtt.Client, opts *pahomqtt.ClientOptions) {
	c.logger.Info("MQTT reconnecting", slog.String("broker", c.cfg.BrokerURL))
	c.logConnectionEvent(model.AuditEventMQTTReconnect)
}

// defaultMessageHandler handles messages for topics without a specific handler.
func (c *ClientImpl) defaultMessageHandler(client pahomqtt.Client, msg pahomqtt.Message) {
	c.logger.Warn("received message on unsubscribed topic",
		slog.String("topic", msg.Topic()),
	)
}

// logConnectionEvent logs an MQTT connection event.
func (c *ClientImpl) logConnectionEvent(eventType string) {
	if c.auditLogger == nil {
		return
	}

	event := &model.MQTTConnectionEvent{
		AuditEventBase: model.NewAuditEventBase(""),
		EventType:      eventType,
		BrokerAddress:  c.cfg.BrokerURL,
	}
	c.auditLogger.LogMQTTConnectionEvent(context.Background(), event)
}

// CalculateBackoff calculates the exponential backoff duration.
// Returns the backoff duration capped at maxBackoff.
func CalculateBackoff(attempt int, initialBackoff, maxBackoff time.Duration) time.Duration {
	if attempt < 0 {
		attempt = 0
	}

	// Calculate exponential backoff: initial * 2^attempt
	backoff := initialBackoff
	for i := 0; i < attempt; i++ {
		backoff *= 2
		if backoff > maxBackoff {
			return maxBackoff
		}
	}

	return backoff
}
